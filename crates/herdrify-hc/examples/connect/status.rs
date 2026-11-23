//! What a connected `HerdrCore` can tell about its core, whatever transport carries it.

use herdrify_hc::{HerdrCore, Transport};

use super::BoxError;

/// Prints the core's version, its state and its live terminals.
///
/// # Errors
///
/// When the core refuses a call or closes the connection.
pub async fn report<T: Transport>(core: &HerdrCore<T>) -> Result<(), BoxError> {
    // The handshake's answer: the core's version and the methods it offers.
    let hello = core.hello();
    println!(
        "connected to hc {}, {} methods",
        hello.version,
        hello.methods.len()
    );

    let status = core.server().status().await?;
    println!(
        "core pid {}, up {} s, {} live and {} finished terminals",
        status.pid,
        status.uptime_ms / 1000,
        status.live,
        status.finished
    );

    for terminal in core.terminals().list().await?.terminals {
        println!("  {}  {}", terminal.address, terminal.argv.join(" "));
    }
    Ok(())
}
