//! Probe: every safe herdr-app command (run on 0.1.17), in isolated windows.
//! Phase T: wrong-typed and missing values for every argument (what herdr-app accepts).
//! Phase V: every safe command once with valid values (what it prints).
//! Every herdr-app call names an isolated home and an isolated hc session.
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

/// Never run: shared install, network, the person's other apps, the system clipboard, Herdr's
/// feedback flow.
const SKIP: [&str; 10] = [
    "app.update",
    "app.update_core",
    "app.check_updates",
    "app.hide",
    "app.hide_others",
    "app.show_all",
    "terminal.copy",
    "terminal.paste",
    "terminal.paste_image",
    "app.feedback",
];
/// Ends the window: only at the end of phase V.
const END: [&str; 2] = ["app.quit", "window.close"];

#[derive(Clone, Copy)]
struct Iso {
    home: &'static str,
    xdg: &'static str,
    session: &'static str,
}

impl Iso {
    fn cmd(self, program: &str) -> Command {
        let mut c = Command::new(program);
        for (k, _) in std::env::vars() {
            if k.starts_with("HC_") || k == "HERDR_APP_HOME" {
                c.env_remove(k);
            }
        }
        c.env("HERDR_APP_HOME", self.home)
            .env("HC_SESSION", self.session)
            .env("XDG_STATE_HOME", format!("{}/s", self.xdg))
            .env("XDG_CONFIG_HOME", format!("{}/c", self.xdg))
            .env("XDG_RUNTIME_DIR", format!("{}/r", self.xdg));
        c.kill_on_drop(true);
        c
    }

    fn reset_dirs(self) {
        for d in [self.home, self.xdg] {
            let _ = std::fs::remove_dir_all(d);
        }
        std::fs::create_dir_all(self.home).unwrap();
        for s in ["s", "c", "r"] {
            std::fs::create_dir_all(format!("{}/{s}", self.xdg)).unwrap();
        }
    }
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

/// `herdr-app do <name> <json>`; prints one line and returns the parsed stdout.
async fn run(iso: Iso, label: &str, name: &str, args: &Value) -> (Option<i32>, Value) {
    let mut c = iso.cmd("herdr-app");
    c.args(["do", name, &args.to_string()]);
    let o = raw(c).await;
    let err = if o.stderr.is_empty() { String::new() } else { format!(" stderr={:?}", o.stderr.trim_end()) };
    println!("{label} | do {name} {args} | exit={:?} | {}{err}", o.code, compact(&o.stdout));
    (o.code, serde_json::from_str(&o.stdout).unwrap_or(Value::Null))
}

async fn sub(iso: Iso, args: &[&str]) -> Value {
    let mut c = iso.cmd("herdr-app");
    c.args(args);
    serde_json::from_str(&raw(c).await.stdout).unwrap_or(Value::Null)
}

async fn launch(iso: Iso) -> Child {
    iso.reset_dirs();
    let mut c = iso.cmd("herdr-app");
    c.stdout(Stdio::null()).stderr(Stdio::null());
    let child = c.spawn().expect("launch");
    let start = Instant::now();
    loop {
        let mut s = iso.cmd("herdr-app");
        s.arg("status");
        if raw(s).await.code == Some(0) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(45), "window never answered");
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    println!("== window {} ready after {:?}", iso.home, start.elapsed());
    child
}

async fn shutdown(iso: Iso, mut child: Child) {
    let mut q = iso.cmd("herdr-app");
    q.args(["do", "app.quit"]);
    let _ = raw(q).await;
    let waited = tokio::time::timeout(Duration::from_secs(20), child.wait()).await;
    println!("== window {} quit: {waited:?}", iso.home);
    if waited.is_err() {
        let _ = child.kill().await;
    }
    let mut stop = iso.cmd("hc");
    stop.args(["server", "stop"]);
    let o = raw(stop).await;
    println!("== core {} stop: exit={:?} {}", iso.session, o.code, o.stdout.trim_end());
}

async fn person_core(label: &str) {
    let mut c = Command::new("hc");
    for (k, _) in std::env::vars() {
        if k.starts_with("HC_") {
            c.env_remove(k);
        }
    }
    c.args(["ls", "--json"]);
    let o = raw(c).await;
    println!(
        "== {label}: person's core `hc ls --json` exit={:?} bytes={} mentions_hzprobe={}",
        o.code,
        o.stdout.len(),
        o.stdout.contains("hzprobe")
    );
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("MISSING").to_string()
}

#[tokio::main]
async fn main() {
    let list: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../herdr-app-0.1.17-commands.json")).unwrap(),
    )
    .unwrap();
    let commands = list.as_array().unwrap();
    let mut ver = Command::new("herdr-app");
    ver.arg("--version");
    println!("== version: {}", raw(ver).await.stdout.trim_end());
    let pid = std::process::id();
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    person_core("B0 before").await;

