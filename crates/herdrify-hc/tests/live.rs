//! The SDK driven against a real `hc` core that belongs to the test alone.
//!
//! Every test starts its own core: its own session, its own scratch directory and none of the
//! `HC_*` variables of the environment it runs in, so it never reaches a core the developer is
//! running. A machine without `hc` on `PATH` skips every live test.
//!
//! Responsibilities:
//!
//! - the terminal methods, the two streams and the typed errors, each against a real core
//! - the put in pieces and its refusals, which a core older than 0.1.22 does not serve and which
//!   the tests then skip
//! - the `docs` filter of the event subscription, which the client refuses with
//!   `Error::NeedsNewerCore` against a core older than 0.1.22, before sending anything, and the
//!   test asserts that refusal there
//! - the check that the installed `hc` still publishes the schema the SDK was written against
//!
//! Non-responsibilities:
//!
//! - the isolated core itself, which `support/live_core.rs` owns
//! - wire details, which the offline tests cover

#[path = "support/live_core.rs"]
pub mod live_core;

use std::error::Error as StdError;
use std::future::Future;
use std::time::Duration;

use herdrify_hc::connection::{
    AttachEvent, AttachStream, CloseReason, ConnectionEndKind, Subscription,
};
use herdrify_hc::protocol::{
    Argv, Command, CoreVersion, DocGet, DocName, DocSet, DocVersion, Edit, EmptyResult, EndReason,
    ErrorKind, EventKind, EventsSubscribe, ExecCommand, FileEdit, FileName, FileRead, FileWrite,
    HelloResult, Keep, KeyName, LabelChange, LabelChanges, LabelKey, LabelValue, MachineAdd,
    MachineConnect, MachineExec, MachineList, MachineName, MachineRename, MachineRm, MachineTarget,
    Notification, RemotePath, RpcDiscover, ServerRestart, ServerStatus, ServerStop, TemplatesList,
    TerminalAddress, TerminalAttach, TerminalGet, TerminalInterrupt, TerminalKeep, TerminalKeys,
    TerminalKill, TerminalLabel, TerminalList, TerminalPut, TerminalPutAbort, TerminalPutBegin,
    TerminalPutChunk, TerminalPutFinish, TerminalRead, TerminalRef, TerminalRm, TerminalRun,
    TerminalSend, TerminalState, TerminalWait, TransferId, WaitReason, WaitTarget,
};
use herdrify_hc::{Client, Error, ProcessTransport};
use live_core::{LiveCore, hc_available};
use serde_json::{Value, json};
use tokio::time::timeout;

type TestError = Box<dyn StdError + Send + Sync>;
type TestResult = Result<(), TestError>;

/// How long a call to the core may take, so a regression fails the test instead of hanging it.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long the first connection may take, which includes starting the core.
const START_PATIENCE: Duration = Duration::from_secs(60);

/// A connected client and the core it talks to.
struct Live {
    client: Client<ProcessTransport>,
    core: LiveCore,
}

/// Starts an isolated core and connects to it, or returns `None` when `hc` is not installed.
async fn live() -> Result<Option<Live>, TestError> {
    if !hc_available() {
        return Ok(None);
    }
    let core = LiveCore::start()?;
    let transport = core.transport().spawn()?;
    let client = within(START_PATIENCE, Client::connect(transport)).await?;
    eprintln!(
        "hfy-live: running against hc core {}",
        client.hello().version
    );
    Ok(Some(Live { client, core }))
}

/// Runs a call under a time limit and turns its failure into a test failure.
async fn within<T>(
    limit: Duration,
    call: impl Future<Output = Result<T, Error>>,
) -> Result<T, TestError> {
    match timeout(limit, call).await {
        Ok(result) => Ok(result?),
        Err(_) => Err("the call did not finish in time".into()),
    }
}

fn argv(words: &[&str]) -> Result<Argv, TestError> {
    Ok(Argv::new(
        words.iter().map(|word| (*word).to_owned()).collect(),
    )?)
}

fn reference(address: &TerminalAddress) -> Result<TerminalRef, TestError> {
    Ok(TerminalRef::new(address.as_str())?)
}

