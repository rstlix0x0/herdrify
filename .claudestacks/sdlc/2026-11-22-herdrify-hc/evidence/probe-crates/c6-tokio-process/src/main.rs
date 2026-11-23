//! C6 probe: tokio::process behaviour the hc transport relies on.
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

fn probe_pid(label: &str, pid: u32) {
    let k = std::process::Command::new("kill").args(["-0", &pid.to_string()]).output().unwrap();
    let ps = std::process::Command::new("ps").args(["-o", "pid=,stat=,comm=", "-p", &pid.to_string()]).output().unwrap();
    println!(
        "  {label}: kill -0 {pid} -> exit {:?} stderr={:?}; ps -> exit {:?} out={:?}",
        k.status.code(),
        String::from_utf8_lossy(&k.stderr).trim(),
        ps.status.code(),
        String::from_utf8_lossy(&ps.stdout).trim()
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("(a) spawn a program that does not exist");
    for prog in ["herdrify-no-such-program-c6", "/nonexistent/dir/hc"] {
        match Command::new(prog).spawn() {
            Ok(_) => println!("  {prog}: unexpectedly spawned"),
            Err(e) => println!(
                "  {prog}: kind={:?} raw_os_error={:?} Display=\"{e}\" Debug={e:?}",
                e.kind(),
                e.raw_os_error()
            ),
        }
    }

    println!("(b) kill_on_drop(true) on `sleep 30`, then drop");
    let child = Command::new("sleep").arg("30").kill_on_drop(true).spawn().unwrap();
    let pid = child.id().unwrap();
    probe_pid("before drop", pid);
    drop(child);
    probe_pid("immediately after drop", pid);
    tokio::time::sleep(Duration::from_millis(200)).await;
    probe_pid("after 200ms", pid);
    // another child operation drives tokio's orphan reaping
    let _ = Command::new("true").status().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    probe_pid("after spawning+awaiting another child and 200ms more", pid);

    println!("(b2) control: kill_on_drop(false) (default) on `sleep 30`, then drop");
    let child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid2 = child.id().unwrap();
    drop(child);
    tokio::time::sleep(Duration::from_millis(200)).await;
    probe_pid("after drop + 200ms", pid2);
    let _ = std::process::Command::new("kill").arg(pid2.to_string()).status();

    println!("(c) read stdout lines after the child has exited");
    let mut child = Command::new("printf").arg("a\\nb\\n").stdout(Stdio::piped()).spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    let status = child.wait().await.unwrap();
    println!("  child exited: {status}");
    let mut lines = BufReader::new(stdout).lines();
    for i in 0..4 {
        println!("  next_line #{i}: {:?}", lines.next_line().await);
    }

    println!("(d) write to stdin of an exited child");
    let mut child = Command::new("true").stdin(Stdio::piped()).spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let status = child.wait().await.unwrap();
    println!("  child exited: {status}");
    let w = stdin.write_all(b"{\"jsonrpc\":\"2.0\"}\n").await;
    match &w {
        Ok(()) => println!("  write_all: Ok(())"),
        Err(e) => println!("  write_all: kind={:?} raw_os_error={:?} Display=\"{e}\"", e.kind(), e.raw_os_error()),
    }
    match stdin.flush().await {
        Ok(()) => println!("  flush: Ok(())"),
        Err(e) => println!("  flush: kind={:?} Display=\"{e}\"", e.kind()),
    }
}
