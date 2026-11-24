//! A `herdr-app` window that belongs to one test, and the rules that skip a test when none can open.
//!
//! Every `herdr-app` and `hc` this file runs gets the same isolated environment: its own
//! `HERDR_APP_HOME`, its own `hc` session, its own `XDG_*` directories, and none of the `HC_*`
//! variables or the `HERDR_APP_HOME` of the surrounding environment. A test run from inside a
//! Herdr terminal therefore never reaches that terminal's window or core. The home is
//! `/tmp/hfa<pid>-<n>` on Unix, short because macOS refuses a socket path over 103 bytes.
//!
//! Responsibilities:
//!
//! - [`Isolation`], the names, paths and environment edits of one isolated window, and
//!   [`IsolationId`] and [`EnvEdit`], the values it is made of
//! - [`Scratch`], the home directory of one isolation, created private and removed on drop
//! - [`LiveWindow`], which launches the window, reaches it through [`HerdrApp`] and `hc`, and
//!   tears it down in order, on [`LiveWindow::shutdown`] or, when a test panics, on drop
//! - [`on_path`], the skip rule for a missing program, and [`notice`], which prints a skip or a
//!   warning where the log of a passing run shows it
//!
//! Non-responsibilities:
//!
//! - the commands a test sends, which go through the SDK

use std::error::Error as StdError;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use herdrify_app::{Error, HerdrApp, HerdrAppBuilder};

/// The number of isolations this process has made, so no two share a session or a directory.
static STARTED: AtomicU32 = AtomicU32::new(0);

/// How long a window that is alive may stay silent before the launch fails.
const LAUNCH_LIMIT: Duration = Duration::from_secs(45);

/// How long the launch waits between two `status` questions.
const LAUNCH_POLL: Duration = Duration::from_millis(300);

/// How long one `status` question may take.
const STATUS_LIMIT: Duration = Duration::from_secs(10);

/// How long a window gets to end after `app.quit`.
const QUIT_WAIT: Duration = Duration::from_secs(20);

/// How long a window gets to end after `SIGTERM`.
const TERM_WAIT: Duration = Duration::from_secs(10);

/// How long a single helper command of the teardown may run before it is killed.
const COMMAND_LIMIT: Duration = Duration::from_secs(30);

/// How long one `hc` call of a scenario may run before it is killed.
const HC_LIMIT: Duration = Duration::from_secs(30);

/// One change to a child's environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvEdit {
    /// The variable `key` is set to `value`.
    Set {
        /// The name of the variable.
        key: OsString,
        /// Its new value.
        value: OsString,
    },
    /// The variable `key` is removed.
    Remove {
        /// The name of the variable.
        key: OsString,
    },
}

impl EnvEdit {
    /// The name of the variable this edit changes.
    #[must_use]
    pub const fn key(&self) -> &OsString {
        match self {
            Self::Set { key, .. } | Self::Remove { key } => key,
        }
    }
}

/// Which isolation of which process: `pid` is the process id and `n` the count of isolations that
/// process had made before this one, so that no two isolations share a session or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsolationId {
    /// The process id.
    pub pid: u32,
    /// The count of isolations that process had made before this one.
    pub n: u32,
}

/// Prints `line` to standard error, past the capture of the test harness.
///
/// The harness hides what `println!` and `eprintln!` print while a test passes, which would hide a
/// skip or a warning from the log of a passing run. A direct write to the stream is not hidden.
pub fn notice(line: impl fmt::Display) {
    // A log that cannot be written leaves nothing to report the failure to.
    let _ = writeln!(io::stderr().lock(), "{line}");
}

/// Where scratch directories live: `/tmp` on Unix, where the socket path limit rules out the long
/// per-user temporary directories, and the temporary directory elsewhere.
fn scratch_root() -> PathBuf {
    if cfg!(unix) {
        PathBuf::from("/tmp")
    } else {
        std::env::temp_dir()
    }
}

/// Whether a variable of the surrounding environment must not reach an isolated child.
fn is_inherited_hazard(name: &OsString) -> bool {
    name.as_encoded_bytes().starts_with(b"HC_") || name == "HERDR_APP_HOME"
}

/// Whether a program with this name is a file in a directory of `PATH`.
#[must_use]
pub fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| {
            let file = dir.join(program);
            file.is_file() || (cfg!(windows) && file.with_extension("exe").is_file())
        })
    })
}

/// The names, paths and environment edits of one isolated window.
#[derive(Debug, Clone)]
pub struct Isolation {
    home: PathBuf,
    session: String,
    env: Vec<EnvEdit>,
}