/// Starts an `sh` terminal and returns the reference to it.
async fn start_sh(client: &Client<ProcessTransport>) -> Result<TerminalRef, TestError> {
    let run = TerminalRun::new(Command::Argv(argv(&["sh"])?));
    let started = within(PATIENCE, client.terminal_run(run)).await?;
    reference(&started.address)
}

#[tokio::test]
async fn live_connect_and_server_status() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let hello = live.client.hello();
    assert!(CoreVersion::parse(&hello.version)? >= CoreVersion::MINIMUM);
    let status = within(PATIENCE, live.client.server_status(ServerStatus::new())).await?;
    assert_eq!(status.version, hello.version);
    assert_eq!(status.core, hello.core);
    assert!(status.pid > 0);
    // A core of its own has no terminals; the developer's core, if reached, would have some.
    assert_eq!((status.live, status.finished), (0, 0));
    Ok(())
}

/// Whether the process `pid` still exists.
#[cfg(unix)]
fn process_exists(pid: u32) -> Result<bool, TestError> {
    let listed = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()?;
    Ok(!listed.stdout.is_empty())
}

#[cfg(unix)]
#[tokio::test]
async fn live_core_is_stopped_and_removed_on_drop() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let status = within(PATIENCE, live.client.server_status(ServerStatus::new())).await?;
    let dir = live.core.dir().to_path_buf();
    assert!(process_exists(status.pid)?, "the core is not running");

    drop(live);

    // The core exits a moment after the stop command returns.
    let mut waited = 0;
    while process_exists(status.pid)? && waited < 100 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        waited += 1;
    }
    assert!(!process_exists(status.pid)?, "the core outlived its guard");
    assert!(!dir.exists(), "{} was not removed", dir.display());
    Ok(())
}

#[tokio::test]
async fn live_templates_list() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    within(PATIENCE, live.client.templates_list(TemplatesList::new())).await?;
    Ok(())
}

#[tokio::test]
async fn live_terminal_lifecycle() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;

    let run = TerminalRun::new(Command::Argv(argv(&["sh"])?));
    let started = within(PATIENCE, client.terminal_run(run)).await?;
    let sh = reference(&started.address)?;
    assert_eq!(started.argv, ["sh"]);
    assert_eq!(started.state, TerminalState::Live);

    let listed = within(PATIENCE, client.terminal_list(TerminalList::new())).await?;
    assert!(
        listed
            .terminals
            .iter()
            .any(|t| t.address == started.address)
    );
    let got = within(PATIENCE, client.terminal_get(TerminalGet::new(sh.clone()))).await?;
    assert_eq!(got.address, started.address);
    let kept = within(
        PATIENCE,
        client.terminal_keep(TerminalKeep::new(sh.clone(), Keep::On)),
    )
    .await?;
    assert_eq!(kept.keep, Some(true));
    let unkept = within(
        PATIENCE,
        client.terminal_keep(TerminalKeep::new(sh.clone(), Keep::Off)),
    )
    .await?;
    assert_eq!(unkept.keep, Some(false));

    let key = LabelKey::new("a")?;
    let mut set = LabelChanges::new();
    set.insert(key.clone(), LabelChange::Set(LabelValue::new("1")?));
    let labelled = within(
        PATIENCE,
        client.terminal_label(TerminalLabel::new(sh.clone(), set)),
    )
    .await?;
    assert_eq!(labelled.labels.len(), 1);
    let mut remove = LabelChanges::new();
    remove.insert(key, LabelChange::Remove);
    let unlabelled = within(
        PATIENCE,
        client.terminal_label(TerminalLabel::new(sh.clone(), remove)),
    )
    .await?;
    assert!(unlabelled.labels.is_empty());

    within(
        PATIENCE,
        client.terminal_send(TerminalSend::new(sh.clone(), "echo sent-ok".to_owned())),
    )
    .await?;
    let read = within(
        PATIENCE,
        client.terminal_read(TerminalRead::new(sh.clone())),
    )
    .await?;
    assert!(read.text.contains("sent-ok"), "screen was {:?}", read.text);

    let keys = TerminalKeys::new(sh.clone(), vec![KeyName::new("enter")?]);
    within(PATIENCE, client.terminal_keys(keys)).await?;
    let interrupt = TerminalInterrupt::new(sh.clone()).timeout(Duration::from_secs(1));
    within(PATIENCE, client.terminal_interrupt(interrupt)).await?;

    let put = TerminalPut::new(sh.clone(), FileName::new("x.txt"), b"x\n".to_vec());
    let stored = within(PATIENCE, client.terminal_put(put)).await?;
    assert_eq!(stored.bytes, 2);
    let wait = TerminalWait::new(WaitTarget::Terminal(sh.clone())).timeout(Duration::from_secs(1));
    let waited = within(PATIENCE, client.terminal_wait(wait)).await?;
    assert_eq!(waited.reason, WaitReason::Timeout);
    assert!(!waited.ended);

    let killed = within(
        PATIENCE,
        client.terminal_kill(TerminalKill::new(sh.clone())),
    )
    .await?;
    assert_eq!(killed.address, started.address);
    let removed = within(PATIENCE, client.terminal_rm(TerminalRm::new(sh))).await?;
    assert_eq!(removed.removed, started.address.as_str());
    Ok(())
}

