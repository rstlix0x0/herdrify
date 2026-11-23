//! Edge inputs against an isolated hc core in plain mode: one request per line, print the reply.
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::main]
async fn main() {
    let dir = std::env::args().nth(1).expect("scratch dir");
    let mut cmd = Command::new("hc");
    cmd.arg("connect");
    for (k, _) in std::env::vars() { if k.starts_with("HC_") { cmd.env_remove(k); } }
    cmd.env("HC_SESSION", "hfy-edge")
        .env("XDG_STATE_HOME", format!("{dir}/state"))
        .env("XDG_CONFIG_HOME", format!("{dir}/config"))
        .env("XDG_RUNTIME_DIR", format!("{dir}/run"))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).kill_on_drop(true);
    let mut child = cmd.spawn().expect("spawn hc connect");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
    let reqs: &[(&str, &str)] = &[
        ("E1 hello", r#"{"method":"hello","params":{"generation":1}}"#),
        ("E2 terminal.run neither argv nor line", r#"{"method":"terminal.run","params":{}}"#),
        ("E3 terminal.run both argv and line", r#"{"method":"terminal.run","params":{"argv":["true"],"line":"true"}}"#),
        ("E4 terminal.run argv empty", r#"{"method":"terminal.run","params":{"argv":[]}}"#),
        ("E5 terminal.run line empty", r#"{"method":"terminal.run","params":{"line":""}}"#),
        ("E6 machine.exec neither", r#"{"method":"machine.exec","params":{}}"#),
        ("E7 machine.exec both", r#"{"method":"machine.exec","params":{"argv":["true"],"line":"true"}}"#),
        ("E8 terminal.get terminal empty", r#"{"method":"terminal.get","params":{"terminal":""}}"#),
        ("E9 terminal.get terminal whitespace", r#"{"method":"terminal.get","params":{"terminal":"  "}}"#),
        ("E10 doc.get name empty", r#"{"method":"doc.get","params":{"name":""}}"#),
        ("E11 doc.get name with space", r#"{"method":"doc.get","params":{"name":"a b"}}"#),
        ("E12 terminal.run then label with empty key", r#"{"method":"terminal.run","params":{"argv":["sh"],"name":"edge-sh"}}"#),
        ("E13 terminal.label empty key", r#"{"method":"terminal.label","params":{"terminal":"edge-sh","labels":{"":"x"}}}"#),
        ("E14 terminal.label key with =", r#"{"method":"terminal.label","params":{"terminal":"edge-sh","labels":{"a=b":"x"}}}"#),
        ("E15 terminal.run name empty", r#"{"method":"terminal.run","params":{"argv":["true"],"name":""}}"#),
        ("E16 terminal.run duplicate name", r#"{"method":"terminal.run","params":{"argv":["sh"],"name":"edge-sh"}}"#),
        ("E17 terminal.keys empty list", r#"{"method":"terminal.keys","params":{"terminal":"edge-sh","keys":[]}}"#),
        ("E18 terminal.send empty text", r#"{"method":"terminal.send","params":{"terminal":"edge-sh","text":""}}"#),
        ("E19 file.read path empty", r#"{"method":"file.read","params":{"path":""}}"#),
        ("E20 file.read relative path no cwd", r#"{"method":"file.read","params":{"path":"rel.txt"}}"#),
        ("E21 terminal.kill edge-sh", r#"{"method":"terminal.kill","params":{"terminal":"edge-sh"}}"#),
        ("E22 terminal.run size 0x0", r#"{"method":"terminal.run","params":{"argv":["true"],"size":{"cols":0,"rows":0}}}"#),
        ("E23 terminal.wait no terminal no children", r#"{"method":"terminal.wait","params":{}}"#),
        ("E24 doc.set value null version 0", r#"{"method":"doc.set","params":{"name":"edgedoc","value":null,"version":0}}"#),
        ("E25 server.stop", r#"{"method":"server.stop","params":{}}"#),
    ];
    for (i, (label, body)) in reqs.iter().enumerate() {
        let mut v: serde_json::Value = serde_json::from_str(body).expect("probe json");
        v["jsonrpc"] = "2.0".into();
        v["id"] = (i as u64 + 1).into();
        let line = v.to_string();
        println!("== {label}\n-> {line}");
        stdin.write_all(format!("{line}\n").as_bytes()).await.expect("write");
        match tokio::time::timeout(std::time::Duration::from_secs(15), lines.next_line()).await {
            Ok(Ok(Some(reply))) => {
                let shown = if reply.len() > 600 { format!("{}…[{} bytes]", &reply[..600], reply.len()) } else { reply };
                println!("<- {shown}");
            }
            other => println!("<- (no reply: {other:?})"),
        }
    }
    drop(stdin);
    println!("hc connect exit: {:?}", child.wait().await);
}
