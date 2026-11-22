//! Throwaway probe: herdr-core (`hc` 0.1.21) method behaviour, FRAMED mode over `hc connect`.
//! Every hc child runs in an isolated env (HC_SESSION=hfy-more, scratch XDG dirs from HFY_D).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;

const HC: &str = "/Users/hiraq/.local/bin/hc";
const DATA: u8 = 0;
const CREDIT: u8 = 1;
const CLOSE: u8 = 2;
const RESIZE: u8 = 3;

static START: OnceLock<Instant> = OnceLock::new();

fn ms() -> u128 {
    START.get_or_init(Instant::now).elapsed().as_millis()
}

fn kind_name(k: u8) -> &'static str {
    match k {
        0 => "DATA",
        1 => "CREDIT",
        2 => "CLOSE",
        3 => "RESIZE",
        4 => "FOCUS",
        5 => "FACTS",
        6 => "AUTOMATIC_INPUT",
        _ => "UNKNOWN",
    }
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_payload(kind: u8, p: &[u8]) -> String {
    if (kind == CREDIT) && p.len() == 4 {
        return format!(
            "u32_le={} (hex {})",
            u32::from_le_bytes([p[0], p[1], p[2], p[3]]),
            hex(p)
        );
    }
    if kind == RESIZE && p.len() == 4 {
        return format!(
            "cols={} rows={} (hex {})",
            u16::from_le_bytes([p[0], p[1]]),
            u16::from_le_bytes([p[2], p[3]]),
            hex(p)
        );
    }
    if p.is_empty() {
        return "<empty>".into();
    }
    match std::str::from_utf8(p) {
        Ok(s) => {
            let limit = 8000;
            let ends_nl = s.ends_with('\n');
            if s.chars().count() > limit {
                let head: String = s.chars().take(limit).collect();
                format!(
                    "utf8(ends_with_newline={ends_nl}) {head:?} ...(truncated; {} bytes total)",
                    p.len()
                )
            } else {
                format!("utf8(ends_with_newline={ends_nl}) {s:?}")
            }
        }
        Err(_) => format!("non-utf8 first64hex=[{}]", hex(&p[..p.len().min(64)])),
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum Ev {
    Frame { ch: u32, kind: u8, payload: Vec<u8> },
    Line(Vec<u8>),
    Eof(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Plain JSON lines for the whole connection.
    Plain,
    /// The first reply may be a plain line (hello); everything after is frames.
    FramedAfterHello,
}

struct Conn {
    name: &'static str,
    stdin: ChildStdin,
    rx: mpsc::UnboundedReceiver<Ev>,
    child: Child,
    sent: usize,
}

impl Conn {
    fn spawn(name: &'static str, mode: Mode) -> Self {
        let d = std::env::var("HFY_D").expect("HFY_D must be set to the scratch dir");
        let mut cmd = Command::new(HC);
        cmd.arg("connect");
        // Remove EVERY inherited HC_* variable (they point at the user's default core).
        for (k, _) in std::env::vars_os() {
            if k.to_string_lossy().starts_with("HC_") {
                cmd.env_remove(&k);
            }
        }
        let mut child = cmd
            .env("HC_SESSION", "hfy-more")
            .env("XDG_STATE_HOME", format!("{d}/state"))
            .env("XDG_CONFIG_HOME", format!("{d}/config"))
            .env("XDG_RUNTIME_DIR", format!("{d}/run"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn hc connect");
        println!(
            "[{name}] +{}ms spawned `hc connect` pid={:?} (HC_SESSION=hfy-more, XDG_*={d}/...)",
            ms(),
            child.id()
        );
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let stdin = child.stdin.take().unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                println!("[{name}] +{}ms STDERR {l:?}", ms());
            }
        });
        tokio::spawn(reader(name, mode, stdout, tx));
        Self {
            name,
            stdin,
            rx,
            child,
            sent: 0,
        }
    }

    async fn send_line(&mut self, s: &str) {
        let line = format!("{s}\n");
        println!(
            "[{}] SEND +{}ms PLAIN-LINE len={} {:?}",
            self.name,
            ms(),
            line.len(),
            line
        );
        if let Err(e) = self.stdin.write_all(line.as_bytes()).await {
            println!("[{}] SEND error: {e}", self.name);
        }
        let _ = self.stdin.flush().await;
    }

    async fn send_frame(&mut self, ch: u32, kind: u8, payload: &[u8]) {
        let mut buf = Vec::with_capacity(9 + payload.len());
        buf.extend_from_slice(&ch.to_le_bytes());
        buf.push(kind);
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        let hdr = hex(&buf);
        buf.extend_from_slice(payload);
        self.sent += 1;
        let hdr_s = if self.sent <= 4 {
            format!(" hdr_hex=[{hdr}]")
        } else {
            String::new()
        };
        println!(
            "[{}] SEND +{}ms ch={ch} kind={kind}/{} len={}{hdr_s} payload={}",
            self.name,
            ms(),
            kind_name(kind),
            payload.len(),
            fmt_payload(kind, payload)
        );
        if let Err(e) = self.stdin.write_all(&buf).await {
            println!("[{}] SEND error: {e}", self.name);
        }
        let _ = self.stdin.flush().await;
    }

    async fn rpc(&mut self, id: u64, method: &str, params: Value) {
        let msg = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string();
        self.send_frame(0, DATA, msg.as_bytes()).await;
    }

    async fn recv(&mut self, t: Duration) -> Option<Ev> {
        tokio::time::timeout(t, self.rx.recv()).await.ok().flatten()
    }

    /// Collect everything for `t`; stop early on EOF.
    async fn drain(&mut self, t: Duration) -> Vec<Ev> {
        let deadline = Instant::now() + t;
        let mut out = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            match self.recv(left).await {
                Some(e @ Ev::Eof(_)) => {
                    out.push(e);
                    break;
                }
                Some(e) => out.push(e),
                None => break,
            }
        }
        out
    }

    /// Wait for the channel-0 reply with this id; other frames are logged by the reader and kept.
    async fn reply(&mut self, id: u64, t: Duration, others: &mut Vec<Ev>) -> Option<Value> {
        let deadline = Instant::now() + t;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let ev = self.recv(left).await?;
            if let Some(v) = as_rpc(&ev)
                && v.get("id") == Some(&json!(id))
            {
                return Some(v);
            }
            if matches!(ev, Ev::Eof(_)) {
                others.push(ev);
                return None;
            }
            others.push(ev);
        }
    }
}