/// Reads notifications until one is the `created` event of `address`.
async fn next_created(
    subscription: &mut Subscription,
    address: &TerminalRef,
) -> Result<(), TestError> {
    let found = timeout(PATIENCE, async {
        loop {
            match subscription.next().await {
                Some(Ok(Notification::Event(event))) => {
                    if matches!(&event.kind, EventKind::Created(t) if t.address.as_str() == address.as_str())
                    {
                        return Ok(());
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(TestError::from(error)),
                None => return Err("the subscription ended before the event".into()),
            }
        }
    })
    .await;
    found.map_err(|_| TestError::from("no created event arrived in time"))?
}

#[tokio::test]
async fn live_events_subscribe() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    let mut subscription =
        within(PATIENCE, client.events_subscribe(EventsSubscribe::new())).await?;
    let sh = start_sh(client).await?;
    next_created(&mut subscription, &sh).await?;

    within(PATIENCE, subscription.close()).await?;
    let ended = timeout(PATIENCE, async {
        // The core may deliver what was already on its way before it closes the channel.
        loop {
            match subscription.next().await {
                None => return Ok(()),
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(TestError::from(error)),
            }
        }
    })
    .await;
    ended.map_err(|_| TestError::from("the subscription did not end in time"))??;

    let got = within(PATIENCE, client.terminal_get(TerminalGet::new(sh))).await?;
    assert_eq!(got.state, TerminalState::Live);
    Ok(())
}

/// Reads attach events until the bytes seen so far contain `needle`, and returns them.
async fn output_containing(stream: &mut AttachStream, needle: &str) -> Result<Vec<u8>, TestError> {
    let seen = timeout(PATIENCE, async {
        let mut seen = Vec::new();
        loop {
            match stream.next().await {
                Some(Ok(AttachEvent::Output(bytes))) => {
                    seen.extend(bytes);
                    if String::from_utf8_lossy(&seen).contains(needle) {
                        return Ok(seen);
                    }
                }
                Some(Ok(AttachEvent::Resize { .. })) => {}
                Some(Ok(AttachEvent::Closed(reason))) => {
                    return Err(TestError::from(format!("closed early: {reason:?}")));
                }
                Some(Err(error)) => return Err(TestError::from(error)),
                None => return Err("the stream ended early".into()),
            }
        }
    })
    .await;
    seen.map_err(|_| TestError::from(format!("{needle:?} did not arrive in time")))?
}

/// Reads attach events until the core closes the channel, and returns why.
async fn closed_with(stream: &mut AttachStream) -> Result<CloseReason, TestError> {
    let reason = timeout(PATIENCE, async {
        loop {
            match stream.next().await {
                Some(Ok(AttachEvent::Closed(reason))) => return Ok(reason),
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(TestError::from(error)),
                None => return Err("the stream ended without a close".into()),
            }
        }
    })
    .await;
    reason.map_err(|_| TestError::from("the channel was not closed in time"))?
}

#[tokio::test]
async fn live_terminal_attach() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    let sh = start_sh(client).await?;

    let mut stream = within(
        PATIENCE,
        client.terminal_attach(TerminalAttach::new(sh.clone(), 80, 24)),
    )
    .await?;
    within(PATIENCE, stream.input(b"echo framed-ok\r")).await?;
    output_containing(&mut stream, "framed-ok").await?;
    within(PATIENCE, stream.resize(100, 30)).await?;
    within(PATIENCE, stream.close()).await?;
    assert_eq!(closed_with(&mut stream).await?, CloseReason::Detached);

    let got = within(PATIENCE, client.terminal_get(TerminalGet::new(sh.clone()))).await?;
    assert_eq!(got.state, TerminalState::Live);

    let mut again = within(
        PATIENCE,
        client.terminal_attach(TerminalAttach::new(sh, 100, 30)),
    )
    .await?;
    within(PATIENCE, again.input(b"exit\r")).await?;
    let CloseReason::Ended {
        reason, exit_code, ..
    } = closed_with(&mut again).await?
    else {
        return Err("the terminal's end was not reported".into());
    };
    assert_eq!(reason, EndReason::Exited);
    assert_eq!(exit_code, Some(0));
    Ok(())
}

