//! Throwaway evidence probe for herdr-core (`hc` 0.1.21) plain mode over `hc connect`.
//! Every hc process is spawned with an isolated environment derived from `HFY_D`:
//! HC_SESSION=hfy-plain, XDG_STATE_HOME=$HFY_D/state, XDG_CONFIG_HOME=$HFY_D/config,
//! XDG_RUNTIME_DIR=$HFY_D/run, and every other inherited HC_* variable removed.

use std::{
    collections::BTreeSet,
    process::Stdio,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::mpsc,
    time::timeout,
};

const HC: &str = "hc";
const SESSION: &str = "hfy-plain";
const LOG_MAX: usize = 4096;
const SCHEMA_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../upstream-v0.1.21-schema-hc.openrpc.json"
);

static T0: OnceLock<Instant> = OnceLock::new();
static CAPTURED: Mutex<Vec<(String, Value)>> = Mutex::new(Vec::new());

fn ms() -> u128 {
    T0.get().unwrap().elapsed().as_millis()
}

fn d() -> String {
    std::env::var("HFY_D").expect("HFY_D must be set to the isolated scratch dir")
}

/// An `hc` command with the isolated environment. `session_env` controls HC_SESSION.
fn hc_cmd(args: &[&str], session_env: bool) -> Command {
    let d = d();
    let mut c = Command::new(HC);
    c.args(args);
    for (k, _) in std::env::vars() {
        if k.starts_with("HC_") {
            c.env_remove(&k);
        }
    }
    if session_env {
        c.env("HC_SESSION", SESSION);
    }
    c.env("XDG_STATE_HOME", format!("{d}/state"))
        .env("XDG_CONFIG_HOME", format!("{d}/config"))
        .env("XDG_RUNTIME_DIR", format!("{d}/run"));
    c.kill_on_drop(true);
    c
}

fn show(s: &str) -> String {
    if s.len() > LOG_MAX {
        let mut cut = 300;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        format!(
            "{}...[probe: log line truncated, full length {} bytes]",
            &s[..cut],
            s.len()
        )
    } else {
        s.to_string()
    }
}

async fn run_cli(args: &[&str]) -> (Option<i32>, String, String) {
    println!("probe: $ hc {} (isolated env)", args.join(" "));
    let out = hc_cmd(args, true)
        .stdin(Stdio::null())
        .output()
        .await
        .unwrap();
    let so = String::from_utf8_lossy(&out.stdout).to_string();
    let se = String::from_utf8_lossy(&out.stderr).to_string();
    println!("probe: [+{}ms] exit status: {:?}", ms(), out.status);
    println!("probe: stdout ({} bytes):\n{}", so.len(), show(&so));
    println!("probe: stderr ({} bytes):\n{}", se.len(), se);
    (out.status.code(), so, se)
}

enum Msg {
    Line(Vec<u8>),
    Eof,
}

struct Conn {
    tag: String,
    child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::UnboundedReceiver<Msg>,
    eof: bool,
}

