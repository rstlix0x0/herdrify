//! Q: what does a successful `machine.exec` reply look like on hc 0.1.21?
//! Runs against an isolated core: own HC_SESSION, scratch XDG dirs under a short /tmp dir, every HC_* removed.
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn isolated(cmd: &mut Command, d: &str) {
    for (k, _) in std::env::vars() {
        if k.starts_with("HC_") {
            cmd.env_remove(&k);
        }
    }
    cmd.env("HC_SESSION", "hfy-exec")
        .env("XDG_STATE_HOME", format!("{d}/state"))
        .env("XDG_CONFIG_HOME", format!("{d}/config"))
        .env("XDG_RUNTIME_DIR", format!("{d}/run"));
}

fn main() {
    let d = format!("/tmp/hfyx{}", std::process::id());
    std::fs::create_dir_all(&d).unwrap();
    let v = Command::new("hc").arg("--version").output().unwrap();
    println!("hc --version: {}", String::from_utf8_lossy(&v.stdout).trim());
    println!("serde_json 1.0.154; isolated dir {d}, HC_SESSION=hfy-exec");
    let mut c = Command::new("hc");
    c.arg("connect").stdin(Stdio::piped()).stdout(Stdio::piped());
    isolated(&mut c, &d);
    let mut child = c.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let reqs = [
        serde_json::json!({"jsonrpc":"2.0","id":0,"method":"hello","params":{"generation":1}}),
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"machine.exec","params":{"argv":["echo","hi"]}}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"machine.exec","params":{"line":"printf 'a\\nb'; echo err >&2; exit 3"}}),
    ];
    for r in reqs {
        let line = serde_json::to_string(&r).unwrap();
        println!("SEND {line}");
        writeln!(stdin, "{line}").unwrap();
        let mut reply = String::new();
        out.read_line(&mut reply).unwrap();
        let reply = reply.trim_end();
        if r["method"] == "hello" {
            let v: serde_json::Value = serde_json::from_str(reply).unwrap();
            println!("RECV hello version={}", v["result"]["version"]);
        } else {
            println!("RECV {reply}");
        }
    }
    drop(stdin);
    println!("hc connect exit: {:?}", child.wait().unwrap().code());
    let mut stop = Command::new("hc");
    stop.args(["server", "stop"]);
    isolated(&mut stop, &d);
    let s = stop.output().unwrap();
    println!("hc server stop (HC_SESSION=hfy-exec) exit: {:?}", s.status.code());
    std::fs::remove_dir_all(&d).unwrap();
}
