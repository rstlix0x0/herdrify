//! Probe: an isolated herdr-app 0.1.16 window (own HERDR_APP_HOME, own hc session and XDG dirs).
//! Every herdr-app call names the isolated home; the person's core is only read.
use std::time::{Duration, Instant};
use tokio::process::Command;

const HOME: &str = "/tmp/hza";
const XDG: &str = "/tmp/hzx";
const SESSION: &str = "hzp";
const MARK: &str = "hzprobe";

struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// A command with every inherited HC_* variable removed.
fn clean(program: &str) -> Command {
    let mut c = Command::new(program);
    for (k, _) in std::env::vars() {
        if k.starts_with("HC_") || k == "HERDR_APP_HOME" {
            c.env_remove(k);
        }
    }
    c
}

/// Isolated: own app home, own core session and XDG dirs.
fn isolated(program: &str) -> Command {
    let mut c = clean(program);
    c.env("HERDR_APP_HOME", HOME)
        .env("HC_SESSION", SESSION)
        .env("XDG_STATE_HOME", format!("{XDG}/s"))
        .env("XDG_CONFIG_HOME", format!("{XDG}/c"))
        .env("XDG_RUNTIME_DIR", format!("{XDG}/r"));
    c
}

async fn out(mut c: Command) -> Out {
    let o = c.output().await.expect("spawn");
    Out {
        code: o.status.code(),
        stdout: String::from_utf8_lossy(&o.stdout).into(),
        stderr: String::from_utf8_lossy(&o.stderr).into(),
    }
}

async fn app(label: &str, args: &[&str]) -> Out {
    let mut c = isolated("herdr-app");
    c.args(args);
    let o = out(c).await;
    let argv: Vec<String> = args.iter().map(|a| format!("{a:?}")).collect();
    println!("== {label}\n$ herdr-app {}", argv.join(" "));
    println!("exit={:?}\nstdout={}\nstderr={:?}\n", o.code, o.stdout.trim_end(), o.stderr);
    o
}

/// The person's default core, read only: does any terminal mention MARK?
async fn person_core_mentions_mark(label: &str) {
    let mut c = clean("hc");
    c.args(["ls", "--json"]);
    let o = out(c).await;
    println!(
        "== {label}: person's core `hc ls --json` exit={:?} bytes={} mentions_{MARK}={}\n",
        o.code,
        o.stdout.len(),
        o.stdout.contains(MARK)
    );
}

async fn isolated_core(label: &str) {
    let mut c = isolated("hc");
    c.args(["ls", "--json"]);
    let o = out(c).await;
    println!(
        "== {label}: isolated core `hc ls --json` exit={:?} mentions_{MARK}={} stdout_bytes={} stderr={:?}\n",
        o.code,
        o.stdout.contains(MARK),
        o.stdout.len(),
        o.stderr
    );
}

fn listing(dir: &str, depth: usize, prefix: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut names: Vec<_> = rd.map(|e| e.unwrap()).collect();
    names.sort_by_key(std::fs::DirEntry::file_name);
    for e in names {
        let name = e.file_name().to_string_lossy().into_owned();
        let is_dir = e.file_type().unwrap().is_dir();
        println!("{prefix}{name}{}", if is_dir { "/" } else { "" });
        if is_dir && depth > 0 {
            listing(&format!("{dir}/{name}"), depth - 1, &format!("{prefix}  "));
        }
    }
}

