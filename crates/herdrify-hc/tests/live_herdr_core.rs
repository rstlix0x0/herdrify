//! `HerdrCore` driven against a real `hc` core that belongs to the test alone.
//!
//! Each test starts its own isolated core, as `live.rs` does, and connects to it through
//! `HerdrCore::builder`, so the builder's session and environment edits are what reach it. A
//! machine without `hc` on `PATH` skips every test.
//!
//! Responsibilities:
//!
//! - the entrypoint end to end: connecting, terminals, streams, commands, files and docs
//!
//! Non-responsibilities:
//!
//! - each method's own behaviour, which `live.rs` covers through the client
//! - the isolated core itself, which `support/live_core.rs` owns

#[path = "support/live_core.rs"]
pub mod live_core;

use std::error::Error as StdError;
use std::future::{Future, IntoFuture};
use std::time::Duration;

use herdrify_hc::connection::{AttachEvent, CloseReason};
use herdrify_hc::protocol::{EventKind, ExecResult, Keep, Notification, WaitReason};
use herdrify_hc::{HerdrCore, HerdrCoreError};
use live_core::{LiveCore, hc_available};
use serde_json::json;
use tokio::time::timeout;

type TestError = Box<dyn StdError + Send + Sync>;
type TestResult = Result<(), TestError>;

/// How long a call to the core may take, so a regression fails the test instead of hanging it.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long the first connection may take, which includes starting the core.
const START_PATIENCE: Duration = Duration::from_secs(60);

/// A connected entrypoint and the core it talks to.
struct Live {
    core: HerdrCore,
    guard: LiveCore,
}

/// Starts an isolated core and connects to it, or returns `None` when `hc` is not installed.
async fn live() -> Result<Option<Live>, TestError> {
    if !hc_available() {
        return Ok(None);
    }
    let guard = LiveCore::start()?;
    let core = within(START_PATIENCE, guard.herdr_core().connect()).await??;
    Ok(Some(Live { core, guard }))
}

/// Awaits `call` under a time limit, failing the test when it hangs.
async fn within<C: IntoFuture>(limit: Duration, call: C) -> Result<C::Output, TestError> {
    timeout(limit, call.into_future())
        .await
        .map_err(|_| "the call did not finish in time".into())
}

/// Awaits `call` and returns its value, failing the test on an error or a hang.
async fn ok<T>(call: impl IntoFuture<Output = Result<T, HerdrCoreError>>) -> Result<T, TestError> {
    match timeout(PATIENCE, call.into_future()).await {
        Ok(result) => Ok(result?),
        Err(_) => Err("the call did not finish in time".into()),
    }
}

/// Runs `step` until it yields a value, for up to the patience.
async fn eventually<T, F: Future<Output = Result<Option<T>, TestError>>>(
    mut step: impl FnMut() -> F,
) -> Result<T, TestError> {
    timeout(PATIENCE, async {
        loop {
            if let Some(value) = step().await? {
                return Ok(value);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| -> TestError { "the condition never held".into() })?
}

#[tokio::test]
async fn live_herdr_core_connects_and_reports_status() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let status = ok(live.core.server().status()).await?;
    assert_eq!(status.live, 0);
    assert!(!live.core.hello().methods.is_empty());
    let templates = ok(live.core.templates()).await?;
    assert!(templates.templates.iter().all(|t| !t.name.is_empty()));
    let document = ok(live.core.discover()).await?;
    assert!(document.get("methods").is_some());
    Ok(())
}

#[tokio::test]
async fn live_herdr_core_runs_a_command_to_completion() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let terminals = live.core.terminals();
    let started = ok(terminals
        .run(["sh", "-c", "echo hello from herdr-core"])
        .labels([("role", "test")]))
    .await?;
    let shell = started.terminal;
    let waited = ok(shell.wait().timeout(Duration::from_secs(10))).await?;
    assert!(
        matches!(waited.reason, WaitReason::Exit),
        "{:?}",
        waited.reason
    );
    assert_eq!(waited.exit_code, Some(0));
    let screen = ok(shell.read()).await?;
    assert!(
        screen.text.contains("hello from herdr-core"),
        "{}",
        screen.text
    );
    let listed = ok(terminals.list().all().labels([("role", "test")])).await?;
    assert_eq!(listed.terminals.len(), 1);
    ok(shell.rm()).await?;
    let gone = within(PATIENCE, shell.get()).await?;
    assert!(matches!(gone, Err(HerdrCoreError::Call(_))), "{gone:?}");
    Ok(())
}

#[tokio::test]
async fn live_herdr_core_drives_a_shell() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let shell = ok(live.core.terminals().run(["sh"])).await?.terminal;
    ok(shell.keep(Keep::Off)).await?;
    ok(shell.set_labels([("step", "send")])).await?;
    ok(shell.send("echo $((6 * 7))")).await?;
    eventually(|| async {
        let screen = ok(shell.read()).await?;
        Ok(screen
            .text
            .lines()
            .any(|line| line.trim() == "42")
            .then_some(()))
    })
    .await?;
    ok(shell.keys(["ctrl+c"])).await?;
    let killed = ok(shell.kill()).await?;
    assert_eq!(killed.address.as_str(), shell.reference().as_str());
    ok(shell.rm()).await?;
    Ok(())
}

