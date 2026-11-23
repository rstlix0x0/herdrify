//! Follow what happens on a core as it happens: core events, and a live terminal.
//!
//! Two walks, one after the other, on the core that `hc connect` reaches:
//!
//! - subscribe to core events and follow one terminal from its start to its exit
//! - attach to a shell: stream its output, type into it, resize it, and see it exit
//!
//! Every terminal it starts, it removes again. Flow-control credit is granted by the SDK as the
//! streams are read, so neither walk deals with it.

pub mod attach;
pub mod events;

use std::time::Duration;

use herdrify_hc::HerdrCore;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// How long a walk waits for the core before it gives up.
const PATIENCE: Duration = Duration::from_secs(20);

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let core = HerdrCore::local().await?;
    events::follow_a_terminal(&core).await?;
    attach::talk_to_a_shell(&core).await?;
    Ok(())
}
