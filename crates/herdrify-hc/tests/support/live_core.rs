//! An `hc` core that belongs to one test, and the rule that skips a test when `hc` is missing.
//!
//! Each [`LiveCore`] has its own session, named `hfy-<pid>-<n>`, and its own short scratch
//! directory with the same name, which holds the core's state, config and runtime directories. The
//! directory is short because macOS refuses a socket path over 103 bytes. Every `hc` this file runs
//! has every `HC_*` variable of the surrounding environment removed, so a test run from inside a
//! Herdr terminal cannot reach that terminal's core, and every one but `hc --version` also names
//! the session.
//!
//! Responsibilities:
//!
//! - [`hc_available`], the skip rule
//! - [`LiveCore`], which starts nothing itself: the first `hc connect` starts the core, and
//!   dropping the guard stops it and removes the directory
//! - the only places this test crate runs `hc`
//!
//! Non-responsibilities:
//!
//! - the calls a test makes, which go through the SDK

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use herdrify_hc::ProcessTransport;
use herdrify_hc::adapter::ProcessTransportBuilder;
use herdrify_hc::protocol::SessionName;

/// The number of cores this process has started, so no two share a session or a directory.
static STARTED: AtomicU32 = AtomicU32::new(0);

/// How long a single `hc` command may run before it is killed.
const COMMAND_LIMIT: Duration = Duration::from_secs(30);

/// How long to wait for what a finished `hc` printed, which a stray child could keep open.
const OUTPUT_LIMIT: Duration = Duration::from_secs(2);

/// Where scratch directories live: `/tmp` on Unix, where the socket path limit rules out the long
/// per-user temporary directories, and the temporary directory elsewhere.
fn scratch_root() -> PathBuf {
    if cfg!(unix) {
        PathBuf::from("/tmp")
    } else {
        std::env::temp_dir()
    }
}

/// The environment every `hc` runs with: each `HC_*` variable removed, and the three directories
/// that hold a core's state moved under `dir`.
fn isolation(dir: &Path) -> Vec<(OsString, Option<OsString>)> {
    let mut edits: Vec<(OsString, Option<OsString>)> = std::env::vars_os()
        .filter(|(key, _)| key.as_encoded_bytes().starts_with(b"HC_"))
        .map(|(key, _)| (key, None))
        .collect();
    for (key, sub) in [
        ("XDG_STATE_HOME", "state"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_RUNTIME_DIR", "run"),
    ] {
        edits.push((key.into(), Some(dir.join(sub).into_os_string())));
    }
    edits
}

/// An `hc` command with the isolated environment of `dir` set, and no session named.
fn isolated_hc(dir: &Path) -> Command {
    let mut command = Command::new("hc");
    for (key, value) in isolation(dir) {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
    command
}

/// Runs `command` to the end, or kills it when it passes `COMMAND_LIMIT`.
fn run_bounded(mut command: Command) -> io::Result<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + COMMAND_LIMIT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "hc did not finish in time",
            ));
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        status,
        stdout: stdout.recv_timeout(OUTPUT_LIMIT).unwrap_or_default(),
        stderr: stderr.recv_timeout(OUTPUT_LIMIT).unwrap_or_default(),
    })
}

/// Reads a pipe to its end on another thread and sends what it read.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            // A pipe that fails half way still gives what it read.
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(bytes);
    });
    receiver
}

