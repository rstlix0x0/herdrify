//! Run commands and work with files on the machine a core runs on, without a terminal.
//!
//! On the core that `hc connect` reaches, this:
//!
//! - runs commands and reads their output and exit code
//! - makes a scratch directory, then writes, reads and edits a file in it
//! - removes the scratch directory again
//!
//! The paths are the core's: through `ssh box hc connect` they are paths on `box`.

pub mod exec;
pub mod files;

use herdrify_hc::HerdrCore;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let core = HerdrCore::local().await?;
    exec::run_commands(&core).await?;

    let dir = exec::scratch_dir(&core).await?;
    let edited = files::write_read_edit(&core, &dir).await;
    // The scratch directory goes, whether or not the file walk succeeded.
    exec::remove_dir(&core, &dir).await?;
    edited
}