#[tokio::test]
async fn live_machine_exec() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let exec = MachineExec::new(ExecCommand::Argv(argv(&["echo", "hi"])?));
    let result = within(PATIENCE, live.client.machine_exec(exec)).await?;
    assert_eq!(result.stdout, b"hi\n");
    assert_eq!(result.code, Some(0));
    Ok(())
}

/// Whether the core's `hello` lists the method `name`.
fn has_method(hello: &HelloResult, name: &str) -> bool {
    hello.methods.iter().any(|method| method == name)
}

/// Whether a put test must stop because the core does not serve `terminal.put_begin`.
///
/// When the method is missing, prints a notice that starts with `hfy-live:`, never `warning:`, so
/// the log check of the gate ignores it. A core of 0.1.22 or newer always serves the method, so
/// there the missing method is an error and the test fails instead of skipping.
fn put_unsupported(hello: &HelloResult) -> Result<bool, TestError> {
    if has_method(hello, "terminal.put_begin") {
        return Ok(false);
    }
    if CoreVersion::parse(&hello.version)? >= CoreVersion::parse("0.1.22")? {
        return Err(format!(
            "hc {} must list terminal.put_begin in hello, but does not",
            hello.version
        )
        .into());
    }
    eprintln!(
        "hfy-live: skipped, no terminal.put_begin on hc {}",
        hello.version
    );
    Ok(true)
}

#[tokio::test]
async fn live_put_in_pieces() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    if put_unsupported(live.client.hello())? {
        return Ok(());
    }
    let core = timeout(START_PATIENCE, live.core.herdr_core().connect()).await??;
    let shell = timeout(PATIENCE, core.terminals().run(["sleep", "300"]))
        .await??
        .terminal;

    // Three pieces: two of 1 MiB and one of 512 KiB.
    let data = "abcdefghijklmnop".repeat(163_840);
    assert_eq!(data.len(), 2_621_440);
    let stored = timeout(PATIENCE, shell.put_in_pieces("big.txt", data)).await??;
    assert_eq!(stored.bytes, 2_621_440);

    let counted = timeout(PATIENCE, core.exec(["wc", "-c", stored.path.as_str()])).await??;
    let printed = String::from_utf8_lossy(&counted.stdout);
    assert_eq!(printed.split_whitespace().next(), Some("2621440"));

    timeout(PATIENCE, shell.kill()).await??;
    Ok(())
}