impl Conn {
    fn spawn(tag: &str, args: &[&str], session_env: bool) -> Self {
        println!(
            "probe: [{tag} +{}ms] spawn: hc {} ({})",
            ms(),
            args.join(" "),
            if session_env {
                "env HC_SESSION=hfy-plain + isolated XDG"
            } else {
                "HC_SESSION removed + isolated XDG"
            }
        );
        let mut child = hc_cmd(args, session_env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        let t = tag.to_string();
        tokio::spawn(async move {
            let mut r = BufReader::new(stdout);
            loop {
                let mut buf = Vec::new();
                let n = r.read_until(b'\n', &mut buf).await.unwrap_or(0);
                if n == 0 {
                    println!("<- [{t} +{}ms] <EOF on stdout>", ms());
                    let _ = tx.send(Msg::Eof);
                    break;
                }
                let text = String::from_utf8_lossy(&buf);
                let text = text.strip_suffix('\n').unwrap_or(&text);
                println!("<- [{t} +{}ms] {}", ms(), show(text));
                let _ = tx.send(Msg::Line(buf));
            }
        });
        let t = tag.to_string();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        println!("!! [{t} +{}ms] <EOF on stderr>", ms());
                        break;
                    }
                    Ok(n) => println!(
                        "!! [{t} +{}ms] stderr: {}",
                        ms(),
                        String::from_utf8_lossy(&buf[..n]).trim_end_matches('\n')
                    ),
                }
            }
        });
        Self {
            tag: tag.to_string(),
            child,
            stdin,
            rx,
            eof: false,
        }
    }

    async fn send_raw(&mut self, s: &str) {
        println!("-> [{} +{}ms] {}", self.tag, ms(), s);
        let w = self.stdin.as_mut().expect("stdin closed");
        let mut b = s.as_bytes().to_vec();
        b.push(b'\n');
        if let Err(e) = w.write_all(&b).await {
            println!("probe: [{} +{}ms] write error: {e}", self.tag, ms());
        }
        let _ = w.flush().await;
    }

    async fn send(&mut self, v: &Value) {
        self.send_raw(&serde_json::to_string(v).unwrap()).await;
    }

    async fn next(&mut self, dur: Duration) -> Option<Msg> {
        if self.eof {
            return None;
        }
        match timeout(dur, self.rx.recv()).await {
            Ok(Some(Msg::Eof)) | Ok(None) => {
                self.eof = true;
                Some(Msg::Eof)
            }
            Ok(Some(m)) => Some(m),
            Err(_) => None,
        }
    }

    /// Wait for the reply with this id; other lines are printed by the reader and skipped.
    async fn wait_id(&mut self, id: &Value, dur: Duration) -> Option<Value> {
        let deadline = Instant::now() + dur;
        loop {
            if self.eof {
                println!("probe: [{} +{}ms] connection already closed (EOF seen); no reply for id {id}", self.tag, ms());
                return None;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.next(left).await {
                Some(Msg::Line(b)) => {
                    if let Ok(v) = serde_json::from_slice::<Value>(&b)
                        && v.get("id") == Some(id)
                    {
                        return Some(v);
                    }
                }
                Some(Msg::Eof) => {
                    println!("probe: [{} +{}ms] connection closed while waiting for id {id}", self.tag, ms());
                    return None;
                }
                None => {
                    println!("probe: [{} +{}ms] no reply for id {id} within {:?}", self.tag, ms(), dur);
                    return None;
                }
            }
        }
    }

    async fn call(&mut self, id: Value, method: &str, params: Value) -> Option<Value> {
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await;
        self.wait_id(&id, Duration::from_secs(30)).await
    }

    /// Call and record a successful result for A9.
    async fn call_rec(&mut self, id: Value, method: &str, params: Value) -> Option<Value> {
        let r = self.call(id, method, params).await;
        if let Some(res) = r.as_ref().and_then(|v| v.get("result")) {
            CAPTURED
                .lock()
                .unwrap()
                .push((method.to_string(), res.clone()));
        }
        r
    }

    async fn hello(&mut self) -> Option<Value> {
        self.call(json!(0), "hello", json!({"generation":1})).await
    }

    /// Collect every line for `dur` (stops early at EOF).
    async fn collect(&mut self, dur: Duration) -> Vec<Vec<u8>> {
        let deadline = Instant::now() + dur;
        let mut out = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            match self.next(left).await {
                Some(Msg::Line(b)) => out.push(b),
                Some(Msg::Eof) | None => break,
            }
        }
        out
    }

    fn close_stdin(&mut self) {
        println!("probe: [{} +{}ms] closing child stdin", self.tag, ms());
        self.stdin = None;
    }

    async fn exit_status(&mut self, dur: Duration) -> String {
        match timeout(dur, self.child.wait()).await {
            Ok(Ok(s)) => format!("exited at +{}ms with {:?}", ms(), s),
            Ok(Err(e)) => format!("wait error {e}"),
            Err(_) => format!("still running after {:?} (at +{}ms)", dur, ms()),
        }
    }
}

fn addr(v: &Option<Value>) -> String {
    v.as_ref()
        .and_then(|v| v["result"]["address"].as_str())
        .unwrap_or("?")
        .to_string()
}

fn save_captured() {
    let path = format!("{}/captured.json", d());
    let mut all: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    for (m, r) in CAPTURED.lock().unwrap().drain(..) {
        all.push(json!({"method": m, "result": r}));
    }
    std::fs::write(&path, serde_json::to_string(&all).unwrap()).unwrap();
    println!("probe: captured results now {} entries in $HFY_D/captured.json", all.len());
}