#[tokio::main]
async fn main() {
    for d in [HOME, XDG] {
        let _ = std::fs::remove_dir_all(d);
    }
    std::fs::create_dir_all(HOME).unwrap();
    for s in ["s", "c", "r"] {
        std::fs::create_dir_all(format!("{XDG}/{s}")).unwrap();
    }
    let mut ver = clean("herdr-app");
    ver.arg("--version");
    println!("== version: {}", out(ver).await.stdout.trim_end());
    person_core_mentions_mark("B0 before").await;

    // L: launch the isolated window and wait for its socket to answer `status`.
    let mut launch = isolated("herdr-app");
    launch.kill_on_drop(true);
    let mut child = launch.spawn().expect("launch herdr-app");
    println!("== L1 launched `herdr-app` (no args), child pid={:?}\n", child.id());
    let start = Instant::now();
    let mut ready = false;
    while start.elapsed() < Duration::from_secs(45) {
        if let Ok(Some(st)) = child.try_wait() {
            println!("== L2 launcher exited early: {st:?} after {:?}\n", start.elapsed());
        }
        let mut c = isolated("herdr-app");
        c.arg("status");
        if out(c).await.code == Some(0) {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    println!("== L3 isolated window answering status: {ready} after {:?}\n", start.elapsed());
    if !ready {
        let _ = child.kill().await;
        listing(HOME, 1, "  ");
        return;
    }

    app("S1 status", &["status"]).await;
    app("S2 ls", &["ls"]).await;

    let spawn = format!(r#"{{"name":"{MARK}","argv":["sh","-c","echo {MARK}; sleep 120"]}}"#);
    app("D1 space.new (JSON object)", &["do", "space.new", &spawn]).await;
    app("D2 tab.new (JSON object)", &["do", "tab.new", r#"{"name":"t2"}"#]).await;
    app("D3 tab.new (flags)", &["do", "tab.new", "--name", "flagtab", "--focus", "false"]).await;
    app("D4 tab.next (no args)", &["do", "tab.next"]).await;
    app("D5 tab.next (empty object)", &["do", "tab.next", "{}"]).await;
    app("D6 layout.toggle", &["do", "layout.toggle", r#"{"id":"sidebar"}"#]).await;
    app("D7 layout.toggle back", &["do", "layout.toggle", r#"{"id":"sidebar"}"#]).await;
    app("D8 pane.split shorthand", &["do", "pane.split:right"]).await;
    app("E1 unknown command", &["do", "no.such", "{}"]).await;
    app("E2 unknown argument", &["do", "tab.new", r#"{"bogus":1}"#]).await;
    app("E3 wrong type (name: number)", &["do", "tab.new", r#"{"name":5}"#]).await;
    app("E4 wrong type (focus: string)", &["do", "tab.new", r#"{"focus":"no"}"#]).await;
    app("E5 missing required (space.rename name)", &["do", "space.rename", "{}"]).await;
    app("E6 target id not a string", &["do", "pane.focus", r#"{"pane":7}"#]).await;
    app("E7 target id unknown", &["do", "pane.focus", r#"{"pane":"p-nope"}"#]).await;
    app("E8 args JSON not an object", &["do", "tab.new", "[1]"]).await;
    app("E9 null means omitted", &["do", "tab.new", r#"{"name":null,"focus":false}"#]).await;
    app("S3 ls after", &["ls"]).await;
    app("S4 status after", &["status"]).await;

    isolated_core("I1 during").await;
    person_core_mentions_mark("B1 during").await;

    // Q: quit the isolated window.
    app("Q1 app.quit", &["do", "app.quit"]).await;
    let waited = tokio::time::timeout(Duration::from_secs(20), child.wait()).await;
    println!("== Q2 launcher wait after quit: {waited:?}\n");
    if waited.is_err() {
        let _ = child.kill().await;
        println!("== Q3 killed launcher\n");
    }
    app("Q4 status after quit", &["status"]).await;

    person_core_mentions_mark("B2 after").await;
    let mut stop = isolated("hc");
    stop.args(["server", "stop"]);
    let o = out(stop).await;
    println!("== Z1 isolated `hc server stop` exit={:?} stdout={:?} stderr={:?}\n", o.code, o.stdout, o.stderr);

    println!("== F files the isolated window wrote in {HOME}:");
    listing(HOME, 1, "  ");
}