fn as_rpc(ev: &Ev) -> Option<Value> {
    match ev {
        Ev::Frame {
            ch: 0,
            kind: DATA,
            payload,
        } => serde_json::from_slice(payload).ok(),
        Ev::Line(l) => serde_json::from_slice(l).ok(),
        _ => None,
    }
}

async fn reader(
    name: &'static str,
    mode: Mode,
    stdout: tokio::process::ChildStdout,
    tx: mpsc::UnboundedSender<Ev>,
) {
    static RECV_HDRS: AtomicUsize = AtomicUsize::new(0);
    let mut r = BufReader::new(stdout);
    if mode == Mode::Plain {
        loop {
            let mut line = Vec::new();
            match r.read_until(b'\n', &mut line).await {
                Ok(0) | Err(_) => {
                    println!("[{name}] RECV +{}ms EOF", ms());
                    let _ = tx.send(Ev::Eof("eof".into()));
                    return;
                }
                Ok(n) => {
                    println!(
                        "[{name}] RECV +{}ms PLAIN-LINE len={n} {:?}",
                        ms(),
                        String::from_utf8_lossy(&line)
                    );
                    let _ = tx.send(Ev::Line(line));
                }
            }
        }
    }
    // FramedAfterHello: inspect the first reply bytes.
    let first = match r.fill_buf().await {
        Ok(b) if !b.is_empty() => b[0],
        _ => {
            println!("[{name}] RECV +{}ms EOF before any byte", ms());
            let _ = tx.send(Ev::Eof("eof".into()));
            return;
        }
    };
    if first == b'{' {
        let mut line = Vec::new();
        let n = r.read_until(b'\n', &mut line).await.unwrap_or(0);
        println!(
            "[{name}] RECV +{}ms HELLO-REPLY is a PLAIN LINE: len={n} first16hex=[{}] last2hex=[{}] text={:?}",
            ms(),
            hex(&line[..line.len().min(16)]),
            hex(&line[line.len().saturating_sub(2)..]),
            String::from_utf8_lossy(&line)
        );
        let _ = tx.send(Ev::Line(line));
    } else {
        println!(
            "[{name}] RECV +{}ms first byte is 0x{first:02x}, not '{{': parsing frames",
            ms()
        );
    }
    let mut nframes = 0usize;
    loop {
        let mut h = [0u8; 9];
        if let Err(e) = r.read_exact(&mut h).await {
            println!("[{name}] RECV +{}ms EOF/err reading header: {e}", ms());
            let _ = tx.send(Ev::Eof(e.to_string()));
            return;
        }
        let ch = u32::from_le_bytes([h[0], h[1], h[2], h[3]]);
        let kind = h[4];
        let len = u32::from_le_bytes([h[5], h[6], h[7], h[8]]) as usize;
        if kind > 6 || len > 16 * 1024 * 1024 {
            let mut rest = vec![0u8; 4096];
            let n = tokio::time::timeout(Duration::from_millis(500), r.read(&mut rest))
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or(0);
            println!(
                "[{name}] RECV +{}ms IMPLAUSIBLE HEADER hex=[{}] (ch={ch} kind={kind} len={len}); following raw bytes ({n}): {:?}",
                ms(),
                hex(&h),
                String::from_utf8_lossy(&[&h[..], &rest[..n]].concat())
            );
            let _ = tx.send(Ev::Eof("implausible header".into()));
            return;
        }
        let mut payload = vec![0u8; len];
        if let Err(e) = r.read_exact(&mut payload).await {
            println!("[{name}] RECV +{}ms EOF/err reading payload: {e}", ms());
            let _ = tx.send(Ev::Eof(e.to_string()));
            return;
        }
        nframes += 1;
        let total = RECV_HDRS.fetch_add(1, Ordering::Relaxed);
        let hdr_s = if nframes <= 4 || total < 6 {
            format!(" hdr_hex=[{}]", hex(&h))
        } else {
            String::new()
        };
        println!(
            "[{name}] RECV +{}ms ch={ch} kind={kind}/{} len={len}{hdr_s} payload={}",
            ms(),
            kind_name(kind),
            fmt_payload(kind, &payload)
        );
        let _ = tx.send(Ev::Frame { ch, kind, payload });
    }
}

