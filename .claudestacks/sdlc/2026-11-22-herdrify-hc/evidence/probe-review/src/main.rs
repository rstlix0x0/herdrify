//! Review follow-ups: R1 channel-0 credit past 256 KiB, R2 attach/RESIZE at 0, R3 empty identifiers,
//! R4 serde_json::from_value error text. Live parts use an isolated core over framed `hc connect`.
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, Command};

struct Conn { w: ChildStdin, r: BufReader<ChildStdout>, t0: Instant, _c: tokio::process::Child }

async fn connect(dir: &str) -> Conn {
    let mut cmd = Command::new("hc");
    cmd.arg("connect");
    for (k, _) in std::env::vars() { if k.starts_with("HC_") { cmd.env_remove(k); } }
    cmd.env("HC_SESSION", "hfy-rev").env("XDG_STATE_HOME", format!("{dir}/state"))
        .env("XDG_CONFIG_HOME", format!("{dir}/config")).env("XDG_RUNTIME_DIR", format!("{dir}/run"))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).kill_on_drop(true);
    let mut c = cmd.spawn().expect("spawn");
    let mut w = c.stdin.take().expect("stdin");
    let mut r = BufReader::new(c.stdout.take().expect("stdout"));
    w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"hello\",\"params\":{\"generation\":1,\"framed\":true}}\n").await.expect("hello");
    let mut line = String::new();
    r.read_line(&mut line).await.expect("hello reply");
    println!("hello reply framed={}", line.contains("\"framed\":true"));
    Conn { w, r, t0: Instant::now(), _c: c }
}
impl Conn {
    async fn send(&mut self, ch: u32, kind: u8, p: &[u8]) {
        let mut f = ch.to_le_bytes().to_vec(); f.push(kind); f.extend((p.len() as u32).to_le_bytes()); f.extend(p);
        self.w.write_all(&f).await.expect("write frame");
    }
    async fn call(&mut self, id: u64, method: &str, params: serde_json::Value) {
        let req = serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string();
        println!("-> +{}ms ch=0 {req}", self.t0.elapsed().as_millis());
        self.send(0, 0, req.as_bytes()).await;
    }
    async fn recv(&mut self, wait: Duration) -> Option<(u32, u8, Vec<u8>)> {
        let mut h = [0u8; 9];
        match tokio::time::timeout(wait, self.r.read_exact(&mut h)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => { println!("<- +{}ms EOF/err: {e}", self.t0.elapsed().as_millis()); return None; }
            Err(_) => return None,
        }
        let ch = u32::from_le_bytes([h[0], h[1], h[2], h[3]]); let kind = h[4];
        let len = u32::from_le_bytes([h[5], h[6], h[7], h[8]]) as usize;
        let mut p = vec![0u8; len]; self.r.read_exact(&mut p).await.expect("payload");
        Some((ch, kind, p))
    }
    async fn reply(&mut self, id: u64) -> String {
        loop {
            let Some((ch, kind, p)) = self.recv(Duration::from_secs(10)).await else { return "(no reply)".into() };
            let s = String::from_utf8_lossy(&p).into_owned();
            if ch == 0 && kind == 0 && s.contains(&format!("\"id\":{id},")) {
                println!("<- +{}ms ch=0 {}", self.t0.elapsed().as_millis(), if s.len() > 400 { format!("{}…[{} bytes]", &s[..400], s.len()) } else { s.clone() });
                return s;
            }
            println!("<- +{}ms (other) ch={ch} kind={kind} len={}", self.t0.elapsed().as_millis(), p.len());
        }
    }
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct Sample { cols: u32 }

#[tokio::main]
async fn main() {
    println!("== R4 serde_json::from_value error Display (no hc)");
    for v in [serde_json::json!({"cols": -1}), serde_json::json!({"cols": 4294967296u64}), serde_json::json!({"cols": "x"}), serde_json::json!({})] {
        println!("from_value::<Sample>({v}) = {:?}", serde_json::from_value::<Sample>(v.clone()).map_err(|e| e.to_string()));
    }
    let dir = std::env::args().nth(1).expect("scratch dir");

    println!("== R1 channel 0 past 256 KiB without any CREDIT: 6 concurrent rpc.discover");
    let mut c = connect(&dir).await;
    for id in 1..=6 { c.call(id, "rpc.discover", serde_json::json!({})).await; }
    let (mut bytes, mut replies, mut credit_frames) = (0usize, 0u32, 0u32);
    while replies < 6 {
        let Some((ch, kind, p)) = c.recv(Duration::from_secs(10)).await else { break };
        if kind == 1 { credit_frames += 1; }
        if ch == 0 && kind == 0 { bytes += p.len(); replies += 1; }
    }
    println!("ch0 replies={replies} ch0 payload bytes={bytes} (256 KiB = 262144) core CREDIT frames={credit_frames}; client sent 0 CREDIT");

    println!("== R2 terminal.attach with cols 0 / rows 0, and RESIZE 0x0 on a live attach");
    c.call(10, "terminal.run", serde_json::json!({"argv":["sh"],"name":"rev-sh"})).await; c.reply(10).await;
    c.call(11, "terminal.attach", serde_json::json!({"terminal":"rev-sh","cols":0,"rows":0})).await;
    let r11 = c.reply(11).await;
    let ch = serde_json::from_str::<serde_json::Value>(&r11).ok().and_then(|v| v["result"]["channel"].as_u64());
    c.call(12, "terminal.attach", serde_json::json!({"terminal":"rev-sh","cols":0,"rows":5})).await; c.reply(12).await;
    c.call(13, "terminal.attach", serde_json::json!({"terminal":"rev-sh","cols":80,"rows":24})).await;
    let r13 = c.reply(13).await;
    let ch13 = serde_json::from_str::<serde_json::Value>(&r13).ok().and_then(|v| v["result"]["channel"].as_u64()).expect("attach channel");
    println!("first attach channel={ch:?}; valid attach channel={ch13}");
    let mut resize = 0u16.to_le_bytes().to_vec(); resize.extend(0u16.to_le_bytes());
    println!("-> RESIZE 0x0 on ch={ch13}");
    c.send(ch13 as u32, 3, &resize).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    c.call(14, "terminal.get", serde_json::json!({"terminal":"rev-sh"})).await;
    let r14 = c.reply(14).await;
    let v14: serde_json::Value = serde_json::from_str(&r14).unwrap_or_default();
    println!("after RESIZE 0x0: terminal cols={} rows={} (connection alive: {})", v14["result"]["cols"], v14["result"]["rows"], !r14.starts_with("(no"));

    println!("== R3 empty identifier values not probed before");
    let cases: &[(&str, serde_json::Value)] = &[
        ("machine.connect", serde_json::json!({"machine":""})),
        ("machine.rename", serde_json::json!({"machine":"","new_name":"x"})),
        ("machine.rm", serde_json::json!({"machine":""})),
        ("terminal.run", serde_json::json!({"argv":["true"],"on":""})),
        ("terminal.run", serde_json::json!({"argv":["true"],"cwd":""})),
        ("terminal.run", serde_json::json!({"argv":["true"],"env":{"":"x"}})),
        ("terminal.run", serde_json::json!({"argv":["true"],"parent":""})),
        ("terminal.get", serde_json::json!({"terminal":"rev-sh","caller":""})),
        ("terminal.put", serde_json::json!({"terminal":"rev-sh","name":"","data":"aGVsbG8="})),
        ("terminal.keys", serde_json::json!({"terminal":"rev-sh","keys":[""]})),
        ("machine.add", serde_json::json!({"name":"m2","via":"false","session":""})),
        ("events.subscribe", serde_json::json!({"children_of":""})),
        ("", serde_json::json!({})),
    ];
    for (i, (m, p)) in cases.iter().enumerate() {
        let id = 100 + i as u64;
        c.call(id, m, p.clone()).await;
        c.reply(id).await;
    }
    c.call(200, "server.stop", serde_json::json!({})).await; c.reply(200).await;
}