fn schema() -> Value {
    serde_json::from_str(&std::fs::read_to_string(SCHEMA_FILE).unwrap()).unwrap()
}

// ---------------------------------------------------------------- A1
fn sock_exists() -> bool {
    std::path::Path::new(&format!("{}/run/herdr-core/{SESSION}.sock", d())).exists()
}

async fn a1(default_pid: u64) {
    println!(
        "probe: env for every hc: HC_SESSION=hfy-plain XDG_STATE_HOME=$HFY_D/state XDG_CONFIG_HOME=$HFY_D/config XDG_RUNTIME_DIR=$HFY_D/run, all other HC_* removed"
    );
    println!("probe: step 1, make sure the isolated core is not running");
    run_cli(&["server", "stop"]).await;
    println!("probe: isolated socket $HFY_D/run/herdr-core/hfy-plain.sock exists = {}", sock_exists());

    println!("probe: step 2, hc connect with no core running (no hc server start)");
    let mut c1 = Conn::spawn("c1", &["connect"], true);
    let h = c1.hello().await;
    if h.is_none() {
        println!("probe: hello got no reply; exit: {}", c1.exit_status(Duration::from_secs(2)).await);
        println!("probe: running hc server start");
        run_cli(&["server", "start"]).await;
        c1 = Conn::spawn("c1b", &["connect"], true);
        c1.hello().await;
    }
    let s1 = c1.call(json!(1), "server.status", json!({})).await;
    let pid1 = s1.as_ref().and_then(|v| v["result"]["pid"].as_u64());
    println!("probe: socket exists now = {}", sock_exists());
    println!("probe: step 3, CLI status after connect");
    run_cli(&["server", "status", "--json"]).await;
    println!(
        "probe: isolated pid {:?} vs default pid {default_pid}: differ = {}",
        pid1,
        pid1.is_some_and(|p| p != default_pid)
    );

    println!("probe: step 4, hc connect --session hfy-plain with HC_SESSION removed");
    let mut c2 = Conn::spawn("c2", &["connect", "--session", SESSION], false);
    c2.hello().await;
    let s2 = c2.call(json!(1), "server.status", json!({})).await;
    let pid2 = s2.as_ref().and_then(|v| v["result"]["pid"].as_u64());
    println!(
        "probe: --session pid {:?} vs HC_SESSION pid {:?}: same core = {}",
        pid2,
        pid1,
        pid1.is_some() && pid1 == pid2
    );
    let default_sock = std::path::Path::new(&format!("{}/run/herdr-core/default.sock", d())).exists();
    println!("probe: an isolated default.sock exists (would mean --session was ignored) = {default_sock}");
    drop(c1);
    drop(c2);

    println!("probe: step 5, does `hc server status` itself start a stopped core?");
    run_cli(&["server", "stop"]).await;
    println!("probe: socket exists after stop = {}", sock_exists());
    run_cli(&["server", "status", "--json"]).await;
    println!("probe: socket exists after status = {}", sock_exists());
}

// ---------------------------------------------------------------- A2
async fn a2() {
    let mut c = Conn::spawn("a2", &["connect"], true);
    c.send(&json!({"jsonrpc":"2.0","id":1,"method":"terminal.list","params":{}}))
        .await;
    let r = c.wait_id(&json!(1), Duration::from_secs(5)).await;
    println!("probe: request before hello answered = {}", r.is_some());
    let h = c.call(json!(2), "hello", json!({"generation":1})).await;
    println!("probe: hello afterwards answered = {}", h.is_some());
    let r3 = c.call(json!(3), "terminal.list", json!({})).await;
    println!("probe: terminal.list after hello answered = {}", r3.is_some());
}