#[tokio::test]
async fn live_put_refusals() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    if put_unsupported(client.hello())? {
        return Ok(());
    }
    let sh = start_sh(client).await?;
    let begin = TerminalPutBegin::new(sh.clone(), FileName::new("p.txt"), 4, TransferId::random());
    let transfer = within(PATIENCE, client.terminal_put_begin(begin))
        .await?
        .transfer;

    let piece = TerminalPutChunk::new(transfer.clone(), 0, b"ab".to_vec());
    let held = within(PATIENCE, client.terminal_put_chunk(piece)).await?;
    assert_eq!(held.bytes, 2);

    let again = TerminalPutChunk::new(transfer.clone(), 0, b"ab".to_vec());
    let repeated = timeout(PATIENCE, client.terminal_put_chunk(again)).await?;
    assert_eq!(
        refusal(repeated)?,
        ("terminal.put_chunk".to_owned(), ErrorKind::InvalidParams)
    );

    let early = timeout(
        PATIENCE,
        client.terminal_put_finish(TerminalPutFinish::new(transfer.clone())),
    )
    .await?;
    assert_eq!(
        refusal(early)?,
        ("terminal.put_finish".to_owned(), ErrorKind::InvalidParams)
    );

    let aborted = within(
        PATIENCE,
        client.terminal_put_abort(TerminalPutAbort::new(transfer)),
    )
    .await?;
    assert_eq!(aborted, EmptyResult {});

    within(PATIENCE, client.terminal_kill(TerminalKill::new(sh))).await?;
    Ok(())
}

#[tokio::test]
async fn live_files() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    let path = live
        .core
        .dir()
        .join("files")
        .join("a.txt")
        .to_string_lossy()
        .into_owned();

    let write = FileWrite::new(RemotePath::new(path.as_str()), b"line1\nline2\n".to_vec());
    let written = within(PATIENCE, client.file_write(write)).await?;
    assert_eq!(written.bytes, 12);

    let read = FileRead::new(RemotePath::new(path.as_str()));
    let text = within(PATIENCE, client.file_read(read)).await?.text;
    assert_eq!(text, "line1\nline2\n");

    let edit = Edit {
        old: "line2".to_owned(),
        new: "LINE-TWO".to_owned(),
    };
    let edit = FileEdit::new(RemotePath::new(path.as_str()), vec![edit]);
    within(PATIENCE, client.file_edit(edit)).await?;
    let read = FileRead::new(RemotePath::new(path.as_str()));
    let text = within(PATIENCE, client.file_read(read)).await?.text;
    assert_eq!(text, "line1\nLINE-TWO\n");
    Ok(())
}

#[tokio::test]
async fn live_docs() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    let name = DocName::new("probe")?;

    let unwritten = within(PATIENCE, client.doc_get(DocGet::new(name.clone()))).await?;
    assert_eq!(unwritten.version, DocVersion(0));

    let first = DocSet::new(name.clone(), json!({"k": "v"}), DocVersion(0));
    let stored = within(PATIENCE, client.doc_set(first)).await?;
    assert_eq!(stored.version, DocVersion(1));
    let got = within(PATIENCE, client.doc_get(DocGet::new(name.clone()))).await?;
    assert_eq!(got.value, json!({"k": "v"}));

    let cleared = DocSet::new(name, Value::Null, DocVersion(1));
    within(PATIENCE, client.doc_set(cleared)).await?;
    Ok(())
}

/// Reads notifications until the document `last` has changed `times` times, and returns every
/// document change seen on the way, in order.
async fn doc_changes_until(
    subscription: &mut Subscription,
    last: &DocName,
    times: usize,
) -> Result<Vec<(DocName, DocVersion)>, TestError> {
    let outcome = timeout(PATIENCE, async {
        let mut changes = Vec::new();
        loop {
            match subscription.next().await {
                Some(Ok(Notification::Event(event))) => {
                    if let EventKind::Doc { name, version } = event.kind {
                        changes.push((name, version));
                        if changes.iter().filter(|(seen, _)| seen == last).count() == times {
                            return Ok(changes);
                        }
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(TestError::from(error)),
                None => return Err("the subscription ended before the event".into()),
            }
        }
    })
    .await;
    outcome.map_err(|_| TestError::from("the document event did not arrive in time"))?
}

#[tokio::test]
async fn live_events_docs() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    let wanted = DocName::new(format!("hfy-live-docs-{}", std::process::id()))?;
    let other = DocName::new(format!("hfy-live-other-{}", std::process::id()))?;
    let subscribe = EventsSubscribe::new().docs([wanted.clone()]);

    if CoreVersion::parse(&client.hello().version)? < CoreVersion::parse("0.1.22")? {
        eprintln!(
            "hfy-live: docs refused by the client on hc {}",
            client.hello().version
        );
        let refused = timeout(PATIENCE, client.events_subscribe(subscribe)).await?;
        let Err(Error::NeedsNewerCore { option, .. }) = refused else {
            return Err(format!("expected a refusal by the client, got {refused:?}").into());
        };
        assert_eq!(option, "events.subscribe docs");
        return Ok(());
    }

    let mut subscription = within(PATIENCE, client.events_subscribe(subscribe)).await?;
    assert!(subscription.terminals().is_empty());

    let first = DocSet::new(wanted.clone(), json!({"n": 1}), DocVersion(0));
    assert_eq!(
        within(PATIENCE, client.doc_set(first)).await?.version,
        DocVersion(1)
    );
    let elsewhere = DocSet::new(other, json!({"n": 1}), DocVersion(0));
    within(PATIENCE, client.doc_set(elsewhere)).await?;
    let second = DocSet::new(wanted.clone(), json!({"n": 2}), DocVersion(1));
    assert_eq!(
        within(PATIENCE, client.doc_set(second)).await?.version,
        DocVersion(2)
    );

    let changes = doc_changes_until(&mut subscription, &wanted, 2).await?;
    assert_eq!(
        changes,
        [(wanted.clone(), DocVersion(1)), (wanted, DocVersion(2))]
    );
    within(PATIENCE, subscription.close()).await?;
    Ok(())
}

#[tokio::test]
async fn live_machine_list() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let listed = within(PATIENCE, live.client.machine_list(MachineList::new())).await?;
    assert!(listed.machines.is_empty());
    Ok(())
}

