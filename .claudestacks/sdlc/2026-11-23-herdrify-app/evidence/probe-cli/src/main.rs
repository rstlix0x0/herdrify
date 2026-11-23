//! Probe: herdr-app 0.1.16 CLI behaviour that needs no open window.
use std::collections::BTreeSet;
use tokio::process::Command;

struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

async fn run(home: Option<&str>, args: &[&str]) -> Out {
    let mut c = Command::new("herdr-app");
    c.args(args);
    if let Some(h) = home {
        c.env("HERDR_APP_HOME", h);
    }
    let o = c.output().await.expect("spawn herdr-app");
    Out {
        code: o.status.code(),
        stdout: String::from_utf8_lossy(&o.stdout).into(),
        stderr: String::from_utf8_lossy(&o.stderr).into(),
    }
}

fn show(label: &str, home: Option<&str>, args: &[&str], o: &Out) {
    let env = home.map(|h| format!("HERDR_APP_HOME={h} ")).unwrap_or_default();
    let argv: Vec<String> = args.iter().map(|a| format!("{a:?}")).collect();
    println!("== {label}\n$ {env}herdr-app {}", argv.join(" "));
    println!("exit={:?}\nstdout={:?}\nstderr={:?}\n", o.code, o.stdout, o.stderr);
}

#[tokio::main]
async fn main() {
    let short = "/tmp/hzp";
    let long = "/tmp/hzp-a-very-long-directory-name-that-pushes-the-socket-path-over-the-limit-for-unix-sockets";
    for d in [short, long] {
        let _ = std::fs::remove_dir_all(d);
        std::fs::create_dir_all(d).unwrap();
    }

    // V: version
    let o = run(None, &["--version"]).await;
    show("V1 version", None, &["--version"], &o);

    // C: commands without a window, and equality with the person's home output
    let a = run(Some(short), &["commands"]).await;
    let b = run(None, &["commands"]).await;
    println!(
        "== C1 commands with empty home\nexit={:?} stderr={:?} stdout_bytes={}",
        a.code,
        a.stderr,
        a.stdout.len()
    );
    let v: serde_json::Value = serde_json::from_str(&a.stdout).expect("commands is JSON");
    let arr = v.as_array().expect("array");
    let mut keys = BTreeSet::new();
    let mut argkeys = BTreeSet::new();
    let (mut req, mut nargs) = (0, 0);
    for c in arr {
        for k in c.as_object().unwrap().keys() {
            keys.insert(k.clone());
        }
        for a in c["args"].as_array().unwrap() {
            nargs += 1;
            if a["required"] == true {
                req += 1;
            }
            for k in a.as_object().unwrap().keys() {
                argkeys.insert(k.clone());
            }
        }
    }
    println!(
        "commands={} command_keys={keys:?} arg_keys={argkeys:?} args_total={nargs} args_required={req}",
        arr.len()
    );
    let names: BTreeSet<&str> = arr.iter().map(|c| c["name"].as_str().unwrap()).collect();
    println!("unique_names={}", names.len());
    println!("C2 equal to output with the person's home: {}", a.stdout == b.stdout);
    let saved = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../herdr-app-0.1.17-commands.json")).unwrap();
    println!("C3 equal to saved herdr-app-0.1.17-commands.json: {}\n", saved == a.stdout);

    // N: window-needing subcommands with no window
    let cases: [&[&str]; 7] = [
        &["ls"],
        &["status"],
        &["do", "tab.next"],
        &["do", "tab.next", "{}"],
        &["do", "no.such", "{}"],
        &["do", "tab.new", "{not json"],
        &["do"],
    ];
    for args in cases {
        let o = run(Some(short), args).await;
        show("N no window (short home)", Some(short), args, &o);
    }
    let o = run(Some(long), &["status"]).await;
    show("N long home", Some(long), &["status"], &o);

    // U: usage errors
    let usage: [&[&str]; 3] = [&["bogus"], &["ls", "extra"], &["--nope"]];
    for args in usage {
        let o = run(Some(short), args).await;
        show("U usage", Some(short), args, &o);
    }

    // W: did any of these write into the empty home?
    let listing: Vec<_> = std::fs::read_dir(short)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    println!("== W files in {short} after all runs: {listing:?}");
}
