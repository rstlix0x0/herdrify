use std::process::Stdio;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use herdrify_hc::protocol::{MethodName, RpcError};
use herdrify_hc::{Error, HerdrCore};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

type R<T> = Result<T, Box<dyn std::error::Error>>;

fn iso(cmd: &mut Command, dir: &str, session: &str) {
    for (k, _) in std::env::vars_os() {
        if k.as_encoded_bytes().starts_with(b"HC_") {
            cmd.env_remove(k);
        }
    }
    cmd.env("HC_SESSION", session)
        .env("XDG_STATE_HOME", format!("{dir}/state"))
        .env("XDG_CONFIG_HOME", format!("{dir}/config"))
        .env("XDG_RUNTIME_DIR", format!("{dir}/run"));
}

async fn call(core: &HerdrCore, label: &str, method: &str, params: Value) -> Option<Value> {
    let shown = shorten(&params);
    match core.client().call_raw(MethodName::new(method).ok()?, params).await {
        Ok(v) => {
            println!("[{label}] {method} {shown}\n    => ok {}", shorten(&v));
            Some(v)
        }
        Err(Error::Rpc { error, .. }) => {
            println!(
                "[{label}] {method} {shown}\n    => err code={} kind={} message={:?} hint={:?}",
                error.code, error.kind, error.message, error.hint
            );
            None
        }
        Err(e) => {
            println!("[{label}] {method} {shown}\n    => other error: {e}");
            None
        }
    }
}

fn shorten(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 300 { format!("{}…({} bytes)", &s[..300], s.len()) } else { s }
}

struct Line {
    child: Child,
    lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    stdin: tokio::process::ChildStdin,
}

impl Line {
    async fn open(dir: &str, session: &str) -> R<Self> {
        let mut cmd = Command::new("hc");
        cmd.arg("connect");
        iso(&mut cmd, dir, session);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or("stdin")?;
        let stdout = child.stdout.take().ok_or("stdout")?;
        let mut me = Self { child, lines: BufReader::new(stdout).lines(), stdin };
        me.send(json!({"jsonrpc":"2.0","id":0,"method":"hello","params":{"generation":1}})).await?;
        let hello = me.next(Duration::from_secs(10)).await.ok_or("no hello")?;
        let v: Value = serde_json::from_str(&hello)?;
        println!("[line] hello framed={} version={}", v["result"]["framed"], v["result"]["version"]);
        Ok(me)
    }
    async fn send(&mut self, v: Value) -> R<()> {
        self.stdin.write_all(format!("{v}\n").as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }
    async fn next(&mut self, wait: Duration) -> Option<String> {
        tokio::time::timeout(wait, self.lines.next_line()).await.ok()?.ok()?
    }
    async fn drain(&mut self, label: &str, wait: Duration) {
        while let Some(l) = self.next(wait).await {
            let v: Value = serde_json::from_str(&l).unwrap_or(Value::String(l.clone()));
            println!("    [{label}] <- {}", shorten(&v));
        }
        println!("    [{label}] (nothing more within {wait:?})");
    }
}

async fn framed_section(core: &HerdrCore, t: &str, dir: &str, session: &str) -> R<()> {
    use herdrify_hc::protocol::{Channel, Frame, FrameDecoder, FrameKind};
    use tokio::io::AsyncReadExt;

    println!("\n== G: rpc.discover on 0.1.22 against `hc schema` (evidence/hc-0.1.22-schema.json)");
    let discovered = core.client().call_raw(MethodName::new("rpc.discover")?, json!({})).await?;
    let schema_path = std::env::var("HC_SCHEMA_FILE")?;
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(&schema_path)?)?;
    println!("[G1] rpc.discover == hc schema: {}", discovered == schema);
    println!("[G1] methods: {} vs {}", discovered["methods"].as_array().map_or(0, Vec::len), schema["methods"].as_array().map_or(0, Vec::len));

