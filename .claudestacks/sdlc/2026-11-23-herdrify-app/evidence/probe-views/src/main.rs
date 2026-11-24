//! Probe: the `ls` and `status` fields of herdr-app 0.1.16 not yet seen filled, in one isolated
//! window: status.errors, status.floats, ls.recent, ls sub_agents.
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

const HOME: &str = "/tmp/hzw";
const XDG: &str = "/tmp/hzwx";

fn cmd(program: &str) -> Command {
    let mut c = Command::new(program);
    for (k, _) in std::env::vars() {
        if k.starts_with("HC_") || k == "HERDR_APP_HOME" {
            c.env_remove(k);
        }
    }
    c.env("HERDR_APP_HOME", HOME)
        .env("HC_SESSION", "hzw")
        .env("XDG_STATE_HOME", format!("{XDG}/s"))
        .env("XDG_CONFIG_HOME", format!("{XDG}/c"))
        .env("XDG_RUNTIME_DIR", format!("{XDG}/r"));
    c.kill_on_drop(true);
    c
}

async fn out(mut c: Command) -> (Option<i32>, String, String) {
    let o = tokio::time::timeout(Duration::from_secs(10), c.output()).await.expect("timeout").expect("spawn");
    (o.status.code(), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

async fn sub(label: &str, args: &[&str]) -> Value {
    let mut c = cmd("herdr-app");
    c.args(args);
    let (code, stdout, stderr) = out(c).await;
    println!("== {label} | herdr-app {} | exit={code:?} stderr={stderr:?}\n{}\n", args.join(" "), stdout.trim_end());
    serde_json::from_str(&stdout).unwrap_or(Value::Null)
}

async fn run(name: &str, args: &Value) -> Value {
    let mut c = cmd("herdr-app");
    c.args(["do", name, &args.to_string()]);
    let (code, stdout, _) = out(c).await;
    let v: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    println!("do {name} {args} | exit={code:?} | {v}");
    v
}

#[tokio::main]
async fn main() {
    let mut ver = cmd("herdr-app");
    ver.arg("--version");
    println!("== version: {}\n", out(ver).await.1.trim_end());
    for d in [HOME, XDG] {
        let _ = std::fs::remove_dir_all(d);
    }
    std::fs::create_dir_all(HOME).unwrap();
    for s in ["s", "c", "r"] {
        std::fs::create_dir_all(format!("{XDG}/{s}")).unwrap();
    }
    let mut c = cmd("herdr-app");
    c.stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = c.spawn().expect("launch");
    let start = Instant::now();
    loop {
        let mut s = cmd("herdr-app");
        s.arg("status");
        if out(s).await.0 == Some(0) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(45));
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    println!("== window ready after {:?}\n", start.elapsed());

    let made = run("space.new", &json!({"name":"hzprobe","argv":["sh","-c","sleep 600"]})).await;
    let t1 = made["terminal"].as_str().unwrap_or("t1").to_string();

    // sub_agents: a terminal whose parent is t1.
    let mut h = cmd("hc");
    h.args(["run", "--bg", "--json", "--parent", &t1, "--", "sh", "-c", "sleep 600"]);
    let (code, stdout, stderr) = out(h).await;
    println!("hc run --parent {t1} | exit={code:?} | {} {}", stdout.trim_end(), stderr.trim_end());

    // recent: a space made and closed.
    let s2 = run("space.new", &json!({"name":"gone","focus":false})).await;
    run("space.close", &json!({"id":s2["space"],"confirmed":true})).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    sub("LS1 sub_agents and recent", &["ls"]).await;

    // errors: a broken layout file and a broken mod.
    std::fs::write(format!("{HOME}/layouts/bad.json"), "{not json").unwrap();
    std::fs::create_dir_all(format!("{HOME}/mods/broken")).unwrap();
    std::fs::write(format!("{HOME}/mods/broken/main.js"), "export default class (").unwrap();
    std::fs::write(format!("{HOME}/mods/broken/mod.json"), r#"{"place":"right","size":200}"#).unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    sub("ST1 errors after broken files", &["status"]).await;
    run("mod.open", &json!({"name":"broken"})).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    sub("ST2 errors after opening the broken mod", &["status"]).await;

    // floats: the shipped win98 layout has a floating block.
    run("layout.switch", &json!({"name":"win98"})).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    sub("ST3 floats on win98", &["status"]).await;
    run("terminal.peek", &json!({"terminal":t1})).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    sub("ST4 overlays while peeking", &["status"]).await;

    run("app.quit", &json!({})).await;
    let waited = tokio::time::timeout(Duration::from_secs(20), child.wait()).await;
    println!("== quit: {waited:?}");
    if waited.is_err() {
        let _ = child.kill().await;
    }
    let mut stop = cmd("hc");
    stop.args(["server", "stop"]);
    println!("== core stop: {:?}", out(stop).await.1.trim_end());
}