#[tokio::test]
async fn live_herdr_core_follows_events_and_attaches() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let mut events = ok(live.core.events()).await?;
    let shell = ok(live.core.terminals().run(["sh"])).await?.terminal;
    let created = timeout(PATIENCE, async {
        loop {
            match events.next().await {
                Some(Ok(Notification::Event(event))) => {
                    if let EventKind::Created(terminal) = event.kind {
                        return Ok(terminal.address);
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(TestError::from(error)),
                None => return Err("the subscription ended".into()),
            }
        }
    })
    .await
    .map_err(|_| -> TestError { "no terminal was created in time".into() })??;
    assert_eq!(created.as_str(), shell.reference().as_str());
    events.close().await?;

    let mut stream = ok(shell.attach(80, 24)).await?;
    stream.input(b"echo $((6 * 7))\r").await?;
    let mut seen = Vec::new();
    let closed = timeout(PATIENCE, async {
        loop {
            match stream.next().await {
                Some(Ok(AttachEvent::Output(bytes))) => {
                    seen.extend(bytes);
                    if String::from_utf8_lossy(&seen).contains("\n42") {
                        stream.input(b"exit\r").await?;
                        seen.clear();
                    }
                }
                Some(Ok(AttachEvent::Closed(reason))) => return Ok(reason),
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(TestError::from(error)),
                None => return Err("the stream ended without a close".into()),
            }
        }
    })
    .await
    .map_err(|_| -> TestError { "the shell did not answer and exit in time".into() })??;
    assert!(matches!(closed, CloseReason::Ended { .. }), "{closed:?}");
    ok(shell.rm()).await?;
    Ok(())
}

#[tokio::test]
async fn live_herdr_core_runs_commands_and_edits_files() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let core = &live.core;
    let counted: ExecResult = ok(core.exec(["wc", "-c"]).input("hello")).await?;
    assert_eq!(String::from_utf8_lossy(&counted.stdout).trim(), "5");
    let failed = ok(core.exec_line("exit 3")).await?;
    assert_eq!(failed.code, Some(3));
    let limited = ok(core.exec_line("sleep 5").timeout(Duration::from_secs(1))).await?;
    assert!(limited.timed_out);

    let path = live
        .guard
        .dir()
        .join("notes.txt")
        .to_string_lossy()
        .into_owned();
    let files = core.files();
    let written = ok(files.write(path.as_str(), "first line\nsecond line\n")).await?;
    assert_eq!(written.bytes, 23);
    ok(files.edit(path.as_str(), [("second line", "2nd line")])).await?;
    let second = ok(files.read(path.as_str()).offset(2).limit(1)).await?;
    assert_eq!(second.text, "2nd line\n");

    let docs = core.docs();
    let empty = ok(docs.get("settings")).await?;
    assert_eq!(empty.version.get(), 0);
    let stored = ok(docs.set("settings", json!({"a": 1}), 0)).await?;
    assert_eq!(ok(docs.get("settings")).await?.value, json!({"a": 1}));
    assert_eq!(stored.version.get(), 1);
    Ok(())
}
