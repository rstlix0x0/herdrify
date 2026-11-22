//! Throwaway probe: herdr-core (`hc` 0.1.21) FRAMED mode over `hc connect`.
//! Every hc child runs in an isolated env (HC_SESSION=hfy-framed, scratch XDG dirs from HFY_D).

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
            let limit = 3000;
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
    _child: Child,
    sent: usize,
}

impl Conn {
    fn spawn(name: &'static str, mode: Mode) -> Self {
        let d = std::env::var("HFY_D").expect("HFY_D must be set to the scratch dir");
        let mut child = Command::new(HC)
            .arg("connect")
            .env_remove("HC_TERMINAL_SOCKET")
            .env_remove("HC_SCOPE")
            .env_remove("HC_TERMINAL_ID")
            .env_remove("HC_SOCKET")
            .env("HC_SESSION", "hfy-framed")
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
            "[{name}] +{}ms spawned `hc connect` pid={:?} (HC_SESSION=hfy-framed, XDG_*={d}/...)",
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
            _child: child,
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

fn result_of(v: &Option<Value>) -> Value {
    v.as_ref()
        .and_then(|v| v.get("result").cloned())
        .unwrap_or(Value::Null)
}

async fn b1() {
    let (mut c, hello) = framed("b1", json!({"generation":1,"framed":true})).await;
    println!("ANALYSIS hello reply (parsed): {hello}");
    // terminal.list with no trailing newline in our payload
    c.rpc(2, "terminal.list", json!({})).await;
    let mut o = Vec::new();
    let r2 = c.reply(2, T, &mut o).await;
    println!("ANALYSIS id=2 reply received={}", r2.is_some());
    // same, but our payload ends with '\n'
    let msg = format!(
        "{}\n",
        json!({"jsonrpc":"2.0","id":3,"method":"terminal.list","params":{}})
    );
    c.send_frame(0, DATA, msg.as_bytes()).await;
    let r3 = c.reply(3, T, &mut o).await;
    println!(
        "ANALYSIS id=3 (payload with trailing \\n) reply received={}",
        r3.is_some()
    );
}

async fn b2() {
    let (mut c, _) = framed("b2", json!({"generation":1,"framed":true})).await;
    let mut o = Vec::new();
    c.rpc(5, "terminal.run", json!({"argv":["sh","-c","sleep 3"]}))
        .await;
    let r = c.reply(5, T, &mut o).await;
    let tid = result_of(&r)["address"].as_str().unwrap_or("?").to_string();
    println!("ANALYSIS terminal id = {tid}");
    c.rpc(10, "terminal.wait", json!({"terminal":tid,"timeout":10}))
        .await;
    c.rpc(11, "terminal.list", json!({})).await;
    let mut order = Vec::new();
    let mut credits = 0;
    let deadline = Instant::now() + Duration::from_secs(15);
    while order.len() < 2 && Instant::now() < deadline {
        if let Some(ev) = c.recv(Duration::from_secs(1)).await {
            if let Ev::Frame { kind: CREDIT, .. } = ev {
                credits += 1;
            }
            if let Some(v) = as_rpc(&ev)
                && let Some(id) = v.get("id").and_then(Value::as_u64)
            {
                order.push((id, ms()));
            }
        }
    }
    let rest = c.drain(Duration::from_millis(500)).await;
    credits += rest
        .iter()
        .filter(|e| matches!(e, Ev::Frame { kind: CREDIT, .. }))
        .count();
    println!(
        "ANALYSIS reply order (id, ms): {order:?}; CREDIT frames received on any channel: {credits}"
    );
}

fn analyze_event_frames(evs: &[Ev], ch: u32, label: &str, acc: &mut Vec<u8>) -> usize {
    let mut bytes = 0;
    for e in evs {
        if let Ev::Frame {
            ch: c,
            kind,
            payload,
        } = e
        {
            if *c == ch {
                bytes += payload.len();
                acc.extend_from_slice(payload);
                let s = String::from_utf8_lossy(payload);
                let msgs = s.split('\n').filter(|l| !l.is_empty()).count();
                let methods: Vec<String> = s
                    .split('\n')
                    .filter(|l| !l.is_empty())
                    .map(|l| {
                        serde_json::from_str::<Value>(l)
                            .map(|v| {
                                format!(
                                    "{}:{}",
                                    v["method"].as_str().unwrap_or("?"),
                                    v["params"]["type"].as_str().unwrap_or("-")
                                )
                            })
                            .unwrap_or_else(|_| "NOT-JSON".into())
                    })
                    .collect();
                println!(
                    "ANALYSIS[{label}] event-channel frame kind={}/{} len={} newline_separated_pieces_in_frame={msgs} trailing_newline={} per_piece_json_parse={methods:?}",
                    kind,
                    kind_name(*kind),
                    payload.len(),
                    s.ends_with('\n')
                );
            } else {
                println!(
                    "ANALYSIS[{label}] frame on ch={c} kind={}",
                    kind_name(*kind)
                );
            }
        }
    }
    bytes
}

async fn b3() {
    let (mut a, _) = framed("b3-sub", json!({"generation":1,"framed":true})).await;
    let mut o = Vec::new();
    a.rpc(2, "events.subscribe", json!({})).await;
    let r = a.reply(2, T, &mut o).await;
    let ch = result_of(&r)["channel"].as_u64().unwrap_or(0) as u32;
    println!(
        "ANALYSIS events.subscribe channel = {ch}; full reply = {}",
        r.unwrap_or(Value::Null)
    );

    let mut b = Conn::spawn("b3-other", Mode::Plain);
    b.send_line(&json!({"jsonrpc":"2.0","id":1,"method":"terminal.run","params":{"argv":["sh","-c","echo hi"]}}).to_string()).await;
    let mut ob = Vec::new();
    let rb = b.reply(1, T, &mut ob).await;
    let tid = result_of(&rb)["address"]
        .as_str()
        .unwrap_or("?")
        .to_string();
    b.send_line(&json!({"jsonrpc":"2.0","id":2,"method":"terminal.wait","params":{"terminal":tid,"timeout":10}}).to_string()).await;
    let _ = b.reply(2, Duration::from_secs(12), &mut ob).await;
    let mut acc: Vec<u8> = Vec::new();
    let evs = a.drain(Duration::from_secs(2)).await;
    let mut total = analyze_event_frames(&evs, ch, "first terminal", &mut acc);

    println!("== step: 5 more short terminals, still sending NO credit on ch={ch}");
    for i in 0..5u64 {
        b.send_line(
            &json!({"jsonrpc":"2.0","id":10+i,"method":"terminal.run","params":{"argv":["true"]}})
                .to_string(),
        )
        .await;
        let _ = b.reply(10 + i, T, &mut ob).await;
    }
    let evs = a.drain(Duration::from_secs(3)).await;
    total += analyze_event_frames(&evs, ch, "5 more", &mut acc);
    println!("ANALYSIS total event-channel bytes so far (no credit sent) = {total}");

    println!("== step: idle 17 s waiting for heartbeat (no credit sent)");
    let evs = a.drain(Duration::from_secs(17)).await;
    total += analyze_event_frames(&evs, ch, "idle", &mut acc);
    println!("ANALYSIS total event-channel bytes so far (no credit sent) = {total}");

    println!(
        "== step: exhaust 256 KiB credit with big-label terminals (labels 4 x 8000 chars each), still NO credit"
    );
    let big = "x".repeat(8000);
    let labels = json!({"l1":big,"l2":big,"l3":big,"l4":big});
    for i in 0..14u64 {
        b.send_line(&json!({"jsonrpc":"2.0","id":100+i,"method":"terminal.run","params":{"argv":["true"],"labels":labels}}).to_string()).await;
        let rr = b.reply(100 + i, T, &mut ob).await;
        if let Some(e) = rr.as_ref().and_then(|v| v.get("error")) {
            println!("ANALYSIS terminal.run with big labels error: {e}");
            break;
        }
    }
    let evs = a.drain(Duration::from_secs(4)).await;
    let got = analyze_event_frames(&evs, ch, "big-labels", &mut acc);
    total += got;
    println!(
        "ANALYSIS event bytes during big-label step = {got}; cumulative without any credit = {total} (256 KiB = 262144)"
    );

    println!("== step: send CREDIT for all {total} consumed bytes on ch={ch}, then watch");
    a.send_frame(ch, CREDIT, &(total as u32).to_le_bytes())
        .await;
    let evs = a.drain(Duration::from_secs(4)).await;
    let got2 = analyze_event_frames(&evs, ch, "after-credit", &mut acc);
    println!("ANALYSIS event bytes delivered after CREDIT = {got2}");
    reassemble(&acc);
}

async fn b4() {
    let (mut c, _) = framed("b4", json!({"generation":1,"framed":true})).await;
    let mut o = Vec::new();
    c.rpc(
        2,
        "terminal.run",
        json!({"argv":["sh"],"size":{"cols":80,"rows":24}}),
    )
    .await;
    let r = c.reply(2, T, &mut o).await;
    let tid = result_of(&r)["address"].as_str().unwrap_or("?").to_string();
    println!("ANALYSIS sh terminal id = {tid}");
    tokio::time::sleep(Duration::from_millis(500)).await;

    c.rpc(
        3,
        "terminal.attach",
        json!({"terminal":tid,"cols":80,"rows":24}),
    )
    .await;
    let r = c.reply(3, T, &mut o).await;
    println!(
        "ANALYSIS full attach reply (pretty):\n{}",
        serde_json::to_string_pretty(&r).unwrap_or_default()
    );
    let ch = result_of(&r)["channel"].as_u64().unwrap_or(0) as u32;
    let mut consumed: usize = 0;
    let mut evs = c.drain(Duration::from_secs(1)).await;
    evs.splice(0..0, o.drain(..));
    for e in &evs {
        if let Ev::Frame {
            ch: fc,
            kind: DATA,
            payload,
        } = e
            && *fc == ch
        {
            consumed += payload.len();
            println!(
                "ANALYSIS first-paint/stream DATA len={} head64hex=[{}]",
                payload.len(),
                hex(&payload[..payload.len().min(64)])
            );
        }
    }
    println!("== step: DATA \"echo framed-ok\\r\"");
    c.send_frame(ch, DATA, b"echo framed-ok\r").await;
    for e in c.drain(Duration::from_secs(1)).await {
        if let Ev::Frame {
            ch: fc,
            kind: DATA,
            payload,
        } = e
            && fc == ch
        {
            consumed += payload.len();
        }
    }
    println!("== step: CREDIT for {consumed} bytes consumed");
    c.send_frame(ch, CREDIT, &(consumed as u32).to_le_bytes())
        .await;
    let _ = c.drain(Duration::from_millis(500)).await;
    println!("== step: RESIZE cols=100 rows=30, then `stty size`");
    let mut rz = Vec::new();
    rz.extend_from_slice(&100u16.to_le_bytes());
    rz.extend_from_slice(&30u16.to_le_bytes());
    c.send_frame(ch, RESIZE, &rz).await;
    let _ = c.drain(Duration::from_millis(500)).await;
    c.send_frame(ch, DATA, b"stty size\r").await;
    let _ = c.drain(Duration::from_secs(1)).await;
    println!("== step: CLOSE (empty) from viewer");
    c.send_frame(ch, CLOSE, b"").await;
    let _ = c.drain(Duration::from_secs(1)).await;
    c.rpc(4, "terminal.get", json!({"terminal":tid})).await;
    let r4 = c.reply(4, T, &mut o).await;
    println!(
        "ANALYSIS terminal state after detach: {}",
        result_of(&r4)["state"]
    );

    println!("== step: attach again, then DATA \"exit\\r\"");
    c.rpc(
        5,
        "terminal.attach",
        json!({"terminal":tid,"cols":100,"rows":30}),
    )
    .await;
    let r5 = c.reply(5, T, &mut o).await;
    let ch2 = result_of(&r5)["channel"].as_u64().unwrap_or(0) as u32;
    println!(
        "ANALYSIS second attach channel = {ch2}; reply keys besides terminal: channel={} automatic_input={} resize_notifications={}",
        result_of(&r5)["channel"],
        result_of(&r5)["automatic_input"],
        result_of(&r5)["resize_notifications"]
    );
    let _ = c.drain(Duration::from_millis(800)).await;
    c.send_frame(ch2, DATA, b"exit\r").await;
    for e in c.drain(Duration::from_secs(2)).await {
        if let Ev::Frame {
            ch: fc,
            kind: CLOSE,
            payload,
        } = e
        {
            println!(
                "ANALYSIS CLOSE on ch={fc} exact payload bytes: {:?} (hex [{}])",
                String::from_utf8_lossy(&payload),
                hex(&payload)
            );
        }
    }
}

async fn b5() {
    let mut c = Conn::spawn("b5-plain", Mode::Plain);
    let mut o = Vec::new();
    c.send_line(
        &json!({"jsonrpc":"2.0","id":1,"method":"terminal.run","params":{"argv":["sh"]}})
            .to_string(),
    )
    .await;
    let r = c.reply(1, T, &mut o).await;
    let tid = result_of(&r)["address"].as_str().unwrap_or("?").to_string();
    println!("== step: plain attach with NO hello at all");
    c.send_line(&json!({"jsonrpc":"2.0","id":2,"method":"terminal.attach","params":{"terminal":tid,"cols":80,"rows":24}}).to_string()).await;
    let _ = c.reply(2, T, &mut o).await;
    println!("== step: new plain connection, hello {{generation:1}} (framed absent), then attach");
    let mut d = Conn::spawn("b5-plainhello", Mode::Plain);
    d.send_line(
        &json!({"jsonrpc":"2.0","id":1,"method":"hello","params":{"generation":1}}).to_string(),
    )
    .await;
    let _ = d.reply(1, T, &mut o).await;
    d.send_line(&json!({"jsonrpc":"2.0","id":2,"method":"terminal.attach","params":{"terminal":tid,"cols":80,"rows":24}}).to_string()).await;
    let _ = d.reply(2, T, &mut o).await;
    c.send_line(
        &json!({"jsonrpc":"2.0","id":3,"method":"terminal.kill","params":{"terminal":tid}})
            .to_string(),
    )
    .await;
    let _ = c.reply(3, T, &mut o).await;
}

fn summarize(evs: &[Ev], label: &str) {
    let eof: Vec<&str> = evs
        .iter()
        .filter_map(|e| {
            if let Ev::Eof(r) = e {
                Some(r.as_str())
            } else {
                None
            }
        })
        .collect();
    let ids: Vec<Value> = evs
        .iter()
        .filter_map(as_rpc)
        .map(|v| v["id"].clone())
        .collect();
    println!("ANALYSIS[{label}] rpc reply ids seen={ids:?} connection_eof={eof:?}");
}

async fn b6(which: &str) {
    match which {
        "a" => {
            let (mut c, _) = framed("b6a-kind9", json!({"generation":1,"framed":true})).await;
            let msg =
                json!({"jsonrpc":"2.0","id":3,"method":"terminal.list","params":{}}).to_string();
            c.send_frame(0, 9, msg.as_bytes()).await;
            c.rpc(4, "terminal.list", json!({})).await;
            let evs = c.drain(Duration::from_secs(2)).await;
            summarize(&evs, "unknown kind 9 on ch0 (id=3), then normal id=4");
        }
        "b" => {
            let (mut c, _) = framed("b6b-ch77", json!({"generation":1,"framed":true})).await;
            c.send_frame(77, DATA, b"hello\r").await;
            c.rpc(5, "terminal.list", json!({})).await;
            let evs = c.drain(Duration::from_secs(2)).await;
            summarize(&evs, "DATA on unopened ch77, then id=5");
        }
        "d" => {
            let (mut c, _) = framed("b6d-ch0credit", json!({"generation":1,"framed":true})).await;
            c.send_frame(0, CREDIT, &1000u32.to_le_bytes()).await;
            c.rpc(8, "terminal.list", json!({})).await;
            let evs = c.drain(Duration::from_secs(2)).await;
            summarize(&evs, "CREDIT on ch0, then id=8");
            c.send_frame(0, DATA, b"not json").await;
            c.rpc(9, "terminal.list", json!({})).await;
            let evs = c.drain(Duration::from_secs(2)).await;
            summarize(&evs, "non-JSON DATA on ch0, then id=9");
        }
        "c" => {
            let (mut c, _) = framed("b6c-plainline", json!({"generation":1,"framed":true})).await;
            c.send_line(
                &json!({"jsonrpc":"2.0","id":6,"method":"terminal.list","params":{}}).to_string(),
            )
            .await;
            let evs = c.drain(Duration::from_secs(2)).await;
            summarize(&evs, "plain JSON line after framed switch (id=6)");
            c.rpc(7, "terminal.list", json!({})).await;
            let evs = c.drain(Duration::from_secs(2)).await;
            summarize(&evs, "framed id=7 after the plain line");
        }
        _ => {}
    }
}

async fn b7() {
    for (name, hello) in [
        ("b7-nofeat", json!({"generation":1,"framed":true})),
        (
            "b7-feat",
            json!({"generation":1,"framed":true,"features":["automatic_input","resize_notifications"]}),
        ),
        (
            "b7-bogus",
            json!({"generation":1,"framed":true,"features":["bogus_feature"]}),
        ),
        ("b7-plain", json!({"generation":1})),
    ] {
        let mode = if hello["framed"] == json!(true) {
            Mode::FramedAfterHello
        } else {
            Mode::Plain
        };
        let mut c = Conn::spawn(name, mode);
        c.send_line(&json!({"jsonrpc":"2.0","id":1,"method":"hello","params":hello}).to_string())
            .await;
        let mut o = Vec::new();
        let r = c.reply(1, T, &mut o).await;
        let res = result_of(&r);
        println!(
            "ANALYSIS[{name}] request={hello} -> features={} framed={} generation={}",
            res["features"], res["framed"], res["generation"]
        );
    }
}

async fn b8() {
    let (mut c, hello) = framed("b8-nogen", json!({"framed":true})).await;
    println!("ANALYSIS hello without generation -> {hello}");
    c.rpc(2, "terminal.list", json!({})).await;
    let mut o = Vec::new();
    let r = c.reply(2, T, &mut o).await;
    println!(
        "ANALYSIS framed terminal.list after generation-less hello received={}",
        r.is_some()
    );
    println!("== step: second hello framed:true as a DATA frame on ch0");
    c.rpc(3, "hello", json!({"generation":1,"framed":true}))
        .await;
    let evs = c.drain(Duration::from_secs(2)).await;
    summarize(&evs, "second hello framed:true (id=3)");
    c.rpc(4, "terminal.list", json!({})).await;
    let evs = c.drain(Duration::from_secs(2)).await;
    summarize(&evs, "framed terminal.list after second hello (id=4)");

    println!("== step: new connection; framed, then second hello framed:false");
    let (mut d, _) = framed("b8-unframe", json!({"generation":1,"framed":true})).await;
    d.rpc(3, "hello", json!({"generation":1,"framed":false}))
        .await;
    let evs = d.drain(Duration::from_secs(2)).await;
    summarize(&evs, "second hello framed:false (id=3)");
    d.rpc(4, "terminal.list", json!({})).await;
    let evs = d.drain(Duration::from_secs(2)).await;
    summarize(&evs, "framed terminal.list after framed:false hello (id=4)");

    println!("== step: new connection; hello generation:2 framed:true");
    let (_e, h2) = framed("b8-gen2", json!({"generation":2,"framed":true})).await;
    println!("ANALYSIS hello generation:2 -> {h2}");
}

#[tokio::main]
async fn main() {
    let _ = ms();
    let arg = std::env::args().nth(1).unwrap_or_default();
    match arg.as_str() {
        "b1" => b1().await,
        "b2" => b2().await,
        "b3" => b3().await,
        "b4" => b4().await,
        "b5" => b5().await,
        "b6a" => b6("a").await,
        "b6b" => b6("b").await,
        "b6c" => b6("c").await,
        "b6d" => b6("d").await,
        "b7" => b7().await,
        "b8" => b8().await,
        _ => eprintln!("usage: probe-framed b1|b2|b3|b4|b5|b6a|b6b|b6c|b6d|b7|b8"),
    }
    println!(
        "-- scenario returned at +{}ms: its connections are dropped (stdin closed, hc connect killed); any EOF stamped at or after this is teardown, not a core reaction --",
        ms()
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
}

/// Concatenate every event-channel payload byte in arrival order and split on '\n'.
fn reassemble(acc: &[u8]) {
    let s = String::from_utf8_lossy(acc);
    let mut ok = 0;
    let mut bad = 0;
    let mut seqs = Vec::new();
    let mut kinds = Vec::new();
    for line in s.split('\n').filter(|l| !l.is_empty()) {
        match serde_json::from_str::<Value>(line) {
            Ok(v) => {
                ok += 1;
                let seq = v["params"]["seq"].as_u64();
                kinds.push(format!(
                    "{}:{}:seq={}",
                    v["method"].as_str().unwrap_or("?"),
                    v["params"]["type"].as_str().unwrap_or("-"),
                    seq.map_or("-".into(), |n| n.to_string())
                ));
                if v["method"] == "event" {
                    seqs.push(seq.unwrap_or(0));
                }
            }
            Err(_) => bad += 1,
        }
    }
    let contiguous = seqs.windows(2).all(|w| w[1] == w[0] + 1);
    println!(
        "ANALYSIS[reassembled stream] total_bytes={} ends_with_newline={} json_lines_ok={ok} json_lines_bad={bad} event_seq_contiguous={contiguous} first_seq={:?} last_seq={:?}",
        acc.len(),
        s.ends_with('\n'),
        seqs.first(),
        seqs.last()
    );
    println!("ANALYSIS[reassembled stream] messages in order: {kinds:?}");
}