const T: Duration = Duration::from_secs(5);

async fn framed(name: &'static str, hello: Value) -> (Conn, Value) {
    let mut c = Conn::spawn(name, Mode::FramedAfterHello);
    let msg = json!({"jsonrpc":"2.0","id":1,"method":"hello","params":hello}).to_string();
    c.send_line(&msg).await;
    let mut others = Vec::new();
    let v = c.reply(1, T, &mut others).await.unwrap_or(Value::Null);
    (c, v)
}

fn hello_framed() -> Value {
    json!({"generation":1,"framed":true})
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(100);

/// Send one call on channel 0 and wait for its reply (other frames are printed by the reader).
async fn call(c: &mut Conn, method: &str, params: Value, t: Duration) -> Option<Value> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed) as u64;
    let t0 = Instant::now();
    c.rpc(id, method, params).await;
    let mut o = Vec::new();
    let r = c.reply(id, t, &mut o).await;
    match &r {
        Some(v) => println!(
            "ANALYSIS[{method} id={id}] elapsed_ms={} outcome={}",
            t0.elapsed().as_millis(),
            if v.get("error").is_some() {
                format!("ERROR {}", v["error"])
            } else {
                "RESULT".to_string()
            }
        ),
        None => println!(
            "ANALYSIS[{method} id={id}] NO REPLY within {} ms (elapsed {} ms)",
            t.as_millis(),
            t0.elapsed().as_millis()
        ),
    }
    r
}

fn b64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    out
}

fn step(s: &str) {
    println!("== step: {s}");
}

const LONG: Duration = Duration::from_secs(25);

