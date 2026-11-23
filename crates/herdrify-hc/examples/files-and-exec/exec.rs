//! Commands run with `machine.exec`: no terminal, just output and an exit code.

use std::time::Duration;

use herdrify_hc::HerdrCore;

use super::BoxError;

/// Runs a few commands and prints what they printed and how they exited.
///
/// # Errors
///
/// When the core refuses a call, or the connection ends.
pub async fn run_commands(core: &HerdrCore) -> Result<(), BoxError> {
    // `exec` takes the program and its arguments; nothing runs before the `.await`.
    let uname = core.exec(["uname", "-s"]).await?;
    println!(
        "uname -s printed {:?}",
        String::from_utf8_lossy(&uname.stdout).trim()
    );

    // A command that fails is not an error of the call: its exit code says how it went.
    let failed = core.exec(["sh", "-c", "exit 3"]).await?;
    println!("sh -c 'exit 3' exited with {:?}", failed.code);

    // Bytes given as input reach the command's stdin.
    let counted = core.exec(["wc", "-c"]).input("hello").await?;
    println!(
        "wc -c counted {} bytes of input",
        String::from_utf8_lossy(&counted.stdout).trim()
    );

    // A command line, run by the machine's shell, with a time limit.
    let limited = core
        .exec_line("sleep 5; echo too late")
        .timeout(Duration::from_secs(1))
        .await?;
    println!("the 1 s time limit stopped it: {}", limited.timed_out);
    Ok(())
}

/// Makes a scratch directory with `mktemp -d` and returns its path.
///
/// # Errors
///
/// When the core refuses the call, the connection ends, or `mktemp` fails.
pub async fn scratch_dir(core: &HerdrCore) -> Result<String, BoxError> {
    let made = core.exec(["mktemp", "-d"]).await?;
    if made.code != Some(0) {
        return Err(format!(
            "mktemp -d failed: {}",
            String::from_utf8_lossy(&made.stderr)
        )
        .into());
    }
    let dir = String::from_utf8(made.stdout)?.trim().to_owned();
    println!("made {dir}");
    Ok(dir)
}

/// Removes `dir` and everything in it.
///
/// # Errors
///
/// When the core refuses the call, or the connection ends.
pub async fn remove_dir(core: &HerdrCore, dir: &str) -> Result<(), BoxError> {
    core.exec(["rm", "-rf", dir]).await?;
    println!("removed {dir}");
    Ok(())
}