impl Isolation {
    /// The isolation `id`, given the names of the variables of the surrounding environment. The
    /// edits remove every `HC_*` variable and `HERDR_APP_HOME` among them, then set the five
    /// variables that move the window and its core under the home.
    #[must_use]
    pub fn new(id: IsolationId, inherited: impl IntoIterator<Item = OsString>) -> Self {
        let IsolationId { pid, n } = id;
        let name = format!("hfa{pid}-{n}");
        let home = scratch_root().join(&name);
        let x = home.join("x");
        let mut env: Vec<EnvEdit> = inherited
            .into_iter()
            .filter(is_inherited_hazard)
            .map(|key| EnvEdit::Remove { key })
            .collect();
        for (key, value) in [
            ("HERDR_APP_HOME", home.clone()),
            ("HC_SESSION", PathBuf::from(&name)),
            ("XDG_STATE_HOME", x.join("state")),
            ("XDG_CONFIG_HOME", x.join("config")),
            ("XDG_RUNTIME_DIR", x.join("run")),
        ] {
            env.push(EnvEdit::Set {
                key: key.into(),
                value: value.into_os_string(),
            });
        }
        Self {
            home,
            session: name,
            env,
        }
    }

    /// The directory that is the window's `HERDR_APP_HOME`.
    #[must_use]
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The name of the `hc` session the window's core runs in.
    #[must_use]
    pub fn session(&self) -> &str {
        &self.session
    }

    /// The environment edits, in the order they apply: removals first, then the settings.
    #[must_use]
    pub fn env(&self) -> &[EnvEdit] {
        &self.env
    }

    /// A command for `program` with the isolated environment set, which no test runs without.
    #[must_use]
    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        self.apply(&mut command);
        command.stdin(Stdio::null());
        command
    }

    /// Applies the environment edits to `command`, in order, over whatever it already has.
    pub fn apply(&self, command: &mut Command) {
        for edit in &self.env {
            match edit {
                EnvEdit::Set { key, value } => command.env(key, value),
                EnvEdit::Remove { key } => command.env_remove(key),
            };
        }
    }

    /// The builder for the SDK entrypoint with the isolated environment set.
    pub fn app_builder(&self) -> HerdrAppBuilder {
        self.apply_to_builder(HerdrApp::builder())
    }

    /// Applies the environment edits to `builder`, in order, over whatever it already has.
    pub fn apply_to_builder(&self, mut builder: HerdrAppBuilder) -> HerdrAppBuilder {
        for edit in &self.env {
            builder = match edit {
                EnvEdit::Set { key, value } => builder.env(key, value),
                EnvEdit::Remove { key } => builder.env_remove(key),
            };
        }
        builder
    }
}

/// The home directory of one isolation. It is created private, and removed when dropped.
#[derive(Debug)]
pub struct Scratch {
    isolation: Isolation,
}

impl Scratch {
    /// Makes a fresh isolation and creates its home and its `x/` directories.
    ///
    /// A directory that already exists was left by a killed run whose process id was reused, so
    /// the core under it is stopped and the directory removed first. Only a real directory owned
    /// by the current user is treated that way; anything else at the path is refused.
    ///
    /// # Errors
    ///
    /// Returns the I/O error that stopped the directories from being prepared.
    pub fn create() -> io::Result<Self> {
        let n = STARTED.fetch_add(1, Ordering::SeqCst);
        let isolation = LiveWindow::isolation(IsolationId {
            pid: std::process::id(),
            n,
        });
        let scratch = Self { isolation };
        scratch.clear_leftover()?;
        create_private_dir(scratch.isolation.home())?;
        for sub in ["state", "config", "run"] {
            std::fs::create_dir_all(scratch.isolation.home().join("x").join(sub))?;
        }
        Ok(scratch)
    }

    /// The isolation whose home this is.
    #[must_use]
    pub const fn isolation(&self) -> &Isolation {
        &self.isolation
    }

