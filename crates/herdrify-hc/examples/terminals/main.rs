//! Run programs in terminals on a core and drive them.
//!
//! Two walks, one after the other, on the core that `hc connect` reaches:
//!
//! - a command that exits by itself: run it, wait for it, read what it printed, remove it
//! - a shell that stays open: type into it, read its screen, interrupt it, kill it, remove it
//!
//! Every terminal it starts, it removes again.

pub mod interact;
pub mod lifecycle;

use herdrify_hc::HerdrCore;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let core = HerdrCore::local().await?;
    lifecycle::run_to_completion(&core).await?;
    interact::drive_a_shell(&core).await?;
    Ok(())
}
