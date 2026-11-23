//! A live terminal: its output as it is printed, and input as if typed at a keyboard.

use herdrify_hc::HerdrCore;
use herdrify_hc::connection::{AttachEvent, AttachStream, CloseReason};
use tokio::time::timeout;

use super::{BoxError, PATIENCE};

/// Starts a shell, attaches to it, types two commands and follows it until it exits.
///
/// # Errors
///
/// When the core refuses a call, the connection ends, or the shell does not answer in time.
pub async fn talk_to_a_shell(core: &HerdrCore) -> Result<(), BoxError> {
    let shell = core.terminals().run(["sh"]).await?.terminal;

    // The attach stream sees the terminal at 80 columns by 24 rows.
    let mut stream = shell.attach(80, 24).await?;
    println!("attached to {}", shell.reference());

    // Input is raw bytes, as a keyboard sends them: `\r` is Enter.
    stream.input(b"echo $((6 * 7))\r").await?;
    timeout(PATIENCE, output_until(&mut stream, "\n42"))
        .await
        .map_err(|_| "the shell did not answer in time")??;

    stream.resize(100, 30).await?;
    stream.input(b"exit\r").await?;
    let closed = timeout(PATIENCE, closed(&mut stream))
        .await
        .map_err(|_| "the shell did not exit in time")??;
    match closed {
        CloseReason::Ended {
            reason, exit_code, ..
        } => println!("the shell ended: {reason:?}, exit code {exit_code:?}"),
        other => println!("the stream closed: {other:?}"),
    }

    shell.rm().await?;
    Ok(())
}

/// Reads the stream's output until the output seen so far contains `needle`.
///
/// Output is the terminal's raw bytes, escape sequences included, as a terminal emulator would
/// draw them. The first bytes redraw the whole screen, so this counts them instead of printing
/// them over the screen of whoever runs the example.
async fn output_until(stream: &mut AttachStream, needle: &str) -> Result<(), BoxError> {
    let mut seen = Vec::new();
    while let Some(event) = stream.next().await {
        match event? {
            AttachEvent::Output(bytes) => {
                seen.extend(bytes);
                if String::from_utf8_lossy(&seen).contains(needle) {
                    println!(
                        "the shell answered {} after {} bytes of output",
                        needle.trim(),
                        seen.len()
                    );
                    return Ok(());
                }
            }
            AttachEvent::Resize { .. } => {}
            AttachEvent::Closed(reason) => {
                return Err(format!("the stream closed early: {reason:?}").into());
            }
        }
    }
    Err("the stream ended early".into())
}

/// Reads the stream until the core closes it, and returns why it closed.
async fn closed(stream: &mut AttachStream) -> Result<CloseReason, BoxError> {
    while let Some(event) = stream.next().await {
        if let AttachEvent::Closed(reason) = event? {
            return Ok(reason);
        }
    }
    Err("the stream ended without saying why".into())
}