    /// Stops and removes what an earlier run left at the home, and refuses what is not plainly
    /// such a leftover.
    fn clear_leftover(&self) -> io::Result<()> {
        let home = self.isolation.home();
        let metadata = match std::fs::symlink_metadata(home) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        #[cfg(unix)]
        let owned = owned_by_current_user(&metadata, self.isolation.session())?;
        // The owner check exists on unix only; any other target takes a leftover directory as its
        // own.
        #[cfg(not(unix))]
        let owned = true;
        if !metadata.is_dir() || !owned {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "{} exists and is not a directory of the current user; remove it by hand",
                    home.display()
                ),
            ));
        }
        // Whether a core was running there or not is no concern of this run.
        let mut stop = self.isolation.command("hc");
        stop.args(["server", "stop"]);
        let _ = run_bounded(stop, COMMAND_LIMIT);
        std::fs::remove_dir_all(home)
    }

    /// Removes the home, retrying for a short while in case the core has not yet let go of it.
    ///
    /// # Errors
    ///
    /// Returns the last I/O error when the home is still there after the retries.
    pub fn remove(&self) -> io::Result<()> {
        let home = self.isolation.home();
        let mut last = Ok(());
        for _ in 0..20 {
            match std::fs::remove_dir_all(home) {
                Ok(()) => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => last = Err(error),
            }
            thread::sleep(Duration::from_millis(100));
        }
        last
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = self.remove() {
            eprintln!(
                "hfa-live: could not remove {}: {error}",
                self.isolation.home().display()
            );
        }
    }
}

/// Creates `dir`, failing when it exists, closed to other users.
#[cfg(unix)]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new().mode(0o700).create(dir)
}

/// Creates `dir`, failing when it exists. It sets no permissions, which only unix has.
#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir(dir)
}

/// Whether `metadata` belongs to the user running this process.
///
/// The owner of a file this process creates is the current user, so that is what `metadata` is
/// compared with. The probe file is named after `session`, which no other isolation shares, and
/// one that a crash left behind is removed first.
#[cfg(unix)]
fn owned_by_current_user(metadata: &std::fs::Metadata, session: &str) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let probe = scratch_root().join(format!("{session}-owner"));
    match std::fs::remove_file(&probe) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)?;
    let owner = file.metadata().map(|own| own.uid());
    drop(file);
    std::fs::remove_file(&probe)?;
    Ok(owner? == metadata.uid())
}

/// Runs `command` with its output discarded, or kills it when it passes `limit`.
fn run_bounded(mut command: Command, limit: Duration) -> io::Result<ExitStatus> {
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(status) = wait_for_exit(&mut child, limit)? {
        return Ok(status);
    }
    child.kill()?;
    child.wait()?;
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "the command did not finish in time",
    ))
}

/// Polls `child` until it ends, for at most `limit`, and returns its status or `None` on time out.
fn wait_for_exit(child: &mut Child, limit: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Turns the outcome of a teardown step into an error that names the step, when it did not succeed.
fn check(step: &str, result: io::Result<ExitStatus>) -> io::Result<()> {
    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(io::Error::other(format!("{step} exited with {status}"))),
        Err(error) => Err(io::Error::new(
            error.kind(),
            format!("{step} failed: {error}"),
        )),
    }
}

/// Prints what a teardown step did when it did not succeed, for the steps whose failure the next
/// step makes up for.
fn report(step: &str, result: io::Result<ExitStatus>) {
    if let Err(error) = check(step, result) {
        eprintln!("hfa-live: {error}");
    }
}

/// The first error among `results`, after printing each of the others.
fn first_error(results: [io::Result<()>; 3]) -> io::Result<()> {
    let mut first = Ok(());
    for result in results {
        match (result, &first) {
            (Err(error), Ok(())) => first = Err(error),
            (Err(error), Err(_)) => eprintln!("hfa-live: {error}"),
            (Ok(()), _) => {}
        }
    }
    first
}

/// A `herdr-app` window that belongs to one test. Dropping it without [`LiveWindow::shutdown`]
/// runs the same teardown, so a test that panics still ends its window.
#[derive(Debug)]
pub struct LiveWindow {
    scratch: Scratch,
    child: Child,
    done: bool,
}

impl LiveWindow {
    /// The isolation `id`, with the surrounding environment as the source of the variables to
    /// remove.
    #[must_use]
    pub fn isolation(id: IsolationId) -> Isolation {
        Isolation::new(id, std::env::vars_os().map(|(key, _)| key))
    }