    // ---------------- Phase T ----------------
    let t = Iso {
        home: leak(format!("/tmp/hfa{pid}-0")),
        xdg: leak(format!("/tmp/hfa{pid}-0/x")),
        session: leak(format!("hfa{pid}-0")),
    };
    let child = launch(t).await;
    run(t, "T0 setup", "space.new", &json!({"name":"hzprobe","argv":["sh","-c","sleep 600"]})).await;
    let wrong: [Value; 7] = [json!(7), json!(1.5), json!(-1), json!("x7"), json!(true), json!({"k":1}), json!(["x7"])];
    for c in commands {
        let name = c["name"].as_str().unwrap();
        if SKIP.contains(&name) || END.contains(&name) {
            println!("T skip {name}");
            continue;
        }
        let args = c["args"].as_array().unwrap();
        if args.iter().any(|a| a["required"] == true) {
            run(t, "T missing", name, &json!({})).await;
        }
        for a in args {
            let an = a["name"].as_str().unwrap();
            for w in &wrong {
                let mut o = serde_json::Map::new();
                o.insert(an.to_string(), w.clone());
                run(t, &format!("T {an}"), name, &Value::Object(o)).await;
            }
        }
    }
    shutdown(t, child).await;
    person_core("B1 between").await;

    // ---------------- Phase V ----------------
    let v = Iso {
        home: leak(format!("/tmp/hfa{pid}-1")),
        xdg: leak(format!("/tmp/hfa{pid}-1/x")),
        session: leak(format!("hfa{pid}-1")),
    };
    let child = launch(v).await;
    let (_, made) = run(v, "V", "space.new", &json!({"name":"hzprobe","argv":["sh","-c","sleep 600"],"tab":"first"})).await;
    let (s1, t1) = (s(&made, "space"), s(&made, "terminal"));
    let (_, made2) = run(v, "V", "space.new", &json!({"name":"s2","focus":false})).await;
    let s2 = s(&made2, "space");
    run(v, "V", "space.rename", &json!({"id":s2,"name":"s2b"})).await;
    run(v, "V", "space.move", &json!({"id":s2,"index":1})).await;
    run(v, "V", "space.select", &json!({"index":2})).await;
    run(v, "V", "space.select", &json!({"id":s1})).await;
    run(v, "V", "space.next", &json!({})).await;
    run(v, "V", "space.prev", &json!({})).await;
    run(v, "V", "space.select", &json!({"id":s1})).await;
    let (_, tb) = run(v, "V", "tab.new", &json!({"name":"tb","space":s1,"argv":["sh","-c","sleep 600"]})).await;
    let tb_id = s(&tb, "tab");
    run(v, "V", "tab.new", &json!({"name":"modtab","mod":"files","focus":false})).await;
    run(v, "V", "tab.rename", &json!({"id":tb_id,"name":"tbb"})).await;
    run(v, "V", "tab.move", &json!({"id":tb_id,"index":1})).await;
    run(v, "V", "tab.move", &json!({"by":1})).await;
    run(v, "V", "tab.select", &json!({"index":1})).await;
    run(v, "V", "tab.select", &json!({"id":tb_id})).await;
    run(v, "V", "tab.next", &json!({})).await;
    run(v, "V", "tab.prev", &json!({})).await;
    run(v, "V", "tab.settle", &json!({"id":tb_id})).await;
    run(v, "V", "tab.unsettle", &json!({"id":tb_id})).await;
    run(v, "V", "tab.select", &json!({"id":tb_id})).await;
    let (_, sp) = run(v, "V", "pane.split", &json!({"direction":"right","argv":["sh","-c","sleep 600"]})).await;
    let (p2, t2) = (s(&sp, "pane"), s(&sp, "terminal"));
    run(v, "V", "pane.split", &json!({"direction":"down","mod":"files"})).await;
    run(v, "V", "pane.zoom", &json!({"pane":p2})).await;
    run(v, "V", "pane.zoom", &json!({"pane":p2})).await;
    run(v, "V", "pane.focus", &json!({"terminal":t2})).await;
    run(v, "V", "pane.focus", &json!({"pane":p2})).await;
    run(v, "V", "pane.go", &json!({"direction":"left"})).await;
    run(v, "V", "pane.swap", &json!({"direction":"right"})).await;
    run(v, "V", "pane.swap", &json!({"target":p2})).await;
    run(v, "V", "pane.rename", &json!({"pane":p2,"name":"renamed"})).await;
    run(v, "V", "pane.rename", &json!({"pane":p2,"name":""})).await;
    run(v, "V", "pane.resize", &json!({"direction":"left","pane":p2,"px":30})).await;
    run(v, "V", "pane.resize", &json!({"direction":"right"})).await;
    run(v, "V", "terminal.open", &json!({"argv":["sh","-c","sleep 600"],"place":"down"})).await;
    run(v, "V", "terminal.open", &json!({"argv":["sh","-c","sleep 600"],"place":"right","name":"opened"})).await;
    run(v, "V", "terminal.open", &json!({"argv":["sh","-c","sleep 600"]})).await;
    run(v, "V", "terminal.peek", &json!({"terminal":t1})).await;
    run(v, "V", "overlay.close", &json!({})).await;
    run(v, "V", "terminal.show", &json!({"terminal":t1})).await;
    run(v, "V", "terminal.show", &json!({})).await;
    run(v, "V", "pane.focus", &json!({"terminal":t2})).await;
    run(v, "V", "terminal.select_all", &json!({})).await;
    run(v, "V", "terminal.select", &json!({"mode":"line"})).await;
    run(v, "V", "terminal.select", &json!({"at":{"row":0,"col":0},"mode":"word"})).await;
    run(v, "V", "terminal.adjust_selection", &json!({"direction":"right"})).await;
    run(v, "V", "terminal.clear_selection", &json!({})).await;
    run(v, "V", "terminal.scroll", &json!({"lines":-1})).await;
    run(v, "V", "terminal.scroll", &json!({"pages":0.5})).await;
    run(v, "V", "terminal.scroll", &json!({"to":"bottom"})).await;
    run(v, "V", "terminal.search", &json!({})).await;
    run(v, "V", "terminal.right_click", &json!({"to":"program"})).await;
    run(v, "V", "terminal.right_click", &json!({"to":"menu","terminal":t2})).await;
    run(v, "V", "terminal.pin", &json!({"terminal":t1,"pinned":true})).await;
    run(v, "V", "terminal.pin", &json!({"terminal":t1,"pinned":false})).await;
    // A terminal with no pane, started straight in the isolated core.
    let mut hr = v.cmd("hc");
    hr.args(["run", "--json", "--", "sh", "-c", "sleep 600"]);
    let o = raw(hr).await;
    println!("V hc run --json | exit={:?} | {}", o.code, compact(&o.stdout));
    let hv: Value = serde_json::from_str(&o.stdout).unwrap_or(Value::Null);
    let loose = hv["terminal"]["address"].as_str().or_else(|| hv["address"].as_str()).unwrap_or("MISSING").to_string();
    run(v, "V", "terminal.keep", &json!({"terminal":loose,"place":"tab"})).await;
    run(v, "V", "terminal.park", &json!({"terminal":loose})).await;
    run(v, "V", "terminal.kill", &json!({"terminal":loose})).await;
    run(v, "V", "agent.focus", &json!({"terminal":t1})).await;
    run(v, "V", "agent.next", &json!({})).await;
    run(v, "V", "agent.prev", &json!({})).await;
    run(v, "V", "layout.switch", &json!({"name":"terminals"})).await;
    run(v, "V", "layout.switch", &json!({})).await;
    run(v, "V", "layout.switch", &json!({"name":"main"})).await;
    run(v, "V", "layout.toggle", &json!({"id":"sidebar"})).await;
    run(v, "V", "layout.show", &json!({"id":"sidebar"})).await;
    run(v, "V", "layout.hide", &json!({"id":"panel"})).await;
    run(v, "V", "layout.resize", &json!({"id":"sidebar","px":200})).await;
    run(v, "V", "layout.resize", &json!({"id":"sidebar","weight":1})).await;
    run(v, "V", "layout.select", &json!({"stack":"panel-body","child":"changes"})).await;
    run(v, "V", "layout.clone", &json!({"name":"main","as":"probe-copy"})).await;
    let tree: Value = std::fs::read_to_string(format!("{}/layouts/probe-copy.json", v.home))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);
    println!("V probe-copy.json top-level keys: {:?}", tree.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));
    run(v, "V", "layout.set", &json!({"layout":tree})).await;
    run(v, "V", "layout.reset", &json!({"name":"main"})).await;
    run(v, "V", "layout.switch", &json!({"name":"main"})).await;
    run(v, "V", "mod.clone", &json!({"name":"files","as":"files-probe"})).await;
    run(v, "V", "mod.open", &json!({"name":"picker"})).await;
    run(v, "V", "mod.open", &json!({"name":"picker"})).await;
    for n in ["app.help", "app.help", "app.palette", "app.palette", "app.welcome"] {
        run(v, "V", n, &json!({})).await;
    }
    run(v, "V", "overlay.close", &json!({})).await;
    run(v, "V", "app.whats_new", &json!({})).await;
    run(v, "V", "overlay.close", &json!({})).await;
    run(v, "V", "app.settings", &json!({"section":"look"})).await;
    run(v, "V", "overlay.close", &json!({})).await;
    run(v, "V", "app.theme", &json!({"name":"atom-one-dark"})).await;
    for n in ["app.zoom_in", "app.zoom_out", "app.zoom_reset"] {
        run(v, "V", n, &json!({})).await;
    }
    run(v, "V", "pane.close", &json!({"pane":p2})).await;
    run(v, "V", "tab.close", &json!({"id":tb_id,"confirmed":true})).await;
    run(v, "V", "space.close", &json!({"id":s2,"confirmed":true})).await;
    run(v, "V", "app.reset", &json!({})).await;
    println!("V status | {}", sub(v, &["status"]).await);
    person_core("B2 during V").await;
    run(v, "V", "window.minimize", &json!({})).await;
    run(v, "V", "window.maximize", &json!({})).await;
    run(v, "V", "window.maximize", &json!({})).await;
    run(v, "V", "window.close", &json!({})).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut st = v.cmd("herdr-app");
    st.arg("status");
    let o = raw(st).await;
    println!("V status after window.close | exit={:?} | {} {}", o.code, compact(&o.stdout), o.stderr.trim_end());
    run(v, "V", "app.quit", &json!({})).await;
    shutdown(v, child).await;
    person_core("B3 after").await;
}
