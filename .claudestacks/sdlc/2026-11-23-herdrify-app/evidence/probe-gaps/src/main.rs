//! Probe: gaps left by probe-commands, in one isolated herdr-app 0.1.16 window.
//! A pane-less terminal for terminal.keep/park, a real selection for adjust_selection,
//! "exactly one of" argument pairs, and the window's behaviour on SIGTERM.
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

const HOME: &str = "/tmp/hzg";
const XDG: &str = "/tmp/hzgx";

fn cmd(program: &str) -> Command {
    let mut c = Command::new(program);
    for (k, _) in std::env::vars() {
        if k.starts_with("HC_") || k == "HERDR_APP_HOME" {
            c.env_remove(k);
        }
    }
    c.env("HERDR_APP_HOME", HOME)
        .env("HC_SESSION", "hzg")
        .env("XDG_STATE_HOME", format!("{XDG}/s"))
        .env("XDG_CONFIG_HOME", format!("{XDG}/c"))
        .env("XDG_RUNTIME_DIR", format!("{XDG}/r"));
    c.kill_on_drop(true);
    c
}

struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

async fn raw(mut c: Command) -> Out {
    match tokio::time::timeout(Duration::from_secs(10), c.output()).await {
        Ok(Ok(o)) => Out {
            code: o.status.code(),
            stdout: String::from_utf8_lossy(&o.stdout).into(),
            stderr: String::from_utf8_lossy(&o.stderr).into(),
        },
        Ok(Err(e)) => Out { code: None, stdout: String::new(), stderr: format!("spawn: {e}") },
        Err(_) => Out { code: None, stdout: String::new(), stderr: "TIMEOUT 10s".into() },
    }
}

fn compact(s: &str) -> String {
    serde_json::from_str::<Value>(s).map_or_else(|_| s.trim_end().to_string(), |v| v.to_string())
}

async fn run(label: &str, name: &str, args: &Value) -> Value {
    let mut c = cmd("herdr-app");
    c.args(["do", name, &args.to_string()]);
    let o = raw(c).await;
    let err = if o.stderr.is_empty() { String::new() } else { format!(" stderr={:?}", o.stderr.trim_end()) };
    println!("{label} | do {name} {args} | exit={:?} | {}{err}", o.code, compact(&o.stdout));
    serde_json::from_str(&o.stdout).unwrap_or(Value::Null)
}

async fn hc(label: &str, args: &[&str]) -> Value {
    let mut c = cmd("hc");
    c.args(args);
    let o = raw(c).await;
    println!("{label} | hc {} | exit={:?} | {} {}", args.join(" "), o.code, compact(&o.stdout), o.stderr.trim_end());
    serde_json::from_str(&o.stdout).unwrap_or(Value::Null)
}

async fn status_code() -> Option<i32> {
    let mut s = cmd("herdr-app");
    s.arg("status");
    raw(s).await.code
}