async fn m1() {
    let (mut c, _) = framed("m1", hello_framed()).await;
    step("terminal.run argv=[sh]");
    let r = call(&mut c, "terminal.run", json!({"argv":["sh"]}), T).await;
    let t = result_of(&r)["address"].as_str().unwrap_or("t1").to_string();
    println!("ANALYSIS terminal under test = {t}");

    step("terminal.keep keep:true, then keep:false");
    call(&mut c, "terminal.keep", json!({"terminal":t,"keep":true}), T).await;
    call(&mut c, "terminal.keep", json!({"terminal":t,"keep":false}), T).await;

    step("terminal.label set a=1 and b=2; terminal.get shows labels");
    call(&mut c, "terminal.label", json!({"terminal":t,"labels":{"a":"1","b":"2"}}), T).await;
    let g = call(&mut c, "terminal.get", json!({"terminal":t}), T).await;
    println!("ANALYSIS labels after set = {}", result_of(&g)["labels"]);
    step("terminal.list labels filter {a:1} and {b:2} (before removal)");
    call(&mut c, "terminal.list", json!({"labels":{"a":"1"}}), T).await;
    step("terminal.label remove a with empty value (schema: `key=` removes one)");
    call(&mut c, "terminal.label", json!({"terminal":t,"labels":{"a":""}}), T).await;
    let g = call(&mut c, "terminal.get", json!({"terminal":t}), T).await;
    println!("ANALYSIS labels after a=\"\" = {}", result_of(&g)["labels"]);
    step("terminal.label with JSON null value (wrong type probe)");
    call(&mut c, "terminal.label", json!({"terminal":t,"labels":{"b":null}}), T).await;
    step("terminal.list labels filter {a:1} (after removal) and {b:2}");
    call(&mut c, "terminal.list", json!({"labels":{"a":"1"}}), T).await;
    call(&mut c, "terminal.list", json!({"labels":{"b":"2"}}), T).await;

    step("terminal.send text=\"echo sent-ok\" (no wait)");
    call(&mut c, "terminal.send", json!({"terminal":t,"text":"echo sent-ok"}), LONG).await;
    step("terminal.send text=\"echo sent-wait\" wait:true timeout:3");
    call(
        &mut c,
        "terminal.send",
        json!({"terminal":t,"text":"echo sent-wait","wait":true,"timeout":3}),
        LONG,
    )
    .await;
    step("terminal.read to confirm what sh received");
    let rd = call(&mut c, "terminal.read", json!({"terminal":t}), T).await;
    let text = result_of(&rd).to_string();
    println!(
        "ANALYSIS screen contains sent-ok={} sent-wait={}",
        text.contains("sent-ok"),
        text.contains("sent-wait")
    );

    step("terminal.keys [enter]");
    call(&mut c, "terminal.keys", json!({"terminal":t,"keys":["enter"]}), LONG).await;
    step("terminal.keys invalid key name [no-such-key-hfy]");
    call(&mut c, "terminal.keys", json!({"terminal":t,"keys":["no-such-key-hfy"]}), LONG).await;
    step("terminal.keys empty list []");
    call(&mut c, "terminal.keys", json!({"terminal":t,"keys":[]}), LONG).await;

    step("terminal.interrupt (default timeout), sh idle at prompt");
    call(&mut c, "terminal.interrupt", json!({"terminal":t}), LONG).await;

    step("terminal.put name=x.txt data=base64(\"hello\")");
    let p = call(
        &mut c,
        "terminal.put",
        json!({"terminal":t,"name":"x.txt","data":b64(b"hello")}),
        T,
    )
    .await;
    if let Some(path) = result_of(&p)["path"].as_str() {
        step("file.read the put path to verify bytes");
        call(&mut c, "file.read", json!({"path":path}), T).await;
    }
    step("terminal.put same name again (suffix when taken?)");
    call(
        &mut c,
        "terminal.put",
        json!({"terminal":t,"name":"x.txt","data":b64(b"hello")}),
        T,
    )
    .await;
    step("terminal.put with non-base64 data");
    call(
        &mut c,
        "terminal.put",
        json!({"terminal":t,"name":"y.txt","data":"not base64!!"}),
        T,
    )
    .await;

    step("terminal.list all:true (before kill)");
    call(&mut c, "terminal.list", json!({"all":true}), T).await;
    step("terminal.kill");
    call(&mut c, "terminal.kill", json!({"terminal":t}), LONG).await;
    step("terminal.kill again on the finished terminal");
    call(&mut c, "terminal.kill", json!({"terminal":t}), LONG).await;
    step("terminal.list {} vs {all:true} after kill");
    call(&mut c, "terminal.list", json!({}), T).await;
    call(&mut c, "terminal.list", json!({"all":true}), T).await;

    step("terminal.run argv=[no-such-prog-hfy]");
    let r = call(&mut c, "terminal.run", json!({"argv":["no-such-prog-hfy"]}), LONG).await;
    if let Some(a) = result_of(&r)["address"].as_str() {
        tokio::time::sleep(Duration::from_millis(500)).await;
        step("terminal.get on it after 500 ms");
        call(&mut c, "terminal.get", json!({"terminal":a}), T).await;
        step("terminal.read on it");
        call(&mut c, "terminal.read", json!({"terminal":a}), T).await;
    }
    step("terminal.run argv=[no-such-prog-hfy] wait:true timeout:5");
    call(
        &mut c,
        "terminal.run",
        json!({"argv":["no-such-prog-hfy"],"wait":true,"timeout":5}),
        LONG,
    )
    .await;
}

