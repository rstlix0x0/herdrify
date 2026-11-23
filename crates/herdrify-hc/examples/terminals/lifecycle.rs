//! A terminal's whole life: run, wait, read, remove, and the typed refusal for one that is gone.

use std::time::Duration;

use herdrify_hc::HerdrCore;
use herdrify_hc::protocol::ErrorKind;

use super::BoxError;

/// Runs a command that exits by itself, waits for it, reads what it printed and removes it.
///
/// # Errors
///
/// When the core refuses a call, or the connection ends.
pub async fn run_to_completion(core: &HerdrCore) -> Result<(), BoxError> {
    // `run` takes the program and its arguments, and returns a handle on the new terminal.
    let started = core
        .terminals()
        .run(["sh", "-c", "echo hello from herdrify-hc"])
        .await?;
    let terminal = started.terminal;
    println!(
        "started {} ({:?})",
        started.result.address, started.result.state
    );

    // A wait ends when the terminal exits, or when its timeout passes first.
    let waited = terminal.wait().timeout(Duration::from_secs(10)).await?;
    println!(
        "wait ended: {:?}, exit code {:?}",
        waited.reason, waited.exit_code
    );

    // A finished terminal keeps its screen until it is removed.
    let screen = terminal.read().await?;
    println!("it printed: {}", screen.text.trim_end());

    let removed = terminal.rm().await?;
    println!("removed {}", removed.removed);

    // The core refuses a call about a terminal it no longer has, with a stable error kind.
    match terminal.get().await {
        Err(error)
            if error
                .refusal()
                .is_some_and(|r| r.kind == ErrorKind::NotFound) =>
        {
            let refusal = error.refusal().ok_or("a refusal")?;
            println!(
                "terminal.get refused it as {}: {}",
                refusal.kind, refusal.message
            );
        }
        Err(other) => return Err(other.into()),
        Ok(still) => return Err(format!("{} is still there", still.address).into()),
    }
    Ok(())
}
