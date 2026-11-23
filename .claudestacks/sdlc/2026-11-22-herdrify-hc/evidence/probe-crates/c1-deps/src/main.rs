//! C1 probe: the dependency set builds on the pinned toolchain.
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
enum E {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Serialize, Deserialize)]
struct Msg {
    id: u32,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), E> {
    let b = bytes::Bytes::from_static(b"{\"id\":1}");
    let m: Msg = serde_json::from_slice(&b).map_err(std::io::Error::other)?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<u32>(1);
    tx.send(m.id).await.map_err(std::io::Error::other)?;
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    let out = tokio::process::Command::new("true").output().await?;
    println!("got {:?} status {}", rx.recv().await, out.status);
    Ok(())
}