async fn m2() {
    let d = std::env::var("HFY_D").unwrap();
    let dir = format!("{d}/files");
    let a = format!("{dir}/sub/a.txt");
    let (mut c, _) = framed("m2", hello_framed()).await;
    step("file.write path (missing folder) data=base64(\"line1\\nline2\\nline3\\n\")");
    call(&mut c, "file.write", json!({"path":a,"data":b64(b"line1\nline2\nline3\n")}), T).await;
    step("file.read whole");
    call(&mut c, "file.read", json!({"path":a}), T).await;
    step("file.write with plain text data (not base64): \"hello world!\"");
    let pt = format!("{dir}/plain.txt");
    call(&mut c, "file.write", json!({"path":pt,"data":"hello world!"}), T).await;
    step("file.write with plain text that happens to be valid base64: \"abcd\"");
    let pt2 = format!("{dir}/plain2.txt");
    call(&mut c, "file.write", json!({"path":pt2,"data":"abcd"}), T).await;
    call(&mut c, "file.read", json!({"path":pt2}), T).await;
    step("file.edit success: line2 -> LINE-TWO");
    call(&mut c, "file.edit", json!({"path":a,"edits":[{"old":"line2","new":"LINE-TWO"}]}), T).await;
    call(&mut c, "file.read", json!({"path":a}), T).await;
    step("file.edit old absent");
    call(&mut c, "file.edit", json!({"path":a,"edits":[{"old":"no-such-text","new":"x"}]}), T).await;
    step("file.edit old found more than once (\"line\")");
    call(&mut c, "file.edit", json!({"path":a,"edits":[{"old":"line","new":"x"}]}), T).await;
    step("file.edit two edits, second absent (all or none?)");
    call(
        &mut c,
        "file.edit",
        json!({"path":a,"edits":[{"old":"line1","new":"ONE"},{"old":"zzz","new":"x"}]}),
        T,
    )
    .await;
    call(&mut c, "file.read", json!({"path":a}), T).await;
    step("file.edit on missing file");
    call(
        &mut c,
        "file.edit",
        json!({"path":format!("{dir}/missing.txt"),"edits":[{"old":"a","new":"b"}]}),
        T,
    )
    .await;
    step("file.read offset:2 limit:1");
    call(&mut c, "file.read", json!({"path":a,"offset":2,"limit":1}), T).await;
    step("file.read offset:0");
    call(&mut c, "file.read", json!({"path":a,"offset":0}), T).await;
    step("file.read offset:99 (past end)");
    call(&mut c, "file.read", json!({"path":a,"offset":99}), T).await;
    step("file.write binary bytes [00 ff fe 01 80], then file.read it");
    let bin = format!("{dir}/bin.dat");
    call(&mut c, "file.write", json!({"path":bin,"data":b64(&[0, 0xff, 0xfe, 1, 0x80])}), T).await;
    call(&mut c, "file.read", json!({"path":bin}), T).await;
    step("file.read missing path");
    call(&mut c, "file.read", json!({"path":format!("{dir}/missing.txt")}), T).await;
    step("file.read on a directory");
    call(&mut c, "file.read", json!({"path":dir}), T).await;
    step("file.read relative path with cwd");
    call(&mut c, "file.read", json!({"path":"sub/a.txt","cwd":dir}), T).await;
}