    println!("\n== F: events.subscribe docs over a FRAMED connection (raw hc connect, the SDK's Frame codec)");
    for (label, params) in [
        ("F1 framed docs exact", json!({"docs":["probe-doc"]})),
        ("F2 framed docs + terminal", json!({"docs":["probe-doc"],"terminal":t})),
    ] {
        let mut cmd = Command::new("hc");
        cmd.arg("connect");
        iso(&mut cmd, dir, session);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
        let mut child = cmd.spawn()?;
        let mut stdin = child.stdin.take().ok_or("stdin")?;
        let mut out = BufReader::new(child.stdout.take().ok_or("stdout")?);
        stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"hello\",\"params\":{\"generation\":1,\"framed\":true}}\n").await?;
        let mut hello = Vec::new();
        out.read_until(b'\n', &mut hello).await?;
        let h: Value = serde_json::from_slice(&hello)?;
        println!("[{label}] hello framed={} version={}", h["result"]["framed"], h["result"]["version"]);
        let req = json!({"jsonrpc":"2.0","id":1,"method":"events.subscribe","params":params});
        let frame = Frame { channel: Channel::RPC, kind: FrameKind::Data, payload: serde_json::to_vec(&req)? };
        stdin.write_all(&frame.encode()?).await?;
        stdin.flush().await?;
        println!("[{label}] subscribe {params}");
        let mut decoder = FrameDecoder::new();
        let mut buf = vec![0u8; 65536];
        let mut stream: Option<u32> = None;
        let mut did_writes = false;
        loop {
            let read = tokio::time::timeout(Duration::from_millis(900), out.read(&mut buf)).await;
            match read {
                Ok(Ok(0)) | Ok(Err(_)) => break,
                Ok(Ok(n)) => decoder.push(&buf[..n]),
                Err(_) => {
                    if did_writes { break; }
                }
            }
            while let Some(f) = decoder.next_frame()? {
                let text = String::from_utf8_lossy(&f.payload).to_string();
                let v: Value = serde_json::from_str(text.trim()).unwrap_or(Value::String(text.clone()));
                if f.channel == Channel::RPC && v["id"] == json!(1) {
                    stream = v["result"]["channel"].as_u64().map(|c| c as u32);
                    let addrs: Vec<&str> = v["result"]["terminals"].as_array().map(|a| a.iter().filter_map(|x| x["address"].as_str()).collect()).unwrap_or_default();
                    println!("    reply on channel 0: stream channel={:?} snapshot terminals={addrs:?}", stream);
                } else {
                    println!("    <- channel {} {:?}: {}", f.channel.get(), f.kind, shorten(&v));
                }
            }
            if stream.is_some() && !did_writes {
                did_writes = true;
                let cur = core.client().call_raw(MethodName::new("doc.get")?, json!({"name":"probe-doc"})).await?;
                let ver = cur["version"].as_u64().unwrap_or(0);
                call(core, "  set probe-doc", "doc.set", json!({"name":"probe-doc","value":ver,"version":ver})).await;
                call(core, "  set other", "doc.set", json!({"name":format!("other-f{ver}"),"value":1,"version":0})).await;
                call(core, "  label t", "terminal.label", json!({"terminal":t,"labels":{"f":ver.to_string()}})).await;
            }
        }
        println!("    [{label}] (nothing more within 900ms)");
        let _ = child.kill().await;
    }
    Ok(())
}

