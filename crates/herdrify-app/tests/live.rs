//! The SDK driven against a real `herdr-app` window that belongs to the test alone.
//!
//! Every window and every `herdr-app` or `hc` run here has its own home, its own `hc` session and
//! none of the `HC_*` variables or `HERDR_APP_HOME` of the environment it runs in, so it never
//! reaches a window or core the developer is running. The drift test needs `herdr-app` on `PATH`
//! and skips without it, but opens no window. The window scenario skips when `herdr-app` or `hc` is
//! missing from `PATH`, on a target that is not unix, and where no window can open. Each skip
//! prints a `skipped:` line that the log of a passing run shows.
//!
//! Responsibilities:
//!
//! - the isolation of the child processes, checked offline, down to the environment the programs
//!   see
//! - the retry of a command that the window refuses only for a moment, checked offline
//! - the command list of the installed `herdr-app` against the built-in table: a difference fails
//!   the test on the proven version, and is printed on any other
//! - one window, driven through the ten steps of the scenario, then ended
//!
//! Non-responsibilities:
//!
//! - the isolated window itself, which `support/live_window.rs` owns
//! - wire details, which the offline tests cover

#[path = "support/live_window.rs"]
pub mod live_window;

#[cfg(unix)]
use std::collections::BTreeMap;
use std::error::Error;
#[cfg(unix)]
use std::ffi::OsString;
use std::fmt;
use std::future::Future;
use std::time::Duration;

#[cfg(unix)]
use herdrify_app::TransportError;
use herdrify_app::files::{CommandName, Layout};
use herdrify_app::protocol::{
    Ack, AppVersion, CommandInfo, Created, Invocation, KNOWN_COMMANDS, Kept, KnownCommand, Opened,
    Parked, Place, Split, SplitDirection, TabMade,
};
use herdrify_app::{Error as AppError, HerdrApp};
#[cfg(unix)]
use live_window::{EnvEdit, Isolation, IsolationId};
use live_window::{LiveWindow, Scratch, notice, on_path};
use serde_json::{Map, Value, json};

/// The layout the window scenario sets, a copy of herdr-app's shipped `main`.
const MAIN_LAYOUT: &str = include_str!("fixtures/layouts/main.json");

/// What herdr-app says when a terminal is parked while its pane is still changing.
const PANE_CHANGE_IN_PROGRESS: &str = "This terminal already has a pane change in progress.";

/// What herdr-app says when a terminal is asked for before the window knows of it.
const NO_RUNNING_TERMINAL: &str = "no running terminal";

/// The program a scenario terminal runs: it stays up until the window ends.
const SLEEP: [&str; 3] = ["sh", "-c", "sleep 600"];

/// The environment a child would have after `isolation`'s edits are applied to `start`.
#[cfg(unix)]
fn apply(
    isolation: &Isolation,
    start: &[(&str, &str)],
) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut env: BTreeMap<String, String> = start
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    for edit in isolation.env() {
        let key = edit.key().to_str().ok_or("a variable name is not UTF-8")?;
        match edit {
            EnvEdit::Set { value, .. } => {
                let value = value.to_str().ok_or("a variable value is not UTF-8")?;
                env.insert(key.to_owned(), value.to_owned());
            }
            EnvEdit::Remove { .. } => {
                env.remove(key);
            }
        }
    }
    Ok(env)
}

/// What differs between the commands an installed `herdr-app` lists and the built-in table.
#[derive(Debug, Default)]
struct Drift {
    /// Commands the installed program lists and the table does not.
    added: Vec<String>,
    /// Commands the table has and the installed program does not list.
    removed: Vec<String>,
    /// Commands in both whose argument names or `required` flags differ, one line each.
    changed: Vec<String>,
}

impl Drift {
    const fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

impl fmt::Display for Drift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "added [{}]; removed [{}]; changed [{}]",
            self.added.join(", "),
            self.removed.join(", "),
            self.changed.join(", ")
        )
    }
}

/// Compares the installed command list with the table: names, argument names and `required`.
fn drift(installed: &[CommandInfo], known: &[KnownCommand]) -> Drift {
    let mut found = Drift::default();
    for command in installed {
        let name = command.name.as_str();
        match known.iter().find(|candidate| candidate.name == name) {
            None => found.added.push(name.to_owned()),
            Some(table) => found.changed.extend(differences(command, table)),
        }
    }
    for table in known {
        if !installed
            .iter()
            .any(|command| command.name.as_str() == table.name)
        {
            found.removed.push(table.name.to_owned());
        }
    }
    found
}