async fn m3() {
    let (mut c, _) = framed("m3", hello_framed()).await;
    step("machine.list");
    call(&mut c, "machine.list", json!({}), T).await;
    step("machine.connect / rename / rm on unknown machine `nope`");
    call(&mut c, "machine.connect", json!({"machine":"nope"}), LONG).await;
    call(&mut c, "machine.rename", json!({"machine":"nope","new_name":"x"}), T).await;
    call(&mut c, "machine.rm", json!({"machine":"nope"}), T).await;
    step("machine.add {name:m1, via:\"false\"} (exits 1 at once; dials nothing)");
    call(&mut c, "machine.add", json!({"name":"m1","via":"false"}), LONG).await;
    step("machine.add {via:\"false\"} without name (schema: via needs --name)");
    call(&mut c, "machine.add", json!({"via":"false"}), LONG).await;
    step("machine.add {target:\"hfy-invalid.invalid\"} (.invalid TLD never resolves)");
    call(&mut c, "machine.add", json!({"target":"hfy-invalid.invalid"}), LONG).await;
    step("machine.add {} (neither target nor via)");
    call(&mut c, "machine.add", json!({}), LONG).await;
    step("machine.list after");
    let l = call(&mut c, "machine.list", json!({}), T).await;
    if let Some(ms_) = result_of(&l)["machines"].as_array() {
        for m in ms_.clone() {
            if let Some(n) = m["name"].as_str() {
                step(&format!("machine.rm {n} (cleanup of an added record)"));
                call(&mut c, "machine.rm", json!({"machine":n}), T).await;
            }
        }
    }
}

async fn m5() {
    let (mut a, _) = framed("m5-sub", hello_framed()).await;
    step("events.subscribe {}");
    let r = call(&mut a, "events.subscribe", json!({}), T).await;
    let ch = result_of(&r)["channel"].as_u64().unwrap_or(1) as u32;
    println!("ANALYSIS event channel = {ch}");
    a.drain(Duration::from_millis(300)).await;
    step(&format!("send CLOSE (kind 2, empty payload) on channel {ch}; watch 2 s"));
    a.send_frame(ch, CLOSE, &[]).await;
    let evs = a.drain(Duration::from_secs(2)).await;
    let closes = evs
        .iter()
        .filter(|e| matches!(e, Ev::Frame { ch: c, kind: CLOSE, .. } if *c == ch))
        .count();
    println!("ANALYSIS frames received in 2 s after our CLOSE = {}, CLOSE frames on ch {ch} = {closes}", evs.len());
    step("second connection runs `sh -c \"sleep 0.2\"`; watch subscriber 3 s");
    let (mut b, _) = framed("m5-other", hello_framed()).await;
    call(&mut b, "terminal.run", json!({"argv":["sh","-c","sleep 0.2"]}), T).await;
    let evs = a.drain(Duration::from_secs(3)).await;
    let on_ch = evs
        .iter()
        .filter(|e| matches!(e, Ev::Frame { ch: c, .. } if *c == ch))
        .count();
    println!("ANALYSIS frames on ch {ch} after CLOSE while a terminal was created+finished = {on_ch} (all frames: {})", evs.len());
    step("channel 0 still usable? terminal.list on the subscriber connection");
    call(&mut a, "terminal.list", json!({}), T).await;
    step("subscribe again on the same connection: which channel number?");
    let r2 = call(&mut a, "events.subscribe", json!({}), T).await;
    println!("ANALYSIS second subscription channel = {}", result_of(&r2)["channel"]);
    step("another terminal on second connection; are events delivered on the new channel?");
    call(&mut b, "terminal.run", json!({"argv":["sh","-c","true"]}), T).await;
    a.drain(Duration::from_secs(2)).await;
}