// ---------------------------------------------------------------- A3
async fn a3() {
    let sch = schema();
    let schema_methods: BTreeSet<String> = sch["methods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap().to_string())
        .collect();

    println!("probe: case generation 1");
    let mut a = Conn::spawn("g1", &["connect"], true);
    let r = a.call_rec(json!(1), "hello", json!({"generation":1})).await;
    if let Some(res) = r.as_ref().map(|v| &v["result"]) {
        let feats: Vec<&str> = res["features"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let methods: BTreeSet<String> = res["methods"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        println!("probe: features ({}): {:?}", feats.len(), feats);
        println!("probe: methods count: {}", methods.len());
        println!("probe: schema methods count: {}", schema_methods.len());
        println!(
            "probe: in hello not in schema: {:?}",
            methods.difference(&schema_methods).collect::<Vec<_>>()
        );
        println!(
            "probe: in schema not in hello: {:?}",
            schema_methods.difference(&methods).collect::<Vec<_>>()
        );
    }
    println!("probe: second hello on same connection");
    a.call(json!(2), "hello", json!({"generation":1})).await;
    println!("probe: connection still usable after second hello?");
    a.call(json!(3), "terminal.list", json!({})).await;

    for (tag, params) in [
        ("gnone", json!({})),
        ("g2", json!({"generation":2})),
        ("g0", json!({"generation":0})),
    ] {
        println!("probe: case hello {params}");
        let mut c = Conn::spawn(tag, &["connect"], true);
        let r = c.call(json!(1), "hello", params).await;
        if tag == "gnone" {
            if let Some(v) = &r
                && v.get("result").is_some()
            {
                CAPTURED
                    .lock()
                    .unwrap()
                    .push(("hello".into(), v["result"].clone()));
            }
        }
        println!("probe: connection usable afterwards (terminal.list)?");
        let r2 = c.call(json!(2), "terminal.list", json!({})).await;
        println!("probe: answered = {}", r2.is_some());
        println!("probe: hello generation 1 afterwards?");
        c.call(json!(3), "hello", json!({"generation":1})).await;
    }
    save_captured();
}

// ---------------------------------------------------------------- A4
async fn a4() {
    let cases: Vec<(&str, String)> = vec![
        ("terminal.get t999", r#"{"jsonrpc":"2.0","id":1,"method":"terminal.get","params":{"terminal":"t999"}}"#.into()),
        ("terminal.get missing required", r#"{"jsonrpc":"2.0","id":2,"method":"terminal.get","params":{}}"#.into()),
        ("terminal.get wrong type", r#"{"jsonrpc":"2.0","id":3,"method":"terminal.get","params":{"terminal":5}}"#.into()),
        ("unknown method", r#"{"jsonrpc":"2.0","id":4,"method":"nope.x","params":{}}"#.into()),
        ("malformed line", "{".into()),
        ("empty line", String::new()),
        ("params as array", r#"{"jsonrpc":"2.0","id":7,"method":"terminal.list","params":[]}"#.into()),
        ("string id", r#"{"jsonrpc":"2.0","id":"abc","method":"terminal.list","params":{}}"#.into()),
        ("null id", r#"{"jsonrpc":"2.0","id":null,"method":"terminal.list","params":{}}"#.into()),
        ("no id (notification)", r#"{"jsonrpc":"2.0","method":"terminal.list","params":{}}"#.into()),
        ("extra: params omitted", r#"{"jsonrpc":"2.0","id":11,"method":"terminal.list"}"#.into()),
        ("extra: jsonrpc member missing", r#"{"id":12,"method":"terminal.list","params":{}}"#.into()),
        ("extra: unknown param key", r#"{"jsonrpc":"2.0","id":13,"method":"terminal.list","params":{"bogus":1}}"#.into()),
    ];
    for (i, (label, line)) in cases.into_iter().enumerate() {
        println!("probe: ---- case {}: {label}", i + 1);
        let mut c = Conn::spawn(&format!("a4.{}", i + 1), &["connect"], true);
        c.hello().await;
        c.send_raw(&line).await;
        let got = c.collect(Duration::from_secs(2)).await;
        println!("probe: lines received within 2s after the case: {}", got.len());
        if c.eof {
            println!("probe: connection survived = false (EOF); {}", c.exit_status(Duration::from_secs(2)).await);
            continue;
        }
        let ok = c
            .call(json!(99), "server.status", json!({}))
            .await
            .is_some();
        println!("probe: connection survived (server.status id 99 answered) = {ok}");
    }
}

// ---------------------------------------------------------------- A5
async fn a5() {
    let mut c = Conn::spawn("a5", &["connect"], true);
    c.hello().await;
    let run = c
        .call(json!(1), "terminal.run", json!({"argv":["sh","-c","sleep 3"]}))
        .await;
    let t = addr(&run);
    c.send(&json!({"jsonrpc":"2.0","id":10,"method":"terminal.wait","params":{"terminal":t,"timeout":10}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":11,"method":"terminal.list","params":{}}))
        .await;
    let mut order = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while order.len() < 2 {
        match c.next(deadline.saturating_duration_since(Instant::now())).await {
            Some(Msg::Line(b)) => {
                if let Ok(v) = serde_json::from_slice::<Value>(&b)
                    && let Some(id) = v.get("id")
                {
                    order.push((id.clone(), ms()));
                }
            }
            _ => break,
        }
    }
    println!("probe: reply order (id, +ms): {order:?}");
    c.call(json!(12), "terminal.rm", json!({"terminal":t})).await;
}

// ---------------------------------------------------------------- A6
async fn a6() {
    let mut sub = Conn::spawn("sub", &["connect"], true);
    sub.hello().await;
    sub.call(json!(1), "events.subscribe", json!({})).await;

    let mut ctl = Conn::spawn("ctl", &["connect"], true);
    ctl.hello().await;
    let run = ctl
        .call(
            json!(1),
            "terminal.run",
            json!({"argv":["sh","-c","echo ev; sleep 0.5"],"wait":true,"timeout":10}),
        )
        .await;
    let t = addr(&run);
    ctl.call(json!(2), "terminal.rm", json!({"terminal":t})).await;
    ctl.call(json!(3), "doc.set", json!({"name":"probe-ev","value":{"n":1},"version":0}))
        .await;

    println!("probe: collecting notifications on sub for 2s");
    let got = sub.collect(Duration::from_secs(2)).await;
    println!("probe: {} lines on sub after ctl finished", got.len());

    println!("probe: waiting up to 20s for a heartbeat on sub");
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut hb = false;
    while !hb {
        match sub.next(deadline.saturating_duration_since(Instant::now())).await {
            Some(Msg::Line(b)) => {
                if let Ok(v) = serde_json::from_slice::<Value>(&b) {
                    hb = v["method"] == "heartbeat";
                }
            }
            _ => break,
        }
    }
    println!("probe: heartbeat seen = {hb}");

    println!("probe: request on the subscribed connection");
    sub.send(&json!({"jsonrpc":"2.0","id":5,"method":"terminal.list","params":{}}))
        .await;
    let got = sub.collect(Duration::from_secs(5)).await;
    let answered = got.iter().any(|b| {
        serde_json::from_slice::<Value>(b)
            .ok()
            .is_some_and(|v| v.get("id") == Some(&json!(5)))
    });
    println!("probe: id 5 answered/refused (a line with id 5) = {answered}; eof = {}", sub.eof);
    if sub.eof {
        println!("probe: sub {}", sub.exit_status(Duration::from_secs(2)).await);
    } else {
        println!("probe: sub child: {:?}", sub.child.try_wait());
        println!("probe: second request id 6 on subscribed connection");
        sub.send(&json!({"jsonrpc":"2.0","id":6,"method":"server.status","params":{}}))
            .await;
        let got = sub.collect(Duration::from_secs(3)).await;
        println!("probe: lines within 3s: {}; eof = {}", got.len(), sub.eof);
    }
}

// ---------------------------------------------------------------- A7
async fn a7() {
    let mut c = Conn::spawn("a7", &["connect"], true);
    c.hello().await;
    let run = c
        .call_rec(json!(1), "terminal.run", json!({"argv":["sh","-c","echo hi; sleep 1"]}))
        .await;
    let t = addr(&run);
    c.call_rec(json!(2), "terminal.read", json!({"terminal":t})).await;
    c.call_rec(json!(3), "terminal.wait", json!({"terminal":t,"timeout":10})).await;
    c.call_rec(json!(4), "terminal.get", json!({"terminal":t})).await;
    c.call_rec(json!(5), "terminal.read", json!({"terminal":t})).await;
    c.call_rec(json!(6), "terminal.rm", json!({"terminal":t})).await;

    println!("probe: terminal.rm on a live terminal");
    let live = c
        .call_rec(json!(7), "terminal.run", json!({"argv":["sh","-c","sleep 30"]}))
        .await;
    let l = addr(&live);
    c.call_rec(json!(8), "terminal.rm", json!({"terminal":l})).await;
    println!("probe: cleanup: kill then rm");
    c.call_rec(json!(9), "terminal.kill", json!({"terminal":l})).await;
    c.call_rec(json!(10), "terminal.rm", json!({"terminal":l})).await;

    c.call_rec(json!(11), "templates.list", json!({})).await;
    c.call_rec(json!(12), "server.status", json!({})).await;
    c.call_rec(json!(13), "machine.list", json!({})).await;
    c.call_rec(json!(14), "doc.get", json!({"name":"probe"})).await;
    c.call_rec(json!(15), "doc.set", json!({"name":"probe","value":{"k":"v"},"version":0}))
        .await;
    println!("probe: stale doc.set with version 0 again");
    c.call_rec(json!(16), "doc.set", json!({"name":"probe","value":{"k":"w"},"version":0}))
        .await;
    c.call_rec(json!(17), "doc.get", json!({"name":"probe"})).await;

    let path = format!("{}/probe-file.txt", d());
    std::fs::write(&path, "line1\nline2\n").unwrap();
    println!("probe: wrote 12 bytes \"line1\\nline2\\n\" to $HFY_D/probe-file.txt");
    c.call_rec(json!(18), "file.read", json!({"path":path})).await;
    c.call_rec(json!(19), "machine.exec", json!({"argv":["echo","x"]})).await;
    c.call_rec(json!(20), "terminal.list", json!({"all":true})).await;
    save_captured();
}

// ---------------------------------------------------------------- A8
fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if out.len() > 40 {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()).collect::<BTreeSet<_>>() {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => diff(p, q, &format!("{path}/{k}"), out),
                    (Some(_), None) => out.push(format!("{path}/{k}: only in left")),
                    (None, Some(_)) => out.push(format!("{path}/{k}: only in right")),
                    _ => {}
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                diff(p, q, &format!("{path}/{i}"), out);
            }
        }
        _ if a != b => out.push(format!("{path}: {} vs {}", show(&a.to_string()), show(&b.to_string()))),
        _ => {}
    }
}

async fn a8() {
    let mut c = Conn::spawn("a8", &["connect"], true);
    c.hello().await;
    let r = c.call(json!(1), "rpc.discover", json!({})).await.unwrap();
    let disc = r["result"].clone();
    let (_, so, _) = run_cli(&["schema"]).await;
    let cli: Value = serde_json::from_str(&so).unwrap();
    let file = schema();
    println!("probe: hc schema stdout bytes = {}, ends with newline = {}, contains interior newlines (pretty) = {}", so.len(), so.ends_with('\n'), so.trim_end().contains('\n'));
    println!("probe: rpc.discover result == hc schema (serde_json::Value) = {}", disc == cli);
    println!("probe: rpc.discover result == evidence upstream-v0.1.21-schema-hc.openrpc.json = {}", disc == file);
    println!("probe: hc schema == evidence file = {}", cli == file);
    let mut out = Vec::new();
    diff(&disc, &cli, "", &mut out);
    println!("probe: diff rpc.discover vs hc schema (first 40): {out:#?}");
    let compact_cli = serde_json::to_string(&cli).unwrap();
    let compact_disc = serde_json::to_string(&disc).unwrap();
    println!("probe: re-serialized compact strings equal = {}", compact_cli == compact_disc);
    println!(
        "probe: methods in rpc.discover = {}",
        disc["methods"].as_array().map_or(0, Vec::len)
    );
}

// ---------------------------------------------------------------- A9
fn resolve<'a>(root: &'a Value, s: &'a Value) -> &'a Value {
    if let Some(r) = s.get("$ref").and_then(Value::as_str)
        && let Some(name) = r.strip_prefix("#/components/schemas/")
    {
        return resolve(root, &root["components"]["schemas"][name]);
    }
    s
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_ok(t: &str, v: &Value) -> bool {
    let k = kind(v);
    t == k || (t == "number" && k == "integer")
}

fn check(root: &Value, schema: &Value, v: &Value, path: &str, out: &mut Vec<String>) {
    let s = resolve(root, schema);
    if let Some(vars) = s.get("anyOf").or_else(|| s.get("oneOf")).and_then(Value::as_array) {
        let mut best: Option<(usize, Vec<String>)> = None;
        for (i, var) in vars.iter().enumerate() {
            let mut f = Vec::new();
            check(root, var, v, path, &mut f);
            if best.as_ref().is_none_or(|(_, b)| f.len() < b.len()) {
                best = Some((i, f));
            }
        }
        if let Some((i, f)) = best {
            for x in f {
                out.push(format!("{x} (best anyOf/oneOf variant #{i} of {})", vars.len()));
            }
        }
    }
    if let Some(all) = s.get("allOf").and_then(Value::as_array) {
        for x in all {
            check(root, x, v, path, out);
        }
    }
    if let Some(t) = s.get("type") {
        let ts: Vec<&str> = match t {
            Value::String(x) => vec![x.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        if !ts.is_empty() && !ts.iter().any(|t| type_ok(t, v)) {
            out.push(format!("{path}: TYPE live {} vs schema {ts:?}", kind(v)));
            return;
        }
    }
    if let Some(c) = s.get("const")
        && c != v
    {
        out.push(format!("{path}: CONST live {v} vs schema {c}"));
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array)
        && !e.contains(v)
    {
        out.push(format!("{path}: ENUM live {v} not in {}", Value::Array(e.clone())));
    }
    match v {
        Value::Object(o) => {
            let props = s.get("properties").and_then(Value::as_object);
            let addl = s.get("additionalProperties");
            if props.is_some() || addl.is_some() {
                for (k, val) in o {
                    let p = format!("{path}/{k}");
                    if let Some(ps) = props.and_then(|p| p.get(k)) {
                        check(root, ps, val, &p, out);
                    } else if let Some(a) = addl.filter(|a| a.is_object()) {
                        check(root, a, val, &p, out);
                    } else {
                        out.push(format!("{p}: EXTRA key not declared in schema (additionalProperties: {})", addl.map_or("absent".into(), Value::to_string)));
                    }
                }
            }
            if let Some(req) = s.get("required").and_then(Value::as_array) {
                for r in req.iter().filter_map(Value::as_str) {
                    if !o.contains_key(r) {
                        out.push(format!("{path}/{r}: MISSING required key"));
                    }
                }
            }
        }
        Value::Array(a) => {
            if let Some(items) = s.get("items") {
                for (i, x) in a.iter().enumerate() {
                    check(root, items, x, &format!("{path}/{i}"), out);
                }
            }
        }
        _ => {}
    }
}

async fn a9() {
    let root = schema();
    let path = format!("{}/captured.json", d());
    let all: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    println!("probe: {} live results captured by A3 and A7", all.len());
    println!("probe: checks: extra keys, missing required keys, JSON type, const/enum; formats not checked");
    let mut total = 0;
    for (i, e) in all.iter().enumerate() {
        let m = e["method"].as_str().unwrap();
        let method = root["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["name"] == m)
            .unwrap();
        let rs = &method["result"]["schema"];
        let mut out = Vec::new();
        check(&root, rs, &e["result"], "", &mut out);
        total += out.len();
        println!(
            "probe: [{i}] {m} (result schema {}): {} finding(s)",
            rs.get("$ref").and_then(Value::as_str).unwrap_or("inline"),
            out.len()
        );
        for f in out {
            println!("probe:     {f}");
        }
    }
    println!("probe: total findings: {total}");

    println!("probe: walker control: hello result with an added key `zz_extra`, `version` removed, `generation` as string, features[0] as number");
    let mut ctl = all.iter().find(|e| e["method"] == "hello").unwrap()["result"].clone();
    let o = ctl.as_object_mut().unwrap();
    o.insert("zz_extra".into(), json!(1));
    o.remove("version");
    o.insert("generation".into(), json!("1"));
    o["features"][0] = json!(7);
    let mut out = Vec::new();
    check(&root, &json!({"$ref":"#/components/schemas/HelloResult"}), &ctl, "", &mut out);
    println!("probe: control findings ({}; expected 4):", out.len());
    for f in out {
        println!("probe:     {f}");
    }
}

// ---------------------------------------------------------------- A10
fn ws_outside_strings(b: &[u8]) -> usize {
    let (mut in_s, mut esc, mut n) = (false, false, 0);
    for &c in b {
        if in_s {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_s = false;
            }
        } else if c == b'"' {
            in_s = true;
        } else if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
            n += 1;
        }
    }
    n
}

async fn a10() {
    let mut c = Conn::spawn("a10", &["connect"], true);
    c.send(&json!({"jsonrpc":"2.0","id":0,"method":"hello","params":{"generation":1}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":1,"method":"rpc.discover","params":{}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":2,"method":"terminal.list","params":{"all":true}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":3,"method":"terminal.get","params":{"terminal":"t999"}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":4,"method":"server.status","params":{}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":5,"method":"doc.set","params":{"name":"probe-nl","value":{"text":"a\nb\r\nc"},"version":0}}))
        .await;
    c.send(&json!({"jsonrpc":"2.0","id":6,"method":"doc.get","params":{"name":"probe-nl"}}))
        .await;
    let lines = c.collect(Duration::from_secs(4)).await;
    let mut ids = Vec::new();
    for (i, b) in lines.iter().enumerate() {
        let term = if b.ends_with(b"\r\n") {
            "\\r\\n"
        } else if b.ends_with(b"\n") {
            "\\n"
        } else {
            "none"
        };
        let body = b.strip_suffix(b"\n").unwrap_or(b);
        let parsed = serde_json::from_slice::<Value>(body);
        let id = parsed.as_ref().ok().and_then(|v| v.get("id").cloned());
        if let Some(id) = &id {
            ids.push(id.clone());
        }
        println!(
            "probe: line {i}: bytes={} terminator={term} raw CR anywhere={} parses alone as one JSON value={} id={:?} whitespace bytes outside strings={} last 3 bytes={:?}",
            b.len(),
            body.contains(&b'\r'),
            parsed.is_ok(),
            id,
            ws_outside_strings(body),
            &b[b.len().saturating_sub(3)..]
        );
    }
    println!("probe: ids in reply order: {ids:?}");
    println!("probe: {} requests sent, {} lines received", 7, lines.len());
}

// ---------------------------------------------------------------- A11
async fn a11() {
    println!("probe: variant 1, after hello, idle, close stdin");
    let mut c = Conn::spawn("a11.1", &["connect"], true);
    c.hello().await;
    let t = ms();
    c.close_stdin();
    let st = c.exit_status(Duration::from_secs(10)).await;
    println!("probe: {st}; closed at +{t}ms");
    c.collect(Duration::from_millis(300)).await;

    println!("probe: variant 2, no bytes sent at all, close stdin");
    let mut c = Conn::spawn("a11.2", &["connect"], true);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let t = ms();
    c.close_stdin();
    let st = c.exit_status(Duration::from_secs(10)).await;
    println!("probe: {st}; closed at +{t}ms");
    c.collect(Duration::from_millis(300)).await;

    println!("probe: variant 3, terminal.wait in flight (program sleeps 2s), close stdin right after sending it");
    let mut c = Conn::spawn("a11.3", &["connect"], true);
    c.hello().await;
    let run = c
        .call(json!(1), "terminal.run", json!({"argv":["sh","-c","sleep 2"]}))
        .await;
    let tt = addr(&run);
    c.send(&json!({"jsonrpc":"2.0","id":2,"method":"terminal.wait","params":{"terminal":tt,"timeout":10}}))
        .await;
    let t = ms();
    c.close_stdin();
    let st = c.exit_status(Duration::from_secs(10)).await;
    println!("probe: {st}; closed at +{t}ms");
    c.collect(Duration::from_millis(300)).await;
    println!("probe: cleanup rm of {tt} on a new connection");
    let mut k = Conn::spawn("a11.cleanup", &["connect"], true);
    k.hello().await;
    k.call(json!(1), "terminal.wait", json!({"terminal":tt,"timeout":10})).await;
    k.call(json!(2), "terminal.rm", json!({"terminal":tt})).await;
}

#[tokio::main]
async fn main() {
    T0.set(Instant::now()).unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("a1") => a1(args[1].parse().unwrap()).await,
        Some("a2") => a2().await,
        Some("a3") => a3().await,
        Some("a4") => a4().await,
        Some("a5") => a5().await,
        Some("a6") => a6().await,
        Some("a7") => a7().await,
        Some("a8") => a8().await,
        Some("a9") => a9().await,
        Some("a10") => a10().await,
        Some("a11") => a11().await,
        _ => eprintln!("usage: probe-plain a1 <default-pid> | a2 .. a11"),
    }
    // let reader tasks flush their last lines
    tokio::time::sleep(Duration::from_millis(200)).await;
}