/// How one installed command differs from its table row: one line when the argument names
/// differ, otherwise one line for each argument whose `required` flag differs.
fn differences(installed: &CommandInfo, table: &KnownCommand) -> Vec<String> {
    let name = installed.name.as_str();
    let installed_names: Vec<&str> = installed.args.iter().map(|arg| arg.name.as_str()).collect();
    let table_names: Vec<&str> = table.args.iter().map(|(arg, _)| *arg).collect();
    if installed_names != table_names {
        return vec![format!(
            "{name}: args [{}] → [{}]",
            table_names.join(", "),
            installed_names.join(", ")
        )];
    }
    installed
        .args
        .iter()
        .zip(table.args)
        .filter(|(arg, (_, required))| arg.required != *required)
        .map(|(arg, (_, required))| {
            format!(
                "{name}: {} required {required} → {}",
                arg.name, arg.required
            )
        })
        .collect()
}

#[cfg(unix)]
#[test]
fn isolation_env_is_complete() -> Result<(), Box<dyn Error>> {
    let isolation = LiveWindow::isolation(IsolationId { pid: 4242, n: 0 });
    assert_eq!(isolation.home().to_str(), Some("/tmp/hfa4242-0"));
    assert_eq!(isolation.session(), "hfa4242-0");

    let mut sets = BTreeMap::new();
    for edit in isolation.env() {
        if let EnvEdit::Set { key, value } = edit {
            let key = key.to_str().ok_or("a variable name is not UTF-8")?;
            let value = value.to_str().ok_or("a variable value is not UTF-8")?;
            sets.insert(key.to_owned(), value.to_owned());
        }
    }
    let expected: BTreeMap<String, String> = [
        ("HERDR_APP_HOME", "/tmp/hfa4242-0"),
        ("HC_SESSION", "hfa4242-0"),
        ("XDG_STATE_HOME", "/tmp/hfa4242-0/x/state"),
        ("XDG_CONFIG_HOME", "/tmp/hfa4242-0/x/config"),
        ("XDG_RUNTIME_DIR", "/tmp/hfa4242-0/x/run"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    assert_eq!(sets, expected);

    // Every variable of this test's own environment that could reach another window or core is
    // removed, or set to the isolated value.
    for (key, _) in std::env::vars_os() {
        let hazard = key.as_encoded_bytes().starts_with(b"HC_") || key == "HERDR_APP_HOME";
        if hazard {
            let removed = isolation
                .env()
                .iter()
                .any(|edit| *edit.key() == key && matches!(edit, EnvEdit::Remove { .. }));
            assert!(removed, "{key:?} is inherited and not removed");
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn isolation_removes_inherited_variables() -> Result<(), Box<dyn Error>> {
    let isolation = Isolation::new(
        IsolationId { pid: 4242, n: 3 },
        ["HC_SESSION", "HC_SOCKET", "HERDR_APP_HOME", "PATH", "HOME"].map(OsString::from),
    );
    let env = apply(
        &isolation,
        &[
            ("HC_SESSION", "mine"),
            ("HC_SOCKET", "/run/mine.sock"),
            ("HERDR_APP_HOME", "/home/me/.config/herdr-app"),
            ("PATH", "/usr/bin"),
            ("HOME", "/home/me"),
        ],
    )?;
    let get = |key: &str| env.get(key).map(String::as_str);
    assert_eq!(get("HC_SESSION"), Some("hfa4242-3"));
    assert_eq!(get("HC_SOCKET"), None);
    assert_eq!(get("HERDR_APP_HOME"), Some("/tmp/hfa4242-3"));
    assert_eq!(get("XDG_RUNTIME_DIR"), Some("/tmp/hfa4242-3/x/run"));
    assert_eq!(get("PATH"), Some("/usr/bin"));
    assert_eq!(get("HOME"), Some("/home/me"));
    Ok(())
}

/// What the guard scripts print: the session, the home and the synthetic inherited variable.
#[cfg(unix)]
const SHOW_ENV: &str = r#"printf '%s|%s|%s' "$HC_SESSION" "$HERDR_APP_HOME" "${HC_GUARD-unset}""#;

/// Isolation 5 of process 4242, which was told that `HC_GUARD` is inherited.
#[cfg(unix)]
fn guarded_isolation() -> Isolation {
    Isolation::new(
        IsolationId { pid: 4242, n: 5 },
        [OsString::from("HC_GUARD")],
    )
}

#[cfg(unix)]
#[test]
fn a_command_runs_in_the_isolated_environment() -> Result<(), Box<dyn Error>> {
    let isolation = guarded_isolation();

    // The edits are applied over a command that already carries the inherited values.
    let mut inherited = std::process::Command::new("sh");
    inherited
        .args(["-c", SHOW_ENV])
        .env("HC_SESSION", "mine")
        .env("HERDR_APP_HOME", "/home/me/.config/herdr-app")
        .env("HC_GUARD", "leak");
    isolation.apply(&mut inherited);
    let shown = String::from_utf8(inherited.output()?.stdout)?;
    assert_eq!(shown, "hfa4242-5|/tmp/hfa4242-5|unset");

    // `command` applies them to a command of its own, whatever this process inherited.
    let mut own = isolation.command("sh");
    own.args(["-c", r#"printf '%s|%s' "$HC_SESSION" "$HERDR_APP_HOME""#]);
    let shown = String::from_utf8(own.output()?.stdout)?;
    assert_eq!(shown, "hfa4242-5|/tmp/hfa4242-5");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn the_sdk_entrypoint_runs_in_the_isolated_environment() -> Result<(), Box<dyn Error>> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;

    let isolation = guarded_isolation();
    // Answers with a version the SDK accepts only when it ran in the isolated environment.
    let body = format!(
        "#!/bin/sh\n\
         if [ \"$HC_SESSION\" = hfa4242-5 ] && [ \"$HERDR_APP_HOME\" = /tmp/hfa4242-5 ] \
         && [ -z \"${{HC_GUARD+set}}\" ]; then\n\
           printf 'herdr-app {}\\n'\n\
         else\n\
           printf 'herdr-app 0.0.1\\n'\n\
         fi\n",
        AppVersion::PROVEN
    );
    let dir = std::env::temp_dir().join(format!("hfa{}-guard", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let program = dir.join("herdr-app.sh");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o755)
        .open(&program)?;
    file.write_all(body.as_bytes())?;
    file.sync_all()?;
    drop(file);

    let builder = HerdrApp::builder()
        .program(&program)
        .env("HC_SESSION", "mine")
        .env("HERDR_APP_HOME", "/home/me/.config/herdr-app")
        .env("HC_GUARD", "leak");
    let builder = isolation.apply_to_builder(builder);
    // A fork of another test can hold the script's write descriptor for a moment, which makes the
    // first exec fail with ETXTBSY; the exec is started again until it is released.
    let mut attempt = 0;
    let connected = loop {
        let result = builder.clone().connect().await;
        attempt += 1;
        let busy = matches!(
            &result,
            Err(AppError::Transport(TransportError::Io { source, .. }))
                if source.kind() == std::io::ErrorKind::ExecutableFileBusy
        );
        if !busy || attempt == 20 {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    std::fs::remove_dir_all(&dir)?;

    connected?;
    Ok(())
}

#[test]
fn drift_reports_each_kind() -> Result<(), serde_json::Error> {
    let known = [
        KnownCommand {
            name: "a",
            args: &[("x", false)],
        },
        KnownCommand {
            name: "b",
            args: &[("y", true)],
        },
    ];
    let installed: Vec<CommandInfo> = serde_json::from_str(
        r#"[
            {"name": "b", "args": [{"name": "y", "required": false}], "about": ""},
            {"name": "c", "args": [], "about": ""}
        ]"#,
    )?;

    let found = drift(&installed, &known);

    assert_eq!(found.added, ["c"]);
    assert_eq!(found.removed, ["a"]);
    assert_eq!(found.changed, ["b: y required true → false"]);
    assert_eq!(
        found.to_string(),
        "added [c]; removed [a]; changed [b: y required true → false]"
    );
    assert!(!found.is_empty());
    Ok(())
}

#[tokio::test]
async fn installed_commands_match_known_commands() -> Result<(), Box<dyn Error>> {
    if !on_path("herdr-app") {
        notice("skipped: herdr-app is not on PATH");
        return Ok(());
    }
    let scratch = Scratch::create()?;
    // Connecting fails when the installed version is below the oldest one the crate supports.
    let app = scratch.isolation().app_builder().connect().await?;
    let found = drift(&app.commands().await?, KNOWN_COMMANDS);
    if *app.version() == AppVersion::PROVEN {
        assert!(
            found.is_empty(),
            "herdr-app {} lists other commands than the table: {found}",
            app.version()
        );
    } else {
        // Not worded as a compiler warning: the drift of a newer version is shown, and passes.
        notice(format_args!(
            "hfa-live: herdr-app {} differs from the proven {}: {found}",
            app.version(),
            AppVersion::PROVEN
        ));
    }
    Ok(())
}

/// Runs `attempt` until it is not refused with a message `transient` accepts, at most `attempts`
/// times and `pause` apart, and returns the last result. Any other result ends it at once.
async fn retry_refused<T, F, Fut>(
    attempts: u32,
    pause: Duration,
    transient: fn(&str) -> bool,
    mut attempt: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut tries = 1;
    loop {
        let result = attempt().await;
        let again = tries < attempts
            && matches!(&result, Err(AppError::Refused { message, .. }) if transient(message));
        if !again {
            return result;
        }
        tries += 1;
        tokio::time::sleep(pause).await;
    }
}

/// Parks `terminal`, once more after half a second when herdr-app is still changing its pane.
async fn park(app: &HerdrApp, terminal: &str) -> Result<Parked, AppError> {
    let terminals = app.terminals();
    retry_refused(
        2,
        Duration::from_millis(500),
        |message| message == PANE_CHANGE_IN_PROGRESS,
        || async { terminals.park(terminal).await },
    )
    .await
}

/// Keeps `terminal`, for up to five seconds while the window does not yet know of it.
async fn keep(app: &HerdrApp, terminal: &str) -> Result<Kept, AppError> {
    let terminals = app.terminals();
    retry_refused(
        50,
        Duration::from_millis(100),
        |message| message.contains(NO_RUNNING_TERMINAL),
        || async { terminals.keep(terminal).await },
    )
    .await
}

/// A refusal of the window with `message`.
fn refused(message: &str) -> AppError {
    AppError::Refused {
        invocation: "do terminal.park".to_owned(),
        message: message.to_owned(),
    }
}

#[tokio::test]
async fn a_refusal_that_passes_is_tried_again() -> Result<(), Box<dyn Error>> {
    let calls = std::cell::Cell::new(0_u32);
    let result = retry_refused(
        2,
        Duration::ZERO,
        |m| m == PANE_CHANGE_IN_PROGRESS,
        || {
            calls.set(calls.get() + 1);
            let first = calls.get() == 1;
            async move {
                if first {
                    Err(refused(PANE_CHANGE_IN_PROGRESS))
                } else {
                    Ok(7_u32)
                }
            }
        },
    )
    .await?;

    assert_eq!(result, 7);
    assert_eq!(calls.get(), 2);
    Ok(())
}

#[tokio::test]
async fn a_refusal_that_stays_ends_after_the_attempts() -> Result<(), Box<dyn Error>> {
    let calls = std::cell::Cell::new(0_u32);
    let result: Result<u32, AppError> = retry_refused(
        3,
        Duration::ZERO,
        |m| m == PANE_CHANGE_IN_PROGRESS,
        || {
            calls.set(calls.get() + 1);
            async { Err(refused(PANE_CHANGE_IN_PROGRESS)) }
        },
    )
    .await;

    let Err(AppError::Refused { message, .. }) = result else {
        return Err("the refusal should be returned".into());
    };
    assert_eq!(message, PANE_CHANGE_IN_PROGRESS);
    assert_eq!(calls.get(), 3);
    Ok(())
}

#[tokio::test]
async fn any_other_refusal_is_not_tried_again() -> Result<(), Box<dyn Error>> {
    let calls = std::cell::Cell::new(0_u32);
    let result: Result<u32, AppError> = retry_refused(
        5,
        Duration::ZERO,
        |m| m == PANE_CHANGE_IN_PROGRESS,
        || {
            calls.set(calls.get() + 1);
            async { Err(refused("something else")) }
        },
    )
    .await;

    let Err(AppError::Refused { message, .. }) = result else {
        return Err("the refusal should be returned".into());
    };
    assert_eq!(message, "something else");
    assert_eq!(calls.get(), 1);
    Ok(())
}

/// The ten steps of the scenario, against one window. None of them closes the window.
async fn scenario(window: &LiveWindow) -> Result<(), Box<dyn Error>> {
    let app = window.app().await?;

    // 1. `ls` and `status` decode.
    assert_eq!(app.status().await?.layout.as_str(), "main");
    assert!(app.ls().await?.spaces.is_empty());

    // 2. A space that runs a program.
    let created: Created = app.spaces().new_space().argv(SLEEP).await?;
    assert!(
        app.ls()
            .await?
            .spaces
            .iter()
            .any(|space| space.id == created.space)
    );

    // 3. A tab that runs the default shell has a terminal, and a tab that shows a mod has a pane.
    let shell_tab: TabMade = app.tabs().new_tab().await?;
    assert!(shell_tab.terminal.is_some());
    assert_eq!(shell_tab.space, created.space);
    let mod_tab: TabMade = app.tabs().new_mod_tab("files").await?;
    assert!(mod_tab.pane.is_some());

    // 4. A split that runs a program has a terminal, and a split that shows a mod has none.
    let program_split: Split = app.panes().split(SplitDirection::Right).argv(SLEEP).await?;
    assert!(program_split.terminal.is_some());
    let mod_split: Split = app.panes().split_mod(SplitDirection::Down, "files").await?;
    assert!(mod_split.terminal.is_none());

    // 5. Naming a pane.
    let Ack = app.panes().rename("hfa-pane").await?;

    // 6. A terminal opens in a tab of its own, or in a pane when it is given a place.
    let in_tab: Opened = app.terminals().open().argv(SLEEP).await?;
    assert!(in_tab.tab.is_some());
    let in_pane: Opened = app
        .terminals()
        .open()
        .argv(SLEEP)
        .place(Place::Right)
        .await?;
    assert!(in_pane.pane.is_some());

    // 7. A terminal that has no pane is given one, then parked again.
    let mut run = vec!["run", "--bg", "--json", "--no-parent", "--"];
    run.extend(SLEEP);
    let output = window.hc(&run).await?;
    if !output.status.success() {
        return Err(format!("hc run failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    let started: Value = serde_json::from_slice(&output.stdout)?;
    let address = started["address"]
        .as_str()
        .ok_or("hc run printed no address")?;
    // The window learns of a terminal the core started a moment later, so `keep` is asked again
    // while it is refused for that reason.
    let _kept: Kept = keep(&app, address).await?;
    let parked = park(&app, address).await?;
    assert_eq!(parked.terminal.as_str(), address);

    // 8. The next tab.
    let Ack = app.tabs().next().await?;

    // 9. An argument herdr-app does not know is refused, with its own words.
    let name = CommandName::new("tab.new")?;
    let mut arguments = Map::new();
    arguments.insert("bogus".to_owned(), json!(1));
    match app.client().raw(Invocation::run(&name, &arguments)).await {
        Err(AppError::Refused {
            invocation,
            message,
        }) => {
            assert_eq!(invocation, "do tab.new");
            assert!(
                message.starts_with("tab.new: unknown argument(s): bogus"),
                "unexpected refusal: {message}"
            );
        }
        other => return Err(format!("tab.new with a bogus argument gave {other:?}").into()),
    }

    // 10. A personal copy of the shipped layout, then a new tree for it.
    let copy = format!("hfa-{}", std::process::id());
    let Ack = app.layouts().clone("main").as_name(copy).await?;
    let Ack = app.layouts().set(Layout::parse(MAIN_LAYOUT)?).await?;
    Ok(())
}

#[tokio::test]
async fn isolated_window_scenario() -> Result<(), Box<dyn Error>> {
    let Some(window) = LiveWindow::launch().await? else {
        return Ok(());
    };
    let outcome = scenario(&window).await;
    let stopped = window.shutdown().await;
    match (outcome, stopped) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Err(error), Err(stopped)) => Err(format!(
            "the scenario failed: {error}; and ending the window failed: {stopped}"
        )
        .into()),
    }
}
