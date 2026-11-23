//! Core events: a subscription reports every terminal's start, changes and end.

use herdrify_hc::HerdrCore;
use herdrify_hc::connection::Subscription;
use herdrify_hc::protocol::{Event, EventKind, Notification, TerminalAddress};
use tokio::time::timeout;

use super::{BoxError, PATIENCE};

/// Subscribes to core events, starts a short command and prints its events until it exits.
///
/// # Errors
///
/// When the core refuses a call, the connection ends, or the exit is not reported in time.
pub async fn follow_a_terminal(core: &HerdrCore) -> Result<(), BoxError> {
    // Every terminal's events; `.terminal(..)`, `.children_of(..)` or `.labels(..)` narrow them.
    let mut events = core.events().await?;
    // The reply to the subscription is a snapshot: the terminals that exist as it starts.
    println!(
        "subscribed; {} terminals exist already",
        events.terminals().len()
    );

    let started = core
        .terminals()
        .run(["sh", "-c", "sleep 1; exit 3"])
        .await?;
    let address = started.result.address;
    println!("started {address}");

    timeout(PATIENCE, until_finished(&mut events, &address))
        .await
        .map_err(|_| "the terminal's exit was not reported in time")??;

    events.close().await?;
    started.terminal.rm().await?;
    Ok(())
}

/// Prints the events of the terminal at `address` until the one that says it finished.
async fn until_finished(
    events: &mut Subscription,
    address: &TerminalAddress,
) -> Result<(), BoxError> {
    while let Some(notification) = events.next().await {
        let Notification::Event(event) = notification? else {
            // Heartbeats, and notices of events the core had to drop, are not about one terminal.
            continue;
        };
        let Event { terminal, kind, .. } = *event;
        if terminal.as_ref() != Some(address) {
            continue;
        }
        match kind {
            EventKind::Finished {
                reason, exit_code, ..
            } => {
                println!("  {address} finished: {reason:?}, exit code {exit_code:?}");
                return Ok(());
            }
            EventKind::Created(_) => println!("  {address} created"),
            EventKind::Foreground(Some(process)) => {
                println!("  {address} runs {} (pid {})", process.name, process.pid);
            }
            EventKind::Title { title } => println!("  {address} title {title:?}"),
            other => println!("  {address} {other:?}"),
        }
    }
    Err("the subscription ended before the terminal finished".into())
}
