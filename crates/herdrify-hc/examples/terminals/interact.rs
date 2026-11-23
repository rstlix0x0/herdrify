//! Driving a terminal that stays open: type into it, read its screen, interrupt it, kill it.

use std::time::Duration;

use herdrify_hc::prelude::{HerdrCore, TerminalHandle};

use super::BoxError;

/// Starts a shell, runs two commands in it and then stops and removes it.
///
/// # Errors
///
/// When the core refuses a call, the connection ends, or the shell does not answer in time.
pub async fn drive_a_shell(core: &HerdrCore) -> Result<(), BoxError> {
    let shell = core.terminals().run(["sh"]).await?.terminal;
    println!("started a shell at {}", shell.reference());

    // `send` pastes the text and submits it with Enter.
    shell.send("echo $((6 * 7))").await?;
    screen_with_line(&shell, "42").await?;
    println!("the shell answered 42");

    // A command that would run for a while, stopped with an interrupt (Ctrl-C).
    shell.send("sleep 30").await?;
    let interrupted = shell.interrupt().timeout(Duration::from_secs(2)).await?;
    println!(
        "interrupted sleep with {}: {}",
        interrupted.key, interrupted.outcome
    );

    let killed = shell.kill().await?;
    println!("killed it: {:?}", killed.state);
    shell.rm().await?;
    println!("removed it");
    Ok(())
}

/// Reads the shell's screen until one of its lines is `line`, for up to five seconds.
async fn screen_with_line(shell: &TerminalHandle, line: &str) -> Result<(), BoxError> {
    for _ in 0..50 {
        let screen = shell.read().await?;
        if screen.text.lines().any(|shown| shown.trim() == line) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("{line:?} did not appear on the screen").into())
}