async fn m6() {
    let (mut a, _) = framed("m6-sub", hello_framed()).await;
    step("events.subscribe {}");
    call(&mut a, "events.subscribe", json!({}), T).await;
    let (mut b, _) = framed("m6-run", hello_framed()).await;
    let script = "printf '\\033]7501;working\\007'; sleep 1; printf '\\033]7501;idle\\007'; sleep 0.5; printf '\\033]0;hfy-title\\007'; sleep 1";
    step(&format!("terminal.run argv=[sh,-c,{script:?}]; watch events 5 s"));
    let r = call(&mut b, "terminal.run", json!({"argv":["sh","-c",script]}), T).await;
    let t = result_of(&r)["address"].as_str().unwrap_or("").to_string();
    let evs = a.drain(Duration::from_secs(5)).await;
    let mut types = Vec::new();
    for e in &evs {
        if let Ev::Frame { kind: DATA, payload, ch, .. } = e
            && *ch != 0
        {
            for line in String::from_utf8_lossy(payload).split('\n').filter(|l| !l.is_empty()) {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    types.push(format!("{}:{}", v["params"]["type"].as_str().unwrap_or("-"), v["params"]["data"]));
                }
            }
        }
    }
    println!("ANALYSIS event types+data in order for {t}:");
    for x in &types {
        println!("ANALYSIS   {x}");
    }
    step("terminal.get after (status/turn/title fields)");
    call(&mut b, "terminal.get", json!({"terminal":t}), T).await;
}

async fn m7() {
    let (mut c, _) = framed("m7", hello_framed()).await;
    step("templates.list on a fresh isolated core");
    call(&mut c, "templates.list", json!({}), T).await;
    step("terminal.run argv=[sh,-c,\"sleep 5\"] wait:true timeout:1");
    call(
        &mut c,
        "terminal.run",
        json!({"argv":["sh","-c","sleep 5"],"wait":true,"timeout":1}),
        LONG,
    )
    .await;
    step("terminal.list right after");
    call(&mut c, "terminal.list", json!({}), T).await;
}

async fn wait_child(c: &mut Conn, t: Duration) {
    match tokio::time::timeout(t, c.child.wait()).await {
        Ok(Ok(st)) => println!("[{}] +{}ms `hc connect` EXITED status={st} code={:?}", c.name, ms(), st.code()),
        Ok(Err(e)) => println!("[{}] +{}ms wait error {e}", c.name, ms()),
        Err(_) => println!("[{}] +{}ms `hc connect` still running after {} ms", c.name, ms(), t.as_millis()),
    }
}

async fn m4() {
    let (mut a, _) = framed("m4-a", hello_framed()).await;
    step("server.status");
    call(&mut a, "server.status", json!({}), T).await;
    step("server.restart {}; then watch connection 8 s");
    call(&mut a, "server.restart", json!({}), LONG).await;
    a.drain(Duration::from_secs(8)).await;
    wait_child(&mut a, Duration::from_millis(500)).await;
    step("try a call on the old connection after restart");
    call(&mut a, "server.status", json!({}), Duration::from_secs(3)).await;
    wait_child(&mut a, Duration::from_millis(500)).await;
    step("new `hc connect` after restart: server.status (pid/uptime)");
    let (mut b, h) = framed("m4-b", hello_framed()).await;
    println!("ANALYSIS hello after restart = {h}");
    call(&mut b, "server.status", json!({}), T).await;
    call(&mut b, "terminal.list", json!({"all":true}), T).await;
    step("server.stop {}; then watch connection 8 s");
    call(&mut b, "server.stop", json!({}), LONG).await;
    b.drain(Duration::from_secs(8)).await;
    wait_child(&mut b, Duration::from_secs(5)).await;
}