async fn remote_section(core: &HerdrCore, pid: u32) -> R<()> {
    let b64 = |s: &[u8]| STANDARD.encode(s);
    // A second isolated core, reached as the machine "box" through a local command.
    let bdir = format!("/tmp/hfq{pid}");
    let bsession = format!("hfq{pid}");
    for sub in ["state", "config", "run"] {
        std::fs::create_dir_all(format!("{bdir}/{sub}"))?;
    }
    let via = format!(
        "env -u HC_SESSION XDG_STATE_HOME={bdir}/state XDG_CONFIG_HOME={bdir}/config XDG_RUNTIME_DIR={bdir}/run sh -c"
    );
    println!("\n== R: a put to a terminal on another machine (a second isolated core, session {bsession})");
    call(core, "R0 machine.add", "machine.add", json!({"name":"box","via":via,"session":bsession})).await;
    let run = call(core, "R1 run on box", "terminal.run", json!({"argv":["sleep","300"],"on":"box","no_parent":true})).await;
    let rt = run.as_ref().and_then(|r| r["address"].as_str()).unwrap_or("box/t1").to_owned();
    println!("remote terminal={rt}");
    call(core, "R2 terminal.put to box (control)", "terminal.put", json!({"terminal":rt,"name":"w.txt","data":b64(b"whole")})).await;
    let id = "f100000000000000";
    call(core, "R3 put_begin to box", "terminal.put_begin", json!({"terminal":rt,"name":"p.txt","bytes":5,"transfer":id})).await;
    call(core, "R3b put_chunk", "terminal.put_chunk", json!({"transfer":id,"offset":0,"data":b64(b"piece")})).await;
    call(core, "R3c put_finish", "terminal.put_finish", json!({"transfer":id})).await;
    call(core, "R3d put_abort", "terminal.put_abort", json!({"transfer":id})).await;
    call(core, "R4 teardown remote terminal", "terminal.kill", json!({"terminal":rt})).await;
    call(core, "R4 machine.rm", "machine.rm", json!({"name":"box"})).await;
    let mut stop = std::process::Command::new("hc");
    stop.args(["server", "stop"]);
    for (k, _) in std::env::vars_os() {
        if k.as_encoded_bytes().starts_with(b"HC_") {
            stop.env_remove(k);
        }
    }
    let out = stop
        .env("HC_SESSION", &bsession)
        .env("XDG_STATE_HOME", format!("{bdir}/state"))
        .env("XDG_CONFIG_HOME", format!("{bdir}/config"))
        .env("XDG_RUNTIME_DIR", format!("{bdir}/run"))
        .output()?;
    println!("R5 hc server stop (box core): {}", String::from_utf8_lossy(&out.stdout).trim());
    tokio::time::sleep(Duration::from_millis(300)).await;
    std::fs::remove_dir_all(&bdir)?;
    println!("removed {bdir}");
    Ok(())
}