async fn launch() -> tokio::process::Child {
    for d in [HOME, XDG] {
        let _ = std::fs::remove_dir_all(d);
    }
    std::fs::create_dir_all(HOME).unwrap();
    for s in ["s", "c", "r"] {
        std::fs::create_dir_all(format!("{XDG}/{s}")).unwrap();
    }
    let mut c = cmd("herdr-app");
    c.stdout(Stdio::null()).stderr(Stdio::null());
    let child = c.spawn().expect("launch");
    let start = Instant::now();
    while status_code().await != Some(0) {
        assert!(start.elapsed() < Duration::from_secs(45), "window never answered");
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    println!("== window ready after {:?}", start.elapsed());
    child
}

#[tokio::main]
async fn main() {
    let mut ver = cmd("herdr-app");
    ver.arg("--version");
    println!("== version: {}", raw(ver).await.stdout.trim_end());
    let child = launch().await;
    let made = run("G", "space.new", &json!({"name":"hzprobe","argv":["sh","-c","i=0; while [ $i -lt 60 ]; do echo line $i hello world; i=$((i+1)); done; sleep 600"]})).await;
    let t1 = made["terminal"].as_str().unwrap_or("MISSING").to_string();
    tokio::time::sleep(Duration::from_millis(800)).await;

    // K: a terminal with no pane.
    let started = hc("K", &["run", "--bg", "--json", "--no-parent", "--", "sh", "-c", "sleep 600"]).await;
    println!("K hc run keys: {:?}", started.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));
    let loose = started["address"]
        .as_str()
        .or_else(|| started["terminal"]["address"].as_str())
        .or_else(|| started["terminal"].as_str())
        .unwrap_or("MISSING")
        .to_string();
    tokio::time::sleep(Duration::from_millis(500)).await;
    run("K", "terminal.keep", &json!({"terminal":loose,"place":"right"})).await;
    run("K", "terminal.park", &json!({"terminal":loose})).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    run("K", "terminal.keep", &json!({"terminal":loose})).await;
    run("K", "terminal.park", &json!({"terminal":loose})).await;
    run("K", "terminal.kill", &json!({"terminal":loose})).await;

    // S: a real selection.
    run("S", "pane.focus", &json!({"terminal":t1})).await;
    run("S", "terminal.select", &json!({"terminal":t1,"at":{"row":2,"col":0},"mode":"word"})).await;
    run("S", "terminal.adjust_selection", &json!({"terminal":t1,"direction":"right"})).await;
    run("S", "terminal.adjust_selection", &json!({"terminal":t1,"direction":"line_end"})).await;
    run("S", "terminal.select", &json!({"terminal":t1,"at":"cursor"})).await;
    run("S", "terminal.clear_selection", &json!({"terminal":t1})).await;

    // R: pane.resize and pane.swap {target} on two visible panes of one tab.
    let sp = run("R", "pane.split", &json!({"direction":"right","argv":["sh","-c","sleep 600"]})).await;
    let p2 = sp["pane"].as_str().unwrap_or("MISSING").to_string();
    let mut l = cmd("herdr-app");
    l.arg("ls");
    let ls: Value = serde_json::from_str(&raw(l).await.stdout).unwrap_or(Value::Null);
    let p1 = ls["spaces"][0]["tabs"][0]["terminals"]
        .as_array()
        .and_then(|ts| ts.iter().find(|t| t["terminal"] == t1.as_str()))
        .and_then(|t| t["pane"].as_str())
        .unwrap_or("MISSING")
        .to_string();
    println!("R panes: p1={p1} p2={p2}");
    run("R", "pane.resize", &json!({"direction":"left","pane":p2,"px":30})).await;
    run("R", "pane.resize", &json!({"direction":"right"})).await;
    run("R", "pane.swap", &json!({"pane":p2,"target":p1})).await;
    run("R", "pane.close", &json!({"pane":p2})).await;

    // X: "exactly one of" pairs, and an empty string where an id is expected.
    run("X", "space.select", &json!({})).await;
    run("X", "space.select", &json!({"index":1,"id":"x7"})).await;
    run("X", "tab.select", &json!({})).await;
    run("X", "tab.select", &json!({"index":1,"id":"x7"})).await;
    run("X", "tab.move", &json!({"index":1,"by":1})).await;
    run("X", "pane.focus", &json!({})).await;
    run("X", "pane.focus", &json!({"pane":"x7","terminal":t1})).await;
    run("X", "pane.swap", &json!({})).await;
    run("X", "pane.swap", &json!({"direction":"left","target":"x7"})).await;
    run("X", "terminal.scroll", &json!({"lines":1,"pages":1})).await;
    run("X", "layout.resize", &json!({"id":"sidebar"})).await;
    run("X", "layout.resize", &json!({"id":"sidebar","px":200,"weight":1})).await;
    run("X", "tab.new", &json!({"argv":["sh"],"mod":"files"})).await;
    run("X", "pane.split", &json!({"direction":"right","argv":["sh"],"mod":"files"})).await;
    run("X", "terminal.kill", &json!({"terminal":""})).await;
    run("X", "space.rename", &json!({"name":""})).await;
    run("X", "tab.rename", &json!({"name":""})).await;
    run("X", "layout.resize", &json!({"id":"sidebar","px":-5})).await;
    run("X", "layout.resize", &json!({"id":"sidebar","px":2.5})).await;
    run("X", "space.select", &json!({"index":0})).await;
    run("X", "space.move", &json!({"id":"x7","index":0})).await;
    run("X", "app.settings", &json!({"section":"x7"})).await;
    run("X", "layout.switch", &json!({"name":"x7"})).await;
    run("X", "terminal.open", &json!({"place":"x7"})).await;

    // Q: SIGTERM to the window process (the live harness's fallback when app.quit cannot answer).
    let pid = child.id();
    let mut kill = Command::new("kill");
    kill.args(["-TERM", &pid.unwrap_or(0).to_string()]);
    let o = raw(kill).await;
    println!("Q kill -TERM {pid:?} exit={:?}", o.code);
    let mut child = child;
    let waited = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
    println!("Q window wait after SIGTERM: {waited:?}");
    println!("Q status after SIGTERM: {:?}", status_code().await);
    hc("Z", &["server", "stop"]).await;
}