#[tokio::main]
async fn main() {
    let _ = ms();
    let arg = std::env::args().nth(1).unwrap_or_default();
    match arg.as_str() {
        "m1" => m1().await,
        "m2" => m2().await,
        "m3" => m3().await,
        "m4" => m4().await,
        "m5" => m5().await,
        "m6" => m6().await,
        "m7" => m7().await,
        "m6b" => m6b().await,
        "m6c" => m6c().await,
        _ => eprintln!("usage: probe-methods m1|m2|m3|m4|m5|m6|m7"),
    }
    println!(
        "-- scenario returned at +{}ms: its connections are dropped (stdin closed, hc connect killed); any EOF stamped at or after this is teardown, not a core reaction --",
        ms()
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
}

fn result_of(v: &Option<Value>) -> Value {
    v.as_ref()
        .and_then(|v| v.get("result").cloned())
        .unwrap_or(Value::Null)
}

/// OSC 7501 payload variants: which ones the core accepts as status reports.
async fn m6b() {
    let variants = [
        ("state=", "printf '\\033]7501;state=working\\007'; sleep 1; printf '\\033]7501;state=idle\\007'; sleep 1"),
        ("state+msg+progress (colon-separated)", "printf '\\033]7501;state=working:msg=compiling:progress=40\\007'; sleep 1; printf '\\033]7501;state=blocked:kind=question:msg=ok?\\007'; sleep 1; printf '\\033]7501;state=done\\007'; sleep 1"),
        ("record id (;id;state=)", "printf '\\033]7501;job1;state=working\\007'; sleep 1; printf '\\033]7501;job1;state=idle\\007'; sleep 1"),
        ("clear (empty payload)", "printf '\\033]7501;state=working\\007'; sleep 1; printf '\\033]7501;\\007'; sleep 1"),
    ];
    status_variants("m6b", &variants).await;
}

async fn m6c() {
    let variants = [
        ("working:progress=40", "printf '\\033]7501;state=working:progress=40\\007'; sleep 1; printf '\\033]7501;state=idle\\007'; sleep 1"),
        ("working:msg=compiling", "printf '\\033]7501;state=working:msg=compiling\\007'; sleep 1; printf '\\033]7501;state=idle\\007'; sleep 1"),
        ("working;msg=compiling (semicolon)", "printf '\\033]7501;state=working;msg=compiling\\007'; sleep 1; printf '\\033]7501;state=idle\\007'; sleep 1"),
        ("blocked:kind=permission:msg=allow", "printf '\\033]7501;state=blocked:kind=permission:msg=allow\\007'; sleep 1; printf '\\033]7501;state=idle\\007'; sleep 1"),
        ("bare word working (no state=)", "printf '\\033]7501;working\\007'; sleep 1; printf '\\033]7501;idle\\007'; sleep 1"),
    ];
    status_variants("m6c", &variants).await;
}

async fn status_variants(tag: &'static str, variants: &[(&str, &str)]) {
    let sub: &'static str = if tag == "m6b" { "m6b-sub" } else { "m6c-sub" };
    let run: &'static str = if tag == "m6b" { "m6b-run" } else { "m6c-run" };
    let (mut a, _) = framed(sub, hello_framed()).await;
    let (mut b, _) = framed(run, hello_framed()).await;
    for &(label, script) in variants {
        step(&format!("variant {label}: subscribe on a fresh channel, run argv=[sh,-c,{script:?}], watch 4.5 s"));
        let r = call(&mut a, "events.subscribe", json!({}), T).await;
        let ch = result_of(&r)["channel"].as_u64().unwrap_or(0) as u32;
        let rr = call(&mut b, "terminal.run", json!({"argv":["sh","-c",script]}), T).await;
        let t = result_of(&rr)["address"].as_str().unwrap_or("").to_string();
        let evs = a.drain(Duration::from_millis(4500)).await;
        println!("ANALYSIS[{label}] events for {t} on ch {ch} (type: data):");
        for e in &evs {
            if let Ev::Frame { kind: DATA, payload, ch: c, .. } = e
                && *c == ch
            {
                for line in String::from_utf8_lossy(payload).split('\n').filter(|l| !l.is_empty()) {
                    if let Ok(v) = serde_json::from_str::<Value>(line)
                        && v["params"]["terminal"] == json!(t)
                    {
                        println!("ANALYSIS   {}: {}", v["params"]["type"].as_str().unwrap_or("-"), v["params"]["data"]);
                    }
                }
            }
        }
        a.send_frame(ch, CLOSE, &[]).await;
        a.drain(Duration::from_millis(200)).await;
        call(&mut b, "terminal.get", json!({"terminal":t}), T).await;
    }
}