async fn extra_sections(core: &HerdrCore, t: &str, dir: &str, session: &str) -> R<()> {
    let b64 = |s: &[u8]| STANDARD.encode(s);
    println!("\n== E: remote and ended terminals");
    call(core, "E1 put_begin unknown machine", "terminal.put_begin", json!({"terminal":"nowhere/t1","name":"r.txt","bytes":1,"transfer":"e100000000000000"})).await;
    call(core, "E1b terminal.put unknown machine (control)", "terminal.put", json!({"terminal":"nowhere/t1","name":"r.txt","data":b64(b"x")})).await;
    call(core, "E1c put_begin local machine name", "terminal.put_begin", json!({"terminal":format!("local/{t}"),"name":"r.txt","bytes":1,"transfer":"e1c0000000000000"})).await;
    call(core, "  cleanup", "terminal.put_abort", json!({"transfer":"e1c0000000000000"})).await;
    let fin = call(core, "E2 setup: a program that ends", "terminal.run", json!({"argv":["true"],"no_parent":true,"wait":true})).await;
    let done = fin.as_ref().and_then(|r| r["address"].as_str()).unwrap_or("t2").to_owned();
    call(core, "E2 the ended terminal", "terminal.get", json!({"terminal":done})).await;
    call(core, "E2 put_begin on an ended terminal", "terminal.put_begin", json!({"terminal":done,"name":"f.txt","bytes":1,"transfer":"e200000000000000"})).await;
    call(core, "E2b terminal.put on an ended terminal (control)", "terminal.put", json!({"terminal":done,"name":"f.txt","data":b64(b"x")})).await;
    call(core, "  cleanup", "terminal.put_abort", json!({"transfer":"e200000000000000"})).await;
    let live = call(core, "E3 setup", "terminal.run", json!({"argv":["sleep","300"],"no_parent":true})).await;
    let mid = live.as_ref().and_then(|r| r["address"].as_str()).unwrap_or("t3").to_owned();
    call(core, "E3 begin 4 bytes", "terminal.put_begin", json!({"terminal":mid,"name":"m.txt","bytes":4,"transfer":"e300000000000000"})).await;
    call(core, "E3 piece 1", "terminal.put_chunk", json!({"transfer":"e300000000000000","offset":0,"data":b64(b"ab")})).await;
    call(core, "E3 kill the terminal mid-put", "terminal.kill", json!({"terminal":mid})).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    call(core, "E3 piece 2 after the kill", "terminal.put_chunk", json!({"transfer":"e300000000000000","offset":2,"data":b64(b"cd")})).await;
    call(core, "E3 finish after the kill", "terminal.put_finish", json!({"transfer":"e300000000000000"})).await;
    call(core, "E3 abort after the kill", "terminal.put_abort", json!({"transfer":"e300000000000000"})).await;
    let gone = call(core, "E4 setup", "terminal.run", json!({"argv":["true"],"no_parent":true,"wait":true})).await;
    let rmd = gone.as_ref().and_then(|r| r["address"].as_str()).unwrap_or("t4").to_owned();
    call(core, "E4 begin on a finished terminal, then rm it", "terminal.put_begin", json!({"terminal":rmd,"name":"g.txt","bytes":1,"transfer":"e400000000000000"})).await;
    call(core, "E4 rm", "terminal.rm", json!({"terminal":rmd})).await;
    call(core, "E4 piece after rm", "terminal.put_chunk", json!({"transfer":"e400000000000000","offset":0,"data":b64(b"z")})).await;

    println!("\n== C9/C10: docs with labels or children_of (line connection)");
    call(core, "C9 setup label", "terminal.label", json!({"terminal":t,"labels":{"team":"a"}})).await;
    let kid = call(core, "C10 setup child", "terminal.run", json!({"argv":["sleep","300"],"parent":t})).await;
    let child = kid.as_ref().and_then(|r| r["address"].as_str()).unwrap_or("t9").to_owned();
    let mut v = 100u64;
    for (label, params) in [
        ("C9 docs + labels", json!({"docs":["probe-doc"],"labels":{"team":"a"}})),
        ("C10 docs + children_of", json!({"docs":["probe-doc"],"children_of":t})),
    ] {
        let mut l = Line::open(dir, session).await?;
        l.send(json!({"jsonrpc":"2.0","id":1,"method":"events.subscribe","params":params})).await?;
        println!("[{label}] subscribe {params}");
        if let Some(reply) = l.next(Duration::from_secs(5)).await {
            let r: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
            let addrs: Vec<&str> = r["result"]["terminals"].as_array().map(|a| a.iter().filter_map(|x| x["address"].as_str()).collect()).unwrap_or_default();
            println!("    reply snapshot terminals = {addrs:?}");
        }
        let set = call(core, "  set probe-doc", "doc.set", json!({"name":"probe-doc","value":v,"version":0})).await;
        if set.is_none() {
            let cur = call(core, "  get probe-doc", "doc.get", json!({"name":"probe-doc"})).await;
            let ver = cur.as_ref().and_then(|c| c["version"].as_u64()).unwrap_or(0);
            call(core, "  set probe-doc", "doc.set", json!({"name":"probe-doc","value":v,"version":ver})).await;
        }
        v += 1;
        call(core, "  set other", "doc.set", json!({"name":format!("other-{v}"),"value":1,"version":0})).await;
        call(core, "  label the labelled terminal", "terminal.label", json!({"terminal":t,"labels":{"n":v.to_string()}})).await;
        call(core, "  label the child", "terminal.label", json!({"terminal":child,"labels":{"n":v.to_string()}})).await;
        l.drain(label, Duration::from_millis(700)).await;
        let _ = l.child.kill().await;
    }
    call(core, "  cleanup child", "terminal.kill", json!({"terminal":child})).await;
    call(core, "  cleanup E3", "terminal.rm", json!({"terminal":mid})).await;
    Ok(())
}

