use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut child = Command::new("cat").stdin(Stdio::piped()).stdout(Stdio::null()).kill_on_drop(true).spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"x\n").await.unwrap();
    stdin.shutdown().await.unwrap();
    let r = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
    println!("after shutdown() with stdin kept alive: cat exited = {}", r.is_ok());
    drop(stdin);
    let r = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
    println!("after drop(stdin): cat exited = {:?}", r.map(|s| s.unwrap().code()));
}