#[tokio::test]
async fn live_rpc_discover() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let document = within(PATIENCE, live.client.rpc_discover(RpcDiscover::new())).await?;
    assert!(document.get("methods").is_some_and(Value::is_array));
    Ok(())
}

/// The kind of the refusal an `Err` carries, and the method it refused.
fn refusal(result: Result<impl std::fmt::Debug, Error>) -> Result<(String, ErrorKind), TestError> {
    match result {
        Err(Error::Rpc { method, error }) => Ok((method.to_string(), error.kind)),
        other => Err(format!("expected a refusal by the core, got {other:?}").into()),
    }
}

#[tokio::test]
async fn live_machine_typed_errors() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let client = &live.client;
    let nope = MachineName::new("nope")?;

    let target = MachineTarget::Via {
        name: MachineName::new("m1")?,
        command: "false".to_owned(),
    };
    let added = timeout(PATIENCE, client.machine_add(MachineAdd::new(target))).await?;
    assert_eq!(
        refusal(added)?,
        ("machine.add".to_owned(), ErrorKind::Unreachable)
    );

    let connected = timeout(
        PATIENCE,
        client.machine_connect(MachineConnect::new(nope.clone())),
    )
    .await?;
    assert_eq!(
        refusal(connected)?,
        ("machine.connect".to_owned(), ErrorKind::MachineUnknown)
    );
    let renamed = timeout(
        PATIENCE,
        client.machine_rename(MachineRename::new(nope.clone(), MachineName::new("x")?)),
    )
    .await?;
    assert_eq!(
        refusal(renamed)?,
        ("machine.rename".to_owned(), ErrorKind::MachineUnknown)
    );
    let removed = timeout(PATIENCE, client.machine_rm(MachineRm::new(nope))).await?;
    assert_eq!(
        refusal(removed)?,
        ("machine.rm".to_owned(), ErrorKind::MachineUnknown)
    );
    Ok(())
}