#[tokio::main]
async fn main() -> R<()> {
    let pid = std::process::id();
    let session = format!("hfp{pid}");
    let dir = format!("/tmp/hfp{pid}");
    for sub in ["state", "config", "run"] {
        std::fs::create_dir_all(format!("{dir}/{sub}"))?;
    }
    let mut b = HerdrCore::builder().session(session.clone());
    for (k, _) in std::env::vars_os() {
        if k.as_encoded_bytes().starts_with(b"HC_") {
            b = b.env_remove(k);
        }
    }
    let core = b
        .env("XDG_STATE_HOME", format!("{dir}/state"))
        .env("XDG_CONFIG_HOME", format!("{dir}/config"))
        .env("XDG_RUNTIME_DIR", format!("{dir}/run"))
        .connect()
        .await?;
    let h = core.hello();
    println!("== hello\nversion={} framed={} generation={:?}", h.version, h.framed, h.generation);
    println!("features={:?}", h.features);
    println!("methods({})={:?}", h.methods.len(), h.methods);

    println!("\n== error decode (SDK RpcError) of the 0.1.22 doc.set internal entry");
    let e: RpcError = serde_json::from_str(r#"{"code":-32603,"data":{"kind":"internal"},"message":"internal"}"#)?;
    println!("code={} kind={:?} display={e}", e.code, e.kind);

    let run = call(&core, "setup", "terminal.run", json!({"argv":["sleep","300"],"no_parent":true})).await;
    let t = run.as_ref().and_then(|r| r["address"].as_str()).unwrap_or("t1").to_owned();
    println!("terminal={t}");
    let b64 = |s: &[u8]| STANDARD.encode(s);

    let extra = std::env::args().any(|a| a == "extra");
    let remote = std::env::args().any(|a| a == "remote");
    let framed = std::env::args().any(|a| a == "framed");
    if framed {
        framed_section(&core, &t, &dir, &session).await?;
    } else if remote {
        remote_section(&core, pid).await?;
    } else if extra {
        extra_sections(&core, &t, &dir, &session).await?;
    } else {
    println!("\n== happy path");
    let x = "0123456789abcdef";
    call(&core, "A1", "terminal.put_begin", json!({"terminal":t,"name":"a.txt","bytes":10,"transfer":x})).await;
    call(&core, "A2", "terminal.put_chunk", json!({"transfer":x,"offset":0,"data":b64(b"hello")})).await;
    call(&core, "A3 wrong offset (0 again)", "terminal.put_chunk", json!({"transfer":x,"offset":0,"data":b64(b"hello")})).await;
    call(&core, "A3b offset ahead (7)", "terminal.put_chunk", json!({"transfer":x,"offset":7,"data":b64(b"x")})).await;
    call(&core, "A4", "terminal.put_chunk", json!({"transfer":x,"offset":5,"data":b64(b"world")})).await;
    call(&core, "A5 past size", "terminal.put_chunk", json!({"transfer":x,"offset":10,"data":b64(b"!")})).await;
    let fin = call(&core, "A6", "terminal.put_finish", json!({"transfer":x})).await;
    if let Some(p) = fin.as_ref().and_then(|f| f["path"].as_str()) {
        call(&core, "A6 read back", "file.read", json!({"path":p})).await;
    }
    call(&core, "A7 chunk after finish", "terminal.put_chunk", json!({"transfer":x,"offset":10,"data":b64(b"!")})).await;
    call(&core, "A8 finish again", "terminal.put_finish", json!({"transfer":x})).await;
    call(&core, "A9 abort after finish", "terminal.put_abort", json!({"transfer":x})).await;
    call(&core, "A10 begin reusing a finished transfer id", "terminal.put_begin", json!({"terminal":t,"name":"a2.txt","bytes":1,"transfer":x})).await;

    println!("\n== transfer id and size rules");
    for (label, id) in [
        ("B1 15 hex", "0123456789abcde"),
        ("B1b 16 hex", "1123456789abcdef"),
        ("B1c 64 hex", "a".repeat(64).as_str()),
        ("B1d 65 hex", "b".repeat(65).as_str()),
        ("B2 non-hex 16", "zzzzzzzzzzzzzzzz"),
        ("B3 uppercase hex 16", "ABCDEF0123456789"),
        ("B3b empty", ""),
    ] {
        call(&core, label, "terminal.put_begin", json!({"terminal":t,"name":"b.txt","bytes":1,"transfer":id})).await;
        call(&core, "  cleanup", "terminal.put_abort", json!({"transfer":id})).await;
    }
    for (label, bytes) in [("B4 8 MiB", 8u64 * 1024 * 1024), ("B4b 8 MiB + 1", 8 * 1024 * 1024 + 1), ("B4c 8_000_001", 8_000_001)] {
        let id = format!("{:016x}", bytes);
        call(&core, label, "terminal.put_begin", json!({"terminal":t,"name":"big.bin","bytes":bytes,"transfer":id})).await;
        call(&core, "  cleanup", "terminal.put_abort", json!({"transfer":id})).await;
    }

    println!("\n== zero bytes, duplicates, unknowns");
    let z = "2000000000000000";
    call(&core, "B5 begin 0 bytes", "terminal.put_begin", json!({"terminal":t,"name":"empty.txt","bytes":0,"transfer":z})).await;
    call(&core, "B5b finish 0 bytes", "terminal.put_finish", json!({"transfer":z})).await;
    let d = "3000000000000000";
    call(&core, "B6 begin", "terminal.put_begin", json!({"terminal":t,"name":"d.txt","bytes":3,"transfer":d})).await;
    call(&core, "B6b same id while open", "terminal.put_begin", json!({"terminal":t,"name":"d.txt","bytes":3,"transfer":d})).await;
    call(&core, "B9 finish before all bytes", "terminal.put_finish", json!({"transfer":d})).await;
    call(&core, "B11 abort open", "terminal.put_abort", json!({"transfer":d})).await;
    call(&core, "B11b chunk after abort", "terminal.put_chunk", json!({"transfer":d,"offset":0,"data":b64(b"abc")})).await;
    call(&core, "B11c abort unknown", "terminal.put_abort", json!({"transfer":"4000000000000000"})).await;
    call(&core, "B15 chunk unknown transfer", "terminal.put_chunk", json!({"transfer":"5000000000000000","offset":0,"data":b64(b"a")})).await;
    call(&core, "B15b finish unknown transfer", "terminal.put_finish", json!({"transfer":"5000000000000000"})).await;
    call(&core, "B7 unknown terminal", "terminal.put_begin", json!({"terminal":"t999","name":"x.txt","bytes":1,"transfer":"6000000000000000"})).await;
    for (label, name) in [("B8 empty name", ""), ("B8b slash name", "a/b.txt"), ("B8c dotdot", ".."), ("B8d name taken", "a.txt")] {
        let id = format!("7{:015x}", name.len() + label.len());
        call(&core, label, "terminal.put_begin", json!({"terminal":t,"name":name,"bytes":1,"transfer":id})).await;
        call(&core, "  chunk", "terminal.put_chunk", json!({"transfer":id,"offset":0,"data":b64(b"z")})).await;
        call(&core, "  finish", "terminal.put_finish", json!({"transfer":id})).await;
    }
    let c = "8000000000000000";
    call(&core, "B10 begin 2 MiB", "terminal.put_begin", json!({"terminal":t,"name":"c.bin","bytes":2*1024*1024,"transfer":c})).await;
    call(&core, "B10 chunk 1 MiB + 1", "terminal.put_chunk", json!({"transfer":c,"offset":0,"data":b64(&vec![0u8; 1024*1024+1])})).await;
    call(&core, "B10b chunk 1 MiB", "terminal.put_chunk", json!({"transfer":c,"offset":0,"data":b64(&vec![0u8; 1024*1024])})).await;
    call(&core, "B12 bad base64", "terminal.put_chunk", json!({"transfer":c,"offset":1048576,"data":"!!notbase64"})).await;
    call(&core, "B13 empty data", "terminal.put_chunk", json!({"transfer":c,"offset":1048576,"data":""})).await;
    call(&core, "  cleanup", "terminal.put_abort", json!({"transfer":c})).await;

    println!("\n== D: a whole 8 MiB put in 1 MiB pieces, checked by size and content");
    let full = "9100000000000000";
    let total: usize = 8 * 1024 * 1024;
    let body: Vec<u8> = (0..total).map(|i| b'a' + (i % 26) as u8).collect();
    call(&core, "D1 begin 8 MiB", "terminal.put_begin", json!({"terminal":t,"name":"full.txt","bytes":total,"transfer":full})).await;
    let mut ok = 0;
    for (i, piece) in body.chunks(1024 * 1024).enumerate() {
        let r = core.client().call_raw(MethodName::new("terminal.put_chunk")?, json!({"transfer":full,"offset":i*1024*1024,"data":b64(piece)})).await;
        match r { Ok(v) => { ok += 1; println!("    [D2 piece {i}] => ok {v}"); } Err(e) => println!("    [D2 piece {i}] => {e}") }
    }
    println!("    [D2] {ok} of 8 pieces accepted");
    let fin = call(&core, "D3 finish", "terminal.put_finish", json!({"transfer":full})).await;
    if let Some(p) = fin.as_ref().and_then(|f| f["path"].as_str()) {
        for (label, argv) in [("D4 size", json!(["wc","-c",p])), ("D4 checksum", json!(["shasum","-a","256",p]))] {
            if let Some(v) = call(&core, label, "machine.exec", json!({"argv":argv})).await {
                let out = v["stdout"].as_str().and_then(|s| STANDARD.decode(s).ok()).unwrap_or_default();
                println!("    [{label}] stdout decoded: {}", String::from_utf8_lossy(&out).trim());
            }
        }
        use std::fmt::Write as _;
        let mut local = String::new();
        let digest = std::process::Command::new("shasum").args(["-a", "256"]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().and_then(|mut c| {
            use std::io::Write as _;
            c.stdin.take().map(|mut s| s.write_all(&body)).transpose()?;
            c.wait_with_output()
        });
        if let Ok(out) = digest { let _ = write!(local, "{}", String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap_or("")); }
        println!("    [D4] local sha256 of the sent bytes: {local}");
    }

    println!("\n== put_begin over a line (non-framed) connection");
    let mut line = Line::open(&dir, &session).await?;
    line.send(json!({"jsonrpc":"2.0","id":1,"method":"terminal.put_begin","params":{"terminal":t,"name":"l.txt","bytes":1,"transfer":"9000000000000000"}})).await?;
    line.drain("line", Duration::from_millis(800)).await;
    let _ = line.child.kill().await;

    println!("\n== events.subscribe docs");
    let subs: Vec<(&str, Value)> = vec![
        ("C1 docs exact", json!({"docs":["probe-doc"]})),
        ("C2 docs prefix", json!({"docs":["pre*"]})),
        ("C3 docs empty array", json!({"docs":[]})),
        ("C4 no filter", json!({})),
        ("C5 docs + terminal", json!({"docs":["probe-doc"],"terminal":t})),
        ("C6 docs star", json!({"docs":["*"]})),
        ("C7 docs empty string", json!({"docs":[""]})),
        ("C8 docs duplicate", json!({"docs":["probe-doc","probe-doc"]})),
    ];
    let mut version = 0u64;
    let mut pre = 0u64;
    let mut other = 0u64;
    for (label, params) in subs {
        let mut l = Line::open(&dir, &session).await?;
        l.send(json!({"jsonrpc":"2.0","id":1,"method":"events.subscribe","params":params})).await?;
        println!("[{label}] subscribe {params}");
        if let Some(reply) = l.next(Duration::from_secs(5)).await {
            println!("    reply <- {}", shorten(&serde_json::from_str::<Value>(&reply).unwrap_or(Value::Null)));
        }
        if let Some(v) = call(&core, "  set", "doc.set", json!({"name":"probe-doc","value":{"n":version},"version":version})).await {
            version = v["version"].as_u64().unwrap_or(version);
        }
        if let Some(v) = call(&core, "  set", "doc.set", json!({"name":"pre-x","value":1,"version":pre})).await {
            pre = v["version"].as_u64().unwrap_or(pre);
        }
        if let Some(v) = call(&core, "  set", "doc.set", json!({"name":"other","value":1,"version":other})).await {
            other = v["version"].as_u64().unwrap_or(other);
        }
        call(&core, "  label", "terminal.label", json!({"terminal":t,"labels":{"k":label.len().to_string()}})).await;
        l.drain(label, Duration::from_millis(700)).await;
        let _ = l.child.kill().await;
    }

    }

    println!("\n== teardown");
    call(&core, "teardown", "terminal.kill", json!({"terminal":t})).await;
    call(&core, "teardown", "server.stop", json!({})).await;
    drop(core);
    tokio::time::sleep(Duration::from_millis(300)).await;
    std::fs::remove_dir_all(&dir)?;
    println!("removed {dir}");
    Ok(())
}
