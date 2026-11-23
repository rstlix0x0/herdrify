//! The ways in through `HerdrCore`. Each returns the same entrypoint, whatever carries it.

use std::process::Stdio;

use herdrify_hc::{HerdrCore, HerdrCoreError, StreamTransport};
use tokio::process::{ChildStdin, ChildStdout, Command};

use super::BoxError;

/// The core that `hc connect` reaches: the default session, or the one `HC_SESSION` names.
///
/// # Errors
///
/// When `hc` cannot be started, or the core does not answer the handshake.
pub async fn local() -> Result<HerdrCore, HerdrCoreError> {
    HerdrCore::local().await
}

/// A named core on this machine, as `hc connect --session <name>` reaches it.
///
/// `hc connect` starts that core when it is not running, and it keeps running afterwards.
///
/// # Errors
///
/// When `name` is empty, `hc` cannot be started, or the handshake fails.
pub async fn session(name: &str) -> Result<HerdrCore, HerdrCoreError> {
    HerdrCore::builder().session(name).connect().await
}

/// The core on another machine, through `ssh <host> hc connect`.
///
/// `.command([...])` runs any other command that speaks the protocol on stdin and stdout.
///
/// # Errors
///
/// When `ssh` cannot be started, or the handshake fails.
pub async fn ssh(host: &str) -> Result<HerdrCore, HerdrCoreError> {
    HerdrCore::builder().ssh(host).connect().await
}

/// The local core over a transport built by hand.
///
/// `HerdrCore::over` takes any transport: here a `StreamTransport` over the pipes of a child
/// process started without the builder, in a test a fake. When the connection is dropped, the
/// child's stdin closes and `hc connect` exits by itself.
///
/// # Errors
///
/// When `hc` cannot be started, or the handshake fails.
pub async fn by_hand() -> Result<HerdrCore<StreamTransport<ChildStdout, ChildStdin>>, BoxError> {
    let mut child = Command::new("hc")
        .arg("connect")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("hc connect has no stdout pipe")?;
    let stdin = child.stdin.take().ok_or("hc connect has no stdin pipe")?;
    Ok(HerdrCore::over(StreamTransport::new(stdout, stdin)).await?)
}