/// Waits for the connection to end and returns why.
async fn end_of(client: &Client<ProcessTransport>) -> Result<ConnectionEndKind, TestError> {
    let kind = timeout(PATIENCE, async {
        loop {
            if let Some(end) = client.close_reason() {
                return end.kind;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    kind.map_err(|_| TestError::from("the connection did not end in time"))
}

#[tokio::test]
async fn live_server_restart() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let stopped = within(PATIENCE, live.client.server_restart(ServerRestart::new())).await?;
    assert!(stopped.stopped);
    assert_eq!(end_of(&live.client).await?, ConnectionEndKind::PeerClosed);
    Ok(())
}

#[tokio::test]
async fn live_server_stop() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let stopped = within(PATIENCE, live.client.server_stop(ServerStop::new())).await?;
    assert!(stopped.stopped);
    assert_eq!(end_of(&live.client).await?, ConnectionEndKind::PeerClosed);
    Ok(())
}

/// The names of the methods and schemas, and of any other top-level part, in which `actual` is
/// not the same JSON as `expected`.
fn differences(expected: &Value, actual: &Value) -> Vec<String> {
    let mut differing = Vec::new();
    let by_name = |document: &Value| -> std::collections::BTreeMap<String, Value> {
        document["methods"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|method| Some((method["name"].as_str()?.to_owned(), method.clone())))
            .collect()
    };
    let (want, have) = (by_name(expected), by_name(actual));
    for name in want.keys().chain(have.keys()) {
        if want.get(name) != have.get(name) {
            differing.push(format!("method {name}"));
        }
    }
    let schemas = |document: &Value| document["components"]["schemas"].clone();
    let (want, have) = (schemas(expected), schemas(actual));
    let names = want
        .as_object()
        .into_iter()
        .chain(have.as_object())
        .flat_map(|schemas| schemas.keys());
    for name in names {
        if want.get(name) != have.get(name) {
            differing.push(format!("schema {name}"));
        }
    }
    let others = |document: &Value| -> Value {
        let mut rest = document.clone();
        if let Some(object) = rest.as_object_mut() {
            object.remove("methods");
            if let Some(components) = object.get_mut("components").and_then(Value::as_object_mut) {
                components.remove("schemas");
            }
        }
        rest
    };
    if others(expected) != others(actual) {
        differing.push("the rest of the document".to_owned());
    }
    differing.sort();
    differing.dedup();
    differing
}

/// What differs between two documents that are not equal, for a failure message.
fn describe_differences(expected: &Value, actual: &Value) -> String {
    let named = differences(expected, actual);
    if named.is_empty() {
        "the order of the methods, or an entry that repeats a name or has none".to_owned()
    } else {
        named.join(", ")
    }
}

#[tokio::test]
async fn live_schema_matches_the_fixture() -> TestResult {
    let Some(live) = live().await? else {
        return Ok(());
    };
    let fixture: Value = serde_json::from_str(include_str!("fixtures/hc-0.1.22.openrpc.json"))?;

    let printed = live.core.hc(&["schema"])?;
    assert!(printed.status.success(), "hc schema failed: {printed:?}");
    let printed: Value = serde_json::from_slice(&printed.stdout)?;
    let discovered = within(PATIENCE, live.client.rpc_discover(RpcDiscover::new())).await?;

    assert!(
        fixture == printed,
        "hc schema differs from the fixture in: {}",
        describe_differences(&fixture, &printed)
    );
    assert!(
        fixture == discovered,
        "rpc.discover differs from the fixture in: {}",
        describe_differences(&fixture, &discovered)
    );
    Ok(())
}

#[test]
fn differences_name_each_method_and_schema_that_differs() {
    let expected = json!({
        "info": {"version": "1"},
        "methods": [{"name": "a", "x": 1}, {"name": "b", "x": 1}, {"name": "c", "x": 1}],
        "components": {"schemas": {"S": {"x": 1}, "T": {"x": 1}}},
    });
    assert!(differences(&expected, &expected).is_empty());

    let actual = json!({
        "info": {"version": "1"},
        "methods": [{"name": "a", "x": 1}, {"name": "b", "x": 2}, {"name": "d", "x": 1}],
        "components": {"schemas": {"S": {"x": 1}, "T": {"x": 2}, "U": {"x": 1}}},
    });
    assert_eq!(
        differences(&expected, &actual),
        ["method b", "method c", "method d", "schema T", "schema U"]
    );

    let reordered = json!({
        "info": {"version": "1"},
        "methods": [{"name": "b", "x": 1}, {"name": "a", "x": 1}, {"name": "c", "x": 1}],
        "components": expected["components"],
    });
    assert!(differences(&expected, &reordered).is_empty());
    assert_ne!(expected, reordered);
    assert_eq!(
        describe_differences(&expected, &reordered),
        "the order of the methods, or an entry that repeats a name or has none"
    );

    let renamed = json!({"info": {"version": "2"}, "methods": expected["methods"], "components": expected["components"]});
    assert_eq!(
        differences(&expected, &renamed),
        ["the rest of the document"]
    );
}
