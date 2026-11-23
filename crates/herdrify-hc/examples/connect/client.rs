//! The same report without `HerdrCore`: a transport, a `Client` and the typed requests.
//!
//! This is the path `HerdrCore` is built on. It takes more lines and gives full control: the
//! transport is yours to build, and every request is a `protocol` type you fill in yourself.
//! `HerdrCore::client()` reaches the same `Client` from the entrypoint.

use herdrify_hc::protocol::{ServerStatus, TerminalList};
use herdrify_hc::{Client, ProcessTransport};

use super::BoxError;

/// Connects through `hc connect` and prints the core's version, its state and its terminals.
///
/// # Errors
///
/// When `hc` cannot be started, or the core refuses a call or closes the connection.
pub async fn report() -> Result<(), BoxError> {
    // `connect` says hello and switches the connection to frames, so calls can run side by side.
    let transport = ProcessTransport::hc_connect().spawn()?;
    let client = Client::connect(transport).await?;
    let hello = client.hello();
    println!(
        "connected to hc {}, {} methods",
        hello.version,
        hello.methods.len()
    );

    let status = client.server_status(ServerStatus::new()).await?;
    println!(
        "core pid {}, up {} s, {} live and {} finished terminals",
        status.pid,
        status.uptime_ms / 1000,
        status.live,
        status.finished
    );

    let listed = client.terminal_list(TerminalList::new()).await?;
    for terminal in &listed.terminals {
        println!("  {}  {}", terminal.address, terminal.argv.join(" "));
    }
    Ok(())
}