    /// Opens an isolated window and waits for it to answer `status`.
    ///
    /// Returns `Ok(None)` after printing `skipped: <reason>` with [`notice`] when `herdr-app` or
    /// `hc` is not on `PATH`, the target is not unix, or the window process ends before it
    /// answers, which is what a machine with no display does.
    ///
    /// # Errors
    ///
    /// Returns an error when the home cannot be prepared, the process cannot start, or the window
    /// is alive but has not answered `status` within 45 seconds.
    pub async fn launch() -> Result<Option<Self>, Box<dyn StdError>> {
        if !cfg!(unix) {
            notice("skipped: the target is not unix");
            return Ok(None);
        }
        for program in ["herdr-app", "hc"] {
            if !on_path(program) {
                notice(format_args!("skipped: {program} is not on PATH"));
                return Ok(None);
            }
        }
        let scratch = Scratch::create()?;
        let child = scratch
            .isolation()
            .command("herdr-app")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut window = Self {
            scratch,
            child,
            done: false,
        };
        let started = Instant::now();
        loop {
            if let Some(status) = window.child.try_wait()? {
                notice(format_args!(
                    "skipped: herdr-app ended with {status} before it answered status; \
                     no window can be opened here"
                ));
                return Ok(None);
            }
            if window.answers_status().await {
                return Ok(Some(window));
            }
            if started.elapsed() >= LAUNCH_LIMIT {
                return Err(format!(
                    "herdr-app is running but did not answer status within {} seconds",
                    LAUNCH_LIMIT.as_secs()
                )
                .into());
            }
            tokio::time::sleep(LAUNCH_POLL).await;
        }
    }

    /// Whether `herdr-app status` against this window succeeds.
    async fn answers_status(&self) -> bool {
        let mut command =
            tokio::process::Command::from(self.scratch.isolation().command("herdr-app"));
        command
            .arg("status")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        matches!(
            tokio::time::timeout(STATUS_LIMIT, command.status()).await,
            Ok(Ok(status)) if status.success()
        )
    }

    /// The SDK entrypoint on this window, built with the same isolation.
    ///
    /// # Errors
    ///
    /// Returns the error of [`HerdrAppBuilder::connect`].
    pub async fn app(&self) -> Result<HerdrApp, Error> {
        self.scratch.isolation().app_builder().connect().await
    }

    /// Runs `hc` with `args` in this window's isolated session, and returns what it printed.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when `hc` cannot be started or does not finish in time.
    pub async fn hc(&self, args: &[&str]) -> io::Result<Output> {
        let mut command = tokio::process::Command::from(self.scratch.isolation().command("hc"));
        command.args(args).kill_on_drop(true);
        tokio::time::timeout(HC_LIMIT, command.output())
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "hc did not finish in time"))?
    }

    /// Ends the window and removes its home, in order: `app.quit` and a wait, `SIGTERM` and a
    /// wait, a kill, `hc server stop` in the isolated session, then the home.
    ///
    /// # Errors
    ///
    /// Returns the first of: the failure to kill a window that would not end, the failure of
    /// `hc server stop`, the error that kept the home from being removed, or the error that ended
    /// the teardown task. The later failures of the three are printed.
    pub async fn shutdown(self) -> io::Result<()> {
        tokio::task::spawn_blocking(move || {
            let mut window = self;
            window.teardown()
        })
        .await
        .map_err(io::Error::other)?
    }

    /// The teardown that [`LiveWindow::shutdown`] and `Drop` share, blocking. Every step runs,
    /// whether an earlier one failed or not. `app.quit` and `SIGTERM` only print a failure, since
    /// the steps after them end the window anyway; the kill, `hc server stop` and the removal of
    /// the home are returned.
    fn teardown(&mut self) -> io::Result<()> {
        let isolation = self.scratch.isolation();
        let child = &mut self.child;
        let mut killed = Ok(());
        if !matches!(child.try_wait(), Ok(Some(_))) {
            let mut quit = isolation.command("herdr-app");
            quit.args(["do", "app.quit", "{}"]);
            report("app.quit", run_bounded(quit, COMMAND_LIMIT));
            if !matches!(wait_for_exit(child, QUIT_WAIT), Ok(Some(_))) {
                let mut term = Command::new("kill");
                term.args(["-TERM", &child.id().to_string()]);
                report("kill -TERM", run_bounded(term, COMMAND_LIMIT));
                if !matches!(wait_for_exit(child, TERM_WAIT), Ok(Some(_))) {
                    killed = child
                        .kill()
                        .and_then(|()| child.wait().map(drop))
                        .map_err(|error| {
                            io::Error::new(
                                error.kind(),
                                format!("killing herdr-app failed: {error}"),
                            )
                        });
                }
            }
        }
        let mut stop = isolation.command("hc");
        stop.args(["server", "stop"]);
        let stopped = check("hc server stop", run_bounded(stop, COMMAND_LIMIT));
        self.done = true;
        let removed = self.scratch.remove();
        first_error([killed, stopped, removed])
    }
}

impl Drop for LiveWindow {
    fn drop(&mut self) {
        if !self.done
            && let Err(error) = self.teardown()
        {
            eprintln!("hfa-live: teardown failed: {error}");
        }
    }
}