/// The version `hc --version` prints, or `None` when `hc` cannot be started.
fn hc_version() -> Option<String> {
    let dir = scratch_root().join(format!("hfy{}-version", std::process::id()));
    let mut command = isolated_hc(&dir);
    command.arg("--version");
    let output = run_bounded(command).ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Whether `hc` can be started: the rule a test checks first, to skip itself when it cannot.
#[must_use]
pub fn hc_available() -> bool {
    hc_version().is_some()
}

/// A core that belongs to one test. Dropping it stops the core and removes its directory.
#[derive(Debug)]
pub struct LiveCore {
    session: SessionName,
    dir: PathBuf,
}

impl LiveCore {
    /// Prepares a fresh session and directory. The core itself starts when a client first
    /// connects through [`LiveCore::transport`].
    ///
    /// A directory that already exists was left by a killed run whose process id was reused, so
    /// the core under it is stopped and the directory removed before it is created again. Only a
    /// real directory owned by the current user is treated that way; anything else at the path is
    /// refused, because the core's configuration under it would not be this run's own.
    ///
    /// # Errors
    ///
    /// Returns the I/O error that stopped the directory from being prepared, which includes a
    /// path that is not a directory of the current user.
    pub fn start() -> io::Result<Self> {
        let n = STARTED.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let session = SessionName::new(format!("hfy-{pid}-{n}")).map_err(io::Error::other)?;
        let core = Self {
            session,
            dir: scratch_root().join(format!("hfy{pid}-{n}")),
        };
        core.clear_leftover()?;
        create_private_dir(&core.dir)?;
        Ok(core)
    }

    /// Stops and removes what an earlier run left at the directory, and refuses what is not
    /// plainly such a leftover.
    fn clear_leftover(&self) -> io::Result<()> {
        let metadata = match std::fs::symlink_metadata(&self.dir) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        #[cfg(unix)]
        let owned = owned_by_current_user(&metadata)?;
        // Elsewhere the temporary directory is already private to the user.
        #[cfg(not(unix))]
        let owned = true;
        if !metadata.is_dir() || !owned {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "{} exists and is not a directory of the current user; remove it by hand",
                    self.dir.display()
                ),
            ));
        }
        // Whether a core was running there or not is no concern of this run.
        let _ = self.hc(&["server", "stop"]);
        std::fs::remove_dir_all(&self.dir)
    }

    /// The scratch directory, which a test may write files under.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The builder for `hc connect` on this core, with the isolated environment set.
    #[must_use]
    pub fn transport(&self) -> ProcessTransportBuilder {
        let mut builder = ProcessTransport::hc_connect().session(self.session.clone());
        for (key, value) in isolation(&self.dir) {
            builder = match value {
                Some(value) => builder.env(key, value),
                None => builder.env_remove(key),
            };
        }
        builder
    }

    /// Runs `hc` with `args` on this core and returns what it printed.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when `hc` cannot be started or does not finish in time.
    pub fn hc(&self, args: &[&str]) -> io::Result<Output> {
        let mut command = isolated_hc(&self.dir);
        command.args(args);
        command.env("HC_SESSION", self.session.as_str());
        run_bounded(command)
    }
}

/// Creates `dir`, failing when it exists, closed to other users.
#[cfg(unix)]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new().mode(0o700).create(dir)
}

/// Creates `dir`, failing when it exists. The temporary directory is already private to the user.
#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir(dir)
}

/// Whether `metadata` belongs to the user running this process.
///
/// The owner of a file this process creates is the current user, so that is what `metadata` is
/// compared with.
#[cfg(unix)]
fn owned_by_current_user(metadata: &std::fs::Metadata) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let probe = scratch_root().join(format!("hfy{}-owner", std::process::id()));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)?;
    let owner = file.metadata().map(|own| own.uid());
    drop(file);
    std::fs::remove_file(&probe)?;
    Ok(owner? == metadata.uid())
}

impl Drop for LiveCore {
    fn drop(&mut self) {
        match self.hc(&["server", "stop"]) {
            Ok(output) if output.status.success() => {}
            Ok(output) => eprintln!(
                "hfy-live: `hc server stop` for session {} exited with {}",
                self.session, output.status
            ),
            Err(error) => eprintln!(
                "hfy-live: `hc server stop` for session {} failed: {error}",
                self.session
            ),
        }
        // Retried for a short while, in case the core has not yet let go of the directory.
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.dir).is_ok() || !self.dir.exists() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        eprintln!("hfy-live: could not remove {}", self.dir.display());
    }
}
