//! `Client` driven through a scripted transport written outside the crate.
//!
//! The test plays herdr-core with a fake peer: it scripts the hello reply, answers requests in any
//! order, and sends the replies and stray messages the router must discard. It checks the
//! handshake refusals, raw calls and every typed method, using the public API only.
//!
//! Responsibilities:
//!
//! - every way `Client::connect` refuses a hello reply
//! - concurrent raw calls, dropped calls and the messages the router discards
//! - for each typed method, the request that reaches the peer and the reply decoded from a payload
//!   recorded from herdr-core 0.1.21, except where the constant's doc says otherwise
//! - event subscriptions: opening, delivery, credit, bad lines, the ways a stream ends and the
//!   frames that must be sent or withheld when it is closed or dropped
//! - `TerminalHandle::put_in_pieces`: the requests it sends for each size, and when it aborts
//! - the `docs` filter, refused before anything is sent on a core older than 0.1.22
//! - attach streams: opening, output and its credit, resize, the three close reasons, every write,
//!   the refusals of a write and connection loss
//! - that `Client`, `Subscription`, `AttachStream` and the futures of their calls are `Send`
//!
//! Non-responsibilities:
//!
//! - the writer, router and pending table in isolation, which their unit tests cover

#[path = "support/fake_transport.rs"]
pub mod fake_transport;

use std::future::Future;
use std::io;
use std::time::Duration;

use fake_transport::{FakePeer, FakeTransport, TestError, fake_pair};
use herdrify_hc::connection::{
    AttachEvent, AttachStream, CloseReason, ConnectionEndKind, Subscription,
};
use herdrify_hc::herdr_core::TerminalHandle;
use herdrify_hc::protocol::{
    Argv, Channel, Command, CoreVersion, DisplayFacts, DocGet, DocName, DocSet, DocVersion, Edit,
    EndReason, ErrorKind, EventKind, EventsSubscribe, ExecCommand, FileEdit, FileName, FileRead,
    FileWrite, Focus, Frame, FrameError, FrameKind, Keep, KeyName, LabelChange, LabelChanges,
    LabelKey, LabelValue, MachineAdd, MachineConnect, MachineExec, MachineList, MachineName,
    MachineRename, MachineRm, MachineTarget, MethodName, Notification, ProtocolError, PutResult,
    RemotePath, RequestId, RpcDiscover, Seq, ServerRestart, ServerStatus, ServerStop, StatusWord,
    StopResult, TemplatesList, TerminalAttach, TerminalGet, TerminalInterrupt, TerminalKeep,
    TerminalKeys, TerminalKill, TerminalLabel, TerminalList, TerminalPut, TerminalPutAbort,
    TerminalPutBegin, TerminalPutChunk, TerminalPutFinish, TerminalRead, TerminalRef, TerminalRm,
    TerminalRun, TerminalSend, TerminalState, TerminalWait, TransferId, UnexpectedReply,
    WaitReason, WaitTarget,
};
use herdrify_hc::{Client, Error, HerdrCore, HerdrCoreError, TransportError};
use serde_json::{Value, json};
use tokio::time::timeout;

type TestResult = Result<(), TestError>;

/// How long a test waits for a call to resolve, so a regression fails instead of hanging.
const PATIENCE: Duration = Duration::from_secs(5);

fn hello_result(version: &str, framed: bool, generation: u32) -> Value {
    json!({
        "core": {"id": "c1", "name": "core"},
        "features": ["framed"],
        "framed": framed,
        "generation": generation,
        "methods": ["hello"],
        "version": version,
    })
}

fn hello_reply(result: &Value) -> String {
    json!({"id": 0, "jsonrpc": "2.0", "result": result}).to_string()
}

fn method(name: &str) -> Result<MethodName, TestError> {
    Ok(MethodName::new(name)?)
}

/// A client connected over the fake, and the peer that answered its hello.
async fn connected() -> Result<(Client<FakeTransport>, FakePeer), TestError> {
    connected_at("0.1.21").await
}

/// As [`connected`], with a core that reports `version`.
async fn connected_at(version: &str) -> Result<(Client<FakeTransport>, FakePeer), TestError> {
    let (transport, mut peer) = fake_pair();
    peer.reply_line(&hello_reply(&hello_result(version, true, 1)))?;
    let client = timeout(PATIENCE, Client::connect(transport)).await??;
    let _hello = peer.line().await?;
    Ok((client, peer))
}

/// The error `Client::connect` returns when the peer answers with `reply`.
async fn refusal(reply: Option<&str>) -> Result<Error, TestError> {
    let (transport, mut peer) = fake_pair();
    match reply {
        Some(line) => peer.reply_line(line)?,
        None => peer.end(),
    }
    let outcome = timeout(PATIENCE, Client::connect(transport)).await?;
    outcome.err().ok_or_else(|| "connect succeeded".into())
}

#[tokio::test]
async fn connect_succeeds_on_a_valid_hello() -> TestResult {
    let (transport, mut peer) = fake_pair();
    peer.reply_line(&hello_reply(&hello_result("0.1.21", true, 1)))?;
    let client = timeout(PATIENCE, Client::connect(transport)).await??;
    let line = peer.line().await?;
    assert_eq!(
        serde_json::from_str::<Value>(line.as_str())?,
        json!({"id": 0, "jsonrpc": "2.0", "method": "hello",
               "params": {"framed": true, "generation": 1}})
    );
    assert_eq!(client.hello().version, "0.1.21");
    Ok(())
}

#[tokio::test]
async fn connect_refusals() -> TestResult {
    assert!(matches!(refusal(None).await?, Error::Closed));

    assert!(matches!(
        refusal(Some("not json")).await?,
        Error::Protocol(ProtocolError::InvalidJson { .. })
    ));
    assert!(matches!(
        refusal(Some("[1,2]")).await?,
        Error::Protocol(ProtocolError::NotJsonRpc)
    ));

    let other_id = json!({"id": 4, "jsonrpc": "2.0", "result": {}}).to_string();
    assert!(matches!(
        refusal(Some(&other_id)).await?,
        Error::Protocol(ProtocolError::UnexpectedHelloReply {
            got: UnexpectedReply::Response { id: RequestId(4) }
        })
    ));

    let null_id = json!({"id": null, "jsonrpc": "2.0", "result": {}}).to_string();
    let error = refusal(Some(&null_id)).await?;
    assert!(matches!(
        error,
        Error::Protocol(ProtocolError::UnexpectedHelloReply {
            got: UnexpectedReply::NullIdResponse
        })
    ));
    assert!(error.to_string().ends_with("got a response with id null"));

    let notification = json!({"jsonrpc": "2.0", "method": "heartbeat"}).to_string();
    assert!(matches!(
        refusal(Some(&notification)).await?,
        Error::Protocol(ProtocolError::UnexpectedHelloReply {
            got: UnexpectedReply::Notification
        })
    ));

    let generation = r#"{"error":{"code":-32000,"data":{"hint":"this core speaks generation 1","kind":"unsupported_generation"},"message":"protocol generation 2 not supported"},"id":0,"jsonrpc":"2.0"}"#;
    match refusal(Some(generation)).await? {
        Error::UnsupportedGeneration { message } => {
            assert_eq!(message, "protocol generation 2 not supported");
        }
        other => return Err(format!("unexpected {other}").into()),
    }

    let refused = json!({"error": {"code": -32000, "message": "no",
        "data": {"kind": "unavailable"}}, "id": 0, "jsonrpc": "2.0"})
    .to_string();
    match refusal(Some(&refused)).await? {
        Error::Rpc { method, error } => {
            assert_eq!(method.as_str(), "hello");
            assert_eq!(error.kind, ErrorKind::Unavailable);
        }
        other => return Err(format!("unexpected {other}").into()),
    }

    match refusal(Some(&hello_reply(&json!({"x": 1})))).await? {
        Error::Decode { method, .. } => assert_eq!(method.as_str(), "hello"),
        other => return Err(format!("unexpected {other}").into()),
    }

    for (framed, generation) in [(false, 1), (true, 2)] {
        let reply = hello_reply(&hello_result("0.1.21", framed, generation));
        assert!(
            matches!(
                refusal(Some(&reply)).await?,
                Error::Protocol(ProtocolError::HelloMismatch { .. })
            ),
            "framed={framed} generation={generation}"
        );
    }

    let invalid = hello_reply(&hello_result("0.1.x", true, 1));
    assert!(matches!(
        refusal(Some(&invalid)).await?,
        Error::InvalidVersion { .. }
    ));
    let old = hello_reply(&hello_result("0.1.20", true, 1));
    assert!(matches!(
        refusal(Some(&old)).await?,
        Error::UnsupportedVersion { .. }
    ));
    Ok(())
}

#[tokio::test]
async fn concurrent_calls_resolve_out_of_order() -> TestResult {
    let (client, mut peer) = connected().await?;
    let first = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call_raw(method("a.first")?, json!({}))
                .await
                .map_err(TestError::from)
        }
    });
    let second = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call_raw(method("b.second")?, json!({}))
                .await
                .map_err(TestError::from)
        }
    });

    let mut ids = std::collections::HashMap::new();
    for _ in 0..2 {
        let request = peer.request().await?;
        let name = request["method"].as_str().ok_or("no method")?.to_owned();
        ids.insert(name, request["id"].as_u64().ok_or("no id")?);
    }
    let first_id = *ids.get("a.first").ok_or("no a.first request")?;
    let second_id = *ids.get("b.second").ok_or("no b.second request")?;
    assert_ne!(first_id, second_id);

    peer.reply(second_id, &json!("second"))?;
    peer.reply(first_id, &json!("first"))?;
    assert_eq!(timeout(PATIENCE, first).await???, json!("first"));
    assert_eq!(timeout(PATIENCE, second).await???, json!("second"));
    Ok(())
}

#[tokio::test]
async fn late_reply_after_drop_is_discarded() -> TestResult {
    let (client, mut peer) = connected().await?;
    let abandoned = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call_raw(method("slow")?, json!({}))
                .await
                .map_err(TestError::from)
        }
    });
    let request = peer.request().await?;
    let id = request["id"].as_u64().ok_or("no id")?;

    abandoned.abort();
    assert!(
        timeout(PATIENCE, abandoned)
            .await?
            .is_err_and(|e| e.is_cancelled())
    );
    peer.reply(id, &json!("too late"))?;

    let next = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call_raw(method("fast")?, json!({}))
                .await
                .map_err(TestError::from)
        }
    });
    let request = peer.request().await?;
    assert_ne!(request["id"].as_u64().ok_or("no id")?, id);
    peer.reply(request["id"].as_u64().ok_or("no id")?, &json!("ok"))?;
    assert_eq!(timeout(PATIENCE, next).await???, json!("ok"));
    Ok(())
}

#[tokio::test]
async fn null_id_error_and_channel_zero_notification_are_discarded() -> TestResult {
    let (client, mut peer) = connected().await?;
    peer.push_data(
        br#"{"error":{"code":-32700,"data":{"hint":"h","kind":"parse_error"},"message":"m"},"id":null,"jsonrpc":"2.0"}"#,
    )?;
    peer.push_data(br#"{"jsonrpc":"2.0","method":"heartbeat","params":{"seq":1}}"#)?;
    peer.push_data(b"not json")?;
    peer.push_data(b"[1,2]")?;

    let call = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call_raw(method("after")?, json!({}))
                .await
                .map_err(TestError::from)
        }
    });
    let request = peer.request().await?;
    peer.reply(request["id"].as_u64().ok_or("no id")?, &json!(7))?;
    assert_eq!(timeout(PATIENCE, call).await???, json!(7));
    assert_eq!(client.close_reason(), None);
    Ok(())
}

/// Calls `name` on its own task, so the call can wait while the test drives the peer.
fn spawn_call(
    client: &Client<FakeTransport>,
    name: &'static str,
    params: Value,
) -> tokio::task::JoinHandle<Result<Value, Error>> {
    let client = client.clone();
    tokio::spawn(async move {
        let method = MethodName::new(name).map_err(|_| Error::Closed)?;
        client.call_raw(method, params).await
    })
}

async fn finished(
    call: tokio::task::JoinHandle<Result<Value, Error>>,
) -> Result<Result<Value, Error>, TestError> {
    Ok(timeout(PATIENCE, call).await??)
}

/// Waits until the client has recorded the end of the connection.
async fn wait_ended(client: &Client<FakeTransport>) -> Result<(), TestError> {
    timeout(PATIENCE, async {
        while client.close_reason().is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn eof_fails_pending_calls_with_closed() -> TestResult {
    let (client, mut peer) = connected().await?;
    assert_eq!(client.close_reason(), None);
    let call = spawn_call(&client, "slow", json!({}));
    let _request = peer.request().await?;

    peer.end();
    assert!(matches!(finished(call).await?, Err(Error::Closed)));
    let reason = client.close_reason().ok_or("no end recorded")?;
    assert_eq!(
        reason.to_string(),
        "peer closed: herdr-core closed the connection"
    );
    Ok(())
}

#[tokio::test]
async fn failed_write_fails_that_call_with_transport() -> TestResult {
    let (client, mut peer) = connected().await?;
    let waiting = spawn_call(&client, "waiting", json!({}));
    let _request = peer.request().await?;

    peer.fail_next_write();
    let failing = spawn_call(&client, "failing", json!({}));
    assert!(matches!(
        finished(failing).await?,
        Err(Error::Transport(TransportError::Io(_)))
    ));
    assert!(matches!(finished(waiting).await?, Err(Error::Closed)));
    let reason = client.close_reason().ok_or("no end recorded")?;
    assert_eq!(reason.kind, ConnectionEndKind::TransportFailed);
    Ok(())
}

#[tokio::test]
async fn oversized_frame_is_a_protocol_violation() -> TestResult {
    let (client, mut peer) = connected().await?;
    let call = spawn_call(&client, "slow", json!({}));
    let _request = peer.request().await?;

    peer.fail_source(TransportError::Frame(FrameError::TooLarge {
        len: 16_777_217,
    }))?;
    assert!(matches!(finished(call).await?, Err(Error::Closed)));
    let reason = client.close_reason().ok_or("no end recorded")?;
    assert_eq!(reason.kind, ConnectionEndKind::ProtocolViolation);
    assert_eq!(
        reason.message,
        "frame payload of 16777217 bytes exceeds the 16777216-byte limit"
    );
    Ok(())
}

#[tokio::test]
async fn calls_after_the_end_return_closed_without_writing() -> TestResult {
    let (client, mut peer) = connected().await?;
    peer.end();
    wait_ended(&client).await?;

    let outcome = client.call_raw(method("late")?, json!({})).await;
    assert!(matches!(outcome, Err(Error::Closed)));
    peer.nothing_sent().await
}

#[tokio::test]
async fn oversized_call_is_refused_before_sending() -> TestResult {
    let (client, mut peer) = connected().await?;
    let blob = json!({"blob": "x".repeat(16_777_216)});

    let outcome = client.call_raw(method("big")?, blob).await;
    assert!(matches!(outcome, Err(Error::PayloadTooLarge { len }) if len > 16_777_216));
    peer.nothing_sent().await?;

    let next = spawn_call(&client, "small", json!({}));
    let request = peer.request().await?;
    peer.reply(request["id"].as_u64().ok_or("no id")?, &json!("ok"))?;
    assert_eq!(finished(next).await??, json!("ok"));
    assert_eq!(client.close_reason(), None);
    Ok(())
}

#[tokio::test]
async fn closed_wins_over_payload_too_large() -> TestResult {
    let (client, mut peer) = connected().await?;
    peer.end();
    wait_ended(&client).await?;

    let blob = json!({"blob": "x".repeat(16_777_216)});
    let outcome = client.call_raw(method("big")?, blob).await;
    assert!(matches!(outcome, Err(Error::Closed)));
    peer.nothing_sent().await
}

#[tokio::test]
async fn last_client_drop_closes_the_sink() -> TestResult {
    let (client, mut peer) = connected().await?;
    let clone = client.clone();
    drop(client);
    peer.nothing_sent().await?;
    drop(clone);
    peer.sink_closed().await
}

#[tokio::test]
async fn source_io_error_is_a_transport_failure() -> TestResult {
    let (client, mut peer) = connected().await?;
    let call = spawn_call(&client, "slow", json!({}));
    let _request = peer.request().await?;

    peer.fail_source(TransportError::Io(io::Error::from(
        io::ErrorKind::ConnectionReset,
    )))?;
    assert!(matches!(finished(call).await?, Err(Error::Closed)));
    let reason = client.close_reason().ok_or("no end recorded")?;
    assert_eq!(reason.kind, ConnectionEndKind::TransportFailed);
    assert!(reason.message.starts_with("I/O error: "));
    Ok(())
}

#[tokio::test]
async fn end_resolves_calls_whose_write_never_completes() -> TestResult {
    let (client, mut peer) = connected().await?;
    peer.stall_next_write();
    let stalled = spawn_call(&client, "stalled", json!({}));
    let queued = spawn_call(&client, "queued", json!({}));
    tokio::time::sleep(Duration::from_millis(100)).await;

    peer.end();
    wait_ended(&client).await?;
    assert!(matches!(finished(stalled).await?, Err(Error::Closed)));
    assert!(matches!(finished(queued).await?, Err(Error::Closed)));
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::PeerClosed)
    );
    Ok(())
}

/// Runs `call` while the peer plays herdr-core: it checks that the request names `method` and
/// carries `params`, then sends the reply line `reply` builds from the request's id.
async fn exchange<T>(
    peer: &mut FakePeer,
    call: impl Future<Output = Result<T, Error>>,
    method: &str,
    params: Value,
    reply: impl FnOnce(u64) -> String,
) -> Result<Result<T, Error>, TestError> {
    let serve = async {
        let request = peer.request().await?;
        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["method"], method);
        assert_eq!(request["params"], params);
        let id = request["id"].as_u64().ok_or("no id")?;
        peer.push_data(reply(id).as_bytes())?;
        Ok::<(), TestError>(())
    };
    let (outcome, served) = timeout(PATIENCE, async { tokio::join!(call, serve) }).await?;
    served?;
    Ok(outcome)
}

/// Runs `call` while the peer answers it with `result`, a payload recorded from the core.
async fn answered<T>(
    peer: &mut FakePeer,
    call: impl Future<Output = Result<T, Error>>,
    method: &str,
    params: Value,
    result: &str,
) -> Result<T, TestError> {
    let reply = |id| format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#);
    Ok(exchange(peer, call, method, params, reply).await??)
}

/// Runs `call` while the peer refuses it with `error`, an error object recorded from the core, and
/// returns the error the call fails with.
async fn refused<T>(
    peer: &mut FakePeer,
    call: impl Future<Output = Result<T, Error>>,
    method: &str,
    params: Value,
    error: &str,
) -> Result<Error, TestError> {
    let reply = |id| format!(r#"{{"error":{error},"id":{id},"jsonrpc":"2.0"}}"#);
    exchange(peer, call, method, params, reply)
        .await?
        .err()
        .ok_or_else(|| "the call succeeded though the core refused it".into())
}

/// The reply recorded for `server.stop`.
const STOPPED: &str = r#"{"stopped":true}"#;

/// The error recorded for `file.read` sent with `offset` 0.
const INVALID_PARAMS: &str = r#"{"code":-32602,"data":{"hint":"see `hc schema`","kind":"invalid_params"},"message":"offset 0: the file has 3 lines"}"#;

/// The reply recorded for `terminal.put` of `hello`.
const PUT_RESULT: &str =
    r#"{"bytes":5,"path":"/tmp/hfycRyby/state/herdr-core/hfy-more/files/t2/x.txt"}"#;

const NOT_FOUND: &str = r#"{"code":-32000,"data":{"hint":"see `hc ls --all`","kind":"not_found"},"message":"no terminal \"t999\""}"#;

const MACHINE_UNKNOWN: &str = r#"{"code":-32000,"data":{"hint":"no machines yet: hc machine add <ssh-target>","kind":"machine_unknown"},"message":"unknown machine \"nope\""}"#;

const UNREACHABLE: &str = r#"{"code":-32000,"data":{"hint":"check that `false 'echo ok'` prints ok without asking anything","kind":"unreachable"},"message":"cannot reach false: the connection closed"}"#;

/// The head of the core's description of its methods: the `info` and `openrpc` members of the
/// upstream 0.1.21 `OpenRPC` schema, with an empty method list. It is assembled from that schema,
/// not recorded from a live reply, because the probes recorded only the size of the reply.
const DISCOVERED: &str = r#"{"info":{"description":"Terminal runtime for coding agents. JSON-RPC 2.0, one message per line on the hc socket (or framed after `hello`). Protocol generation 1. Errors carry a stable `data.kind`.","license":{"name":"Apache-2.0"},"title":"herdr-core","version":"0.1.21"},"methods":[],"openrpc":"1.3.2"}"#;

// The `cwd` values in the run, list, keep, label, get and kill replies below were recorded under
// a local directory; that path is replaced with the neutral `/work/probe-methods`.
const RUN_RESULT: &str = r#"{"address":"t1","argv":["sh","-c","sleep 5"],"cols":200,"created_ms":1795356727649,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"output":"","parent":null,"pid":56322,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0,"wait":{"children":[],"ended":false,"exit_code":null,"reason":"timeout","signal":null,"status":null,"terminal":"t1","turn":0}}"#;

const LIST_RESULT: &str = r#"{"hidden_finished":0,"machines":[],"terminals":[{"address":"t1","argv":["sh","-c","sleep 5"],"cols":200,"created_ms":1795356727649,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"foreground":{"argv":["sleep","5"],"cwd":"/work/probe-methods","name":"sleep","pid":56322},"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"parent":null,"pid":56322,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}]}"#;

const KEEP_RESULT: &str = r#"{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":true,"labels":{},"name":null,"notify":true,"parent":null,"pid":56464,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}"#;

const LABEL_RESULT: &str = r#"{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{"a":"1","b":"2"},"name":null,"notify":true,"parent":null,"pid":56464,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}"#;

const GET_RESULT: &str = r#"{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{"a":"1","b":"2"},"name":null,"notify":true,"parent":null,"pid":56464,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}"#;

const KILL_RESULT: &str = r#"{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":1795356754016,"identity":"ghostty","keep":false,"labels":{"b":"2"},"name":null,"notify":true,"parent":null,"pid":56464,"reason":"killed","reported":false,"rm":false,"rows":120,"screen":"exact","signal":1,"state":"finished","status":null,"status_ms":null,"template":null,"title":"","turn":0}"#;

// The core name was recorded as the machine's hostname and is replaced with the neutral `host`.
const STATUS_RESULT: &str = r#"{"core":{"id":"378c1e96a6faafa9cd6ada3bf9c3f2db","name":"host"},"finished":14,"holder":{"current":true,"instance":"2c40b0988be7732d6868982553755881","pid":56318,"terminals":0},"holders":[{"current":true,"instance":"2c40b0988be7732d6868982553755881","pid":56318,"terminals":0}],"installed_by":null,"keeps_running_after_logout":null,"live":0,"logout_linger_user":null,"pid":55595,"uptime_ms":182992,"version":"0.1.21"}"#;

#[tokio::test]
async fn server_status_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let status = answered(
        &mut peer,
        client.server_status(ServerStatus::new()),
        "server.status",
        json!({}),
        STATUS_RESULT,
    )
    .await?;
    assert_eq!(status.core.name, "host");
    assert_eq!(status.version, "0.1.21");
    assert_eq!((status.pid, status.live, status.finished), (55595, 0, 14));
    assert_eq!(status.holders.len(), 1);
    assert_eq!(status.holder.map(|holder| holder.pid), Some(56318));
    assert_eq!(status.installed_by, None);
    Ok(())
}

#[tokio::test]
async fn server_stop_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let stopped = answered(
        &mut peer,
        client.server_stop(ServerStop::new()),
        "server.stop",
        json!({}),
        STOPPED,
    )
    .await?;
    assert!(stopped.stopped);
    Ok(())
}

// No `server.restart` reply was recorded; the schema gives `StopResult` as its result type, so this
// reuses the recorded `server.stop` reply.
#[tokio::test]
async fn server_restart_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let stopped = answered(
        &mut peer,
        client.server_restart(ServerRestart::new()),
        "server.restart",
        json!({}),
        STOPPED,
    )
    .await?;
    assert!(stopped.stopped);
    Ok(())
}

#[tokio::test]
async fn templates_list_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let templates = answered(
        &mut peer,
        client.templates_list(TemplatesList::new()),
        "templates.list",
        json!({}),
        r#"{"templates":[]}"#,
    )
    .await?;
    assert!(templates.templates.is_empty());
    Ok(())
}

#[tokio::test]
async fn rpc_discover_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let document = answered(
        &mut peer,
        client.rpc_discover(RpcDiscover::new()),
        "rpc.discover",
        json!({}),
        DISCOVERED,
    )
    .await?;
    assert_eq!(document["openrpc"], "1.3.2");
    assert_eq!(document["info"]["title"], "herdr-core");
    Ok(())
}

#[tokio::test]
async fn server_stop_reply_then_peer_closed() -> TestResult {
    let (client, mut peer) = connected().await?;
    let stopped = answered(
        &mut peer,
        client.server_stop(ServerStop::new()),
        "server.stop",
        json!({}),
        STOPPED,
    )
    .await?;
    assert_eq!(stopped, StopResult { stopped: true });

    peer.end();
    wait_ended(&client).await?;
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::PeerClosed)
    );
    Ok(())
}

#[tokio::test]
async fn undecodable_result_is_decode_error() -> TestResult {
    let (client, mut peer) = connected().await?;
    let outcome = exchange(
        &mut peer,
        client.server_status(ServerStatus::new()),
        "server.status",
        json!({}),
        |id| format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{{"pid":-1}}}}"#),
    )
    .await?;
    match outcome {
        Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "server.status"),
        _ => return Err("expected Error::Decode".into()),
    }
    Ok(())
}

#[tokio::test]
async fn rpc_error_reply_is_rpc_error() -> TestResult {
    let (client, mut peer) = connected().await?;
    let path = "/tmp/hfycRyby/files/sub/a.txt";
    let error = refused(
        &mut peer,
        client.file_read(FileRead::new(RemotePath::new(path)).offset(0)),
        "file.read",
        json!({"offset": 0, "path": path}),
        INVALID_PARAMS,
    )
    .await?;
    match error {
        Error::Rpc { method, error } => {
            assert_eq!(method.as_str(), "file.read");
            assert_eq!(error.kind, ErrorKind::InvalidParams);
        }
        other => return Err(format!("unexpected {other}").into()),
    }
    Ok(())
}

fn terminal(name: &str) -> Result<TerminalRef, TestError> {
    Ok(TerminalRef::new(name)?)
}

#[tokio::test]
async fn terminal_run_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let argv = Argv::new(["sh", "-c", "sleep 5"].map(str::to_owned).to_vec())?;
    let run = TerminalRun::new(Command::Argv(argv))
        .timeout(Duration::from_secs(1))
        .wait();
    let started = answered(
        &mut peer,
        client.terminal_run(run),
        "terminal.run",
        json!({"argv": ["sh", "-c", "sleep 5"], "timeout": 1.0, "wait": true}),
        RUN_RESULT,
    )
    .await?;
    assert_eq!(started.address.as_str(), "t1");
    assert_eq!(started.argv, ["sh", "-c", "sleep 5"]);
    assert_eq!(started.state, TerminalState::Live);
    assert_eq!(started.output.as_deref(), Some(""));
    let wait = started.wait.ok_or("no wait object")?;
    assert!(!wait.ended);
    assert_eq!(wait.reason, WaitReason::Timeout);
    Ok(())
}

#[tokio::test]
async fn terminal_list_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let listed = answered(
        &mut peer,
        client.terminal_list(TerminalList::new()),
        "terminal.list",
        json!({}),
        LIST_RESULT,
    )
    .await?;
    assert_eq!(listed.hidden_finished, 0);
    assert!(listed.machines.is_empty());
    assert_eq!(listed.terminals.len(), 1);
    let first = &listed.terminals[0];
    assert_eq!(first.address.as_str(), "t1");
    assert_eq!(
        first.foreground.as_ref().map(|f| f.name.as_str()),
        Some("sleep")
    );
    Ok(())
}

#[tokio::test]
async fn terminal_get_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let got = answered(
        &mut peer,
        client.terminal_get(TerminalGet::new(terminal("t2")?)),
        "terminal.get",
        json!({"terminal": "t2"}),
        GET_RESULT,
    )
    .await?;
    assert_eq!(got.address.as_str(), "t2");
    assert_eq!(got.labels.len(), 2);
    assert_eq!(got.pid, Some(56464));
    Ok(())
}

#[tokio::test]
async fn terminal_kill_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let killed = answered(
        &mut peer,
        client.terminal_kill(TerminalKill::new(terminal("t2")?)),
        "terminal.kill",
        json!({"terminal": "t2"}),
        KILL_RESULT,
    )
    .await?;
    assert_eq!(killed.state, TerminalState::Finished);
    assert_eq!(killed.reason, Some(EndReason::Killed));
    assert_eq!(killed.signal, Some(1));
    Ok(())
}

#[tokio::test]
async fn terminal_rm_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let removed = answered(
        &mut peer,
        client.terminal_rm(TerminalRm::new(terminal("t1")?)),
        "terminal.rm",
        json!({"terminal": "t1"}),
        r#"{"removed":"t1"}"#,
    )
    .await?;
    assert_eq!(removed.removed, "t1");
    Ok(())
}

#[tokio::test]
async fn terminal_keep_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let kept = answered(
        &mut peer,
        client.terminal_keep(TerminalKeep::new(terminal("t2")?, Keep::On)),
        "terminal.keep",
        json!({"keep": true, "terminal": "t2"}),
        KEEP_RESULT,
    )
    .await?;
    assert_eq!(kept.address.as_str(), "t2");
    assert_eq!(kept.keep, Some(true));
    Ok(())
}

#[tokio::test]
async fn terminal_label_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let mut changes = LabelChanges::new();
    changes.insert(LabelKey::new("a")?, LabelChange::Set(LabelValue::new("1")?));
    changes.insert(LabelKey::new("b")?, LabelChange::Set(LabelValue::new("2")?));
    let labelled = answered(
        &mut peer,
        client.terminal_label(TerminalLabel::new(terminal("t2")?, changes)),
        "terminal.label",
        json!({"labels": {"a": "1", "b": "2"}, "terminal": "t2"}),
        LABEL_RESULT,
    )
    .await?;
    assert_eq!(labelled.address.as_str(), "t2");
    assert_eq!(labelled.labels.len(), 2);
    Ok(())
}

#[tokio::test]
async fn terminal_read_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let read = answered(
        &mut peer,
        client.terminal_read(TerminalRead::new(terminal("t2")?)),
        "terminal.read",
        json!({"terminal": "t2"}),
        r#"{"complete":true,"how":"screen","text":"sh-3.2$ echo sent-ok\nsent-ok\nsh-3.2$ echo sent-wait\nsent-wait\nsh-3.2$"}"#,
    )
    .await?;
    assert!(read.complete);
    assert_eq!(read.how, "screen");
    assert!(read.text.starts_with("sh-3.2$ echo sent-ok\nsent-ok\n"));
    assert_eq!(read.why, None);
    Ok(())
}

#[tokio::test]
async fn terminal_send_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let sent = answered(
        &mut peer,
        client.terminal_send(TerminalSend::new(
            terminal("t2")?,
            "echo sent-ok".to_owned(),
        )),
        "terminal.send",
        json!({"terminal": "t2", "text": "echo sent-ok"}),
        r#"{"started":null,"status":null,"turn":0}"#,
    )
    .await?;
    assert_eq!(sent.turn, 0);
    assert_eq!(sent.started, None);
    assert_eq!(sent.status, None);
    assert!(sent.wait.is_none());
    Ok(())
}

#[tokio::test]
async fn terminal_wait_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let wait =
        TerminalWait::new(WaitTarget::Terminal(terminal("t1")?)).timeout(Duration::from_secs(10));
    let waited = answered(
        &mut peer,
        client.terminal_wait(wait),
        "terminal.wait",
        json!({"terminal": "t1", "timeout": 10.0}),
        r#"{"children":[],"ended":true,"exit_code":0,"reason":"exit","signal":null,"status":null,"terminal":"t1","turn":0}"#,
    )
    .await?;
    assert!(waited.ended);
    assert_eq!(waited.terminal, "t1");
    assert_eq!(waited.exit_code, Some(0));
    assert_eq!(waited.reason, WaitReason::Exit);
    assert_eq!(waited.status, None);
    assert!(waited.children.is_empty());
    Ok(())
}

#[tokio::test]
async fn terminal_keys_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let keys = TerminalKeys::new(terminal("t2")?, vec![KeyName::new("enter")?]);
    let pressed = answered(
        &mut peer,
        client.terminal_keys(keys),
        "terminal.keys",
        json!({"keys": ["enter"], "terminal": "t2"}),
        r#"{"status":null,"turn":0}"#,
    )
    .await?;
    assert_eq!(pressed.turn, 0);
    assert_eq!(pressed.status, None::<StatusWord>);
    Ok(())
}

#[tokio::test]
async fn terminal_interrupt_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let interrupted = answered(
        &mut peer,
        client.terminal_interrupt(TerminalInterrupt::new(terminal("t2")?)),
        "terminal.interrupt",
        json!({"terminal": "t2"}),
        r#"{"key":"ctrl+c","outcome":"unknown"}"#,
    )
    .await?;
    assert_eq!(interrupted.key, "ctrl+c");
    assert_eq!(interrupted.outcome, "unknown");
    Ok(())
}

#[tokio::test]
async fn terminal_put_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let put = TerminalPut::new(terminal("t2")?, FileName::new("x.txt"), b"hello".to_vec());
    let stored = answered(
        &mut peer,
        client.terminal_put(put),
        "terminal.put",
        json!({"data": "aGVsbG8=", "name": "x.txt", "terminal": "t2"}),
        PUT_RESULT,
    )
    .await?;
    assert_eq!(stored.bytes, 5);
    assert!(stored.path.ends_with("/files/t2/x.txt"));
    Ok(())
}

#[tokio::test]
async fn terminal_put_sends_base64() -> TestResult {
    let (client, mut peer) = connected().await?;
    let put = TerminalPut::new(
        terminal("t2")?,
        FileName::new("x.txt"),
        vec![0x00, 0xff, 0xfe, 0x01, 0x7f],
    );
    let call = client.terminal_put(put);
    let serve = async {
        let request = peer.request().await?;
        assert_eq!(request["params"]["data"], "AP/+AX8=");
        let id = request["id"].as_u64().ok_or("no id")?;
        peer.reply(id, &serde_json::from_str::<Value>(PUT_RESULT)?)?;
        Ok::<(), TestError>(())
    };
    let (stored, served) = timeout(PATIENCE, async { tokio::join!(call, serve) }).await?;
    served?;
    assert_eq!(stored?.bytes, 5);
    Ok(())
}

#[tokio::test]
async fn terminal_get_not_found() -> TestResult {
    let (client, mut peer) = connected().await?;
    let error = refused(
        &mut peer,
        client.terminal_get(TerminalGet::new(terminal("t999")?)),
        "terminal.get",
        json!({"terminal": "t999"}),
        NOT_FOUND,
    )
    .await?;
    assert!(matches!(error, Error::Rpc { .. }));
    assert_eq!(
        error.to_string(),
        "terminal.get refused by herdr-core: no terminal \"t999\" [not_found]"
    );
    Ok(())
}

/// The transfer the put-in-pieces tests name.
const TRANSFER: &str = "0123456789abcdef";

fn transfer() -> Result<TransferId, TestError> {
    Ok(TransferId::new(TRANSFER)?)
}

#[tokio::test]
async fn put_calls_send_their_method_and_params() -> TestResult {
    let (client, mut peer) = connected_at("0.1.22").await?;

    let begun = answered(
        &mut peer,
        client.terminal_put_begin(TerminalPutBegin::new(
            terminal("t2")?,
            FileName::new("x.txt"),
            5,
            transfer()?,
        )),
        "terminal.put_begin",
        json!({"bytes": 5, "name": "x.txt", "terminal": "t2", "transfer": TRANSFER}),
        &format!(r#"{{"transfer":"{TRANSFER}"}}"#),
    )
    .await?;
    assert_eq!(begun.transfer.as_str(), TRANSFER);

    let piece = answered(
        &mut peer,
        client.terminal_put_chunk(TerminalPutChunk::new(transfer()?, 0, b"hello".to_vec())),
        "terminal.put_chunk",
        json!({"data": "aGVsbG8=", "offset": 0, "transfer": TRANSFER}),
        r#"{"bytes":5}"#,
    )
    .await?;
    assert_eq!(piece.bytes, 5);

    let stored = answered(
        &mut peer,
        client.terminal_put_finish(TerminalPutFinish::new(transfer()?)),
        "terminal.put_finish",
        json!({"transfer": TRANSFER}),
        PUT_RESULT,
    )
    .await?;
    assert_eq!(stored.bytes, 5);
    assert!(stored.path.ends_with("/files/t2/x.txt"));

    answered(
        &mut peer,
        client.terminal_put_abort(TerminalPutAbort::new(transfer()?)),
        "terminal.put_abort",
        json!({"transfer": TRANSFER}),
        "{}",
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn put_refusal_keeps_the_core_kind() -> TestResult {
    let (client, mut peer) = connected().await?;
    let error = refused(
        &mut peer,
        client.terminal_put_begin(TerminalPutBegin::new(
            terminal("t2")?,
            FileName::new("x.txt"),
            5,
            transfer()?,
        )),
        "terminal.put_begin",
        json!({"bytes": 5, "name": "x.txt", "terminal": "t2", "transfer": TRANSFER}),
        r#"{"code":-32601,"data":{"kind":"unknown_method"},"message":"unknown method \"terminal.put_begin\""}"#,
    )
    .await?;
    assert_eq!(
        rpc_kind(error)?,
        ("terminal.put_begin".to_owned(), ErrorKind::UnknownMethod)
    );
    Ok(())
}

/// What the peer answers one request of a put in pieces with.
enum Answer {
    /// A result as the core sends it.
    Result(String),
    /// An error object as the core sends it.
    Refusal(&'static str),
}

/// The reply of `terminal.put_begin`.
fn opened() -> Answer {
    Answer::Result(format!(r#"{{"transfer":"{TRANSFER}"}}"#))
}

/// The reply of `terminal.put_chunk` once the core holds `bytes` bytes.
fn held(bytes: u64) -> Answer {
    Answer::Result(format!(r#"{{"bytes":{bytes}}}"#))
}

/// The reply of `terminal.put_finish`.
fn stored() -> Answer {
    Answer::Result(PUT_RESULT.to_owned())
}

/// The reply of `terminal.put_abort`.
fn dropped() -> Answer {
    Answer::Result("{}".to_owned())
}

/// The error herdr-core 0.1.22 sent for a piece put to a terminal that ended mid-put (`probe-022`,
/// step E3): kind `finished`, naming the terminal.
const TRANSFER_FINISHED: &str = r#"{"code":-32000,"data":{"hint":"nothing there to read it","kind":"finished"},"message":"t3 has finished"}"#;

/// The error herdr-core 0.1.22 sent for a `terminal.put_finish` before all the bytes had arrived
/// (`probe-022`, step B9): a put begun for 3 bytes and finished with none sent.
const FINISH_TOO_EARLY: &str = r#"{"code":-32602,"data":{"hint":"see `hc schema`","kind":"invalid_params"},"message":"got 0 of 3 bytes"}"#;

/// A core over the fake, the handle on its terminal `t2`, and the peer.
async fn put_shell() -> Result<
    (
        HerdrCore<FakeTransport>,
        TerminalHandle<FakeTransport>,
        FakePeer,
    ),
    TestError,
> {
    let (transport, mut peer) = fake_pair();
    peer.reply_line(&hello_reply(&hello_result("0.1.22", true, 1)))?;
    let core = timeout(PATIENCE, HerdrCore::over(transport)).await??;
    let _hello = peer.line().await?;
    let shell = core.terminals().handle("t2")?;
    Ok((core, shell, peer))
}

/// Runs `put_in_pieces` of `data` while the peer answers each request in turn with the next of
/// `answers`, and returns the call's outcome and the requests in the order they arrived.
async fn put_in_pieces_answered(
    peer: &mut FakePeer,
    shell: &TerminalHandle<FakeTransport>,
    data: Vec<u8>,
    answers: Vec<Answer>,
) -> Result<(Result<PutResult, HerdrCoreError>, Vec<Value>), TestError> {
    let call = shell.put_in_pieces("a.bin", data);
    let serve = async {
        let mut seen = Vec::new();
        for answer in answers {
            let request = peer.request().await?;
            let id = request["id"].as_u64().ok_or("no id")?;
            let reply = match answer {
                Answer::Result(result) => {
                    format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
                }
                Answer::Refusal(error) => {
                    format!(r#"{{"error":{error},"id":{id},"jsonrpc":"2.0"}}"#)
                }
            };
            peer.push_data(reply.as_bytes())?;
            seen.push(request);
        }
        Ok::<_, TestError>(seen)
    };
    let (outcome, seen) = timeout(PATIENCE, async { tokio::join!(call, serve) }).await?;
    Ok((outcome, seen?))
}

/// The methods of `requests`, in order.
fn methods_of(requests: &[Value]) -> Vec<&str> {
    requests
        .iter()
        .filter_map(|request| request["method"].as_str())
        .collect()
}

/// The transfer the begin request of a put in pieces chose.
fn chosen_transfer(requests: &[Value]) -> Result<String, TestError> {
    let begin = requests.first().ok_or("no request was sent")?;
    assert_eq!(begin["method"], "terminal.put_begin");
    Ok(begin["params"]["transfer"]
        .as_str()
        .ok_or("no transfer in the begin request")?
        .to_owned())
}

/// How many bytes the base64 text `data` of a piece decodes to.
fn piece_len(data: &Value) -> Result<usize, TestError> {
    let text = data.as_str().ok_or("a piece has no data")?;
    Ok(base64::Engine::decode(&base64::engine::general_purpose::STANDARD, text)?.len())
}

#[tokio::test]
async fn put_in_pieces_splits_at_one_mebibyte() -> TestResult {
    for (len, pieces) in [(0_usize, 0_usize), (1, 1), (1_048_576, 1), (1_048_577, 2)] {
        let (_core, shell, mut peer) = put_shell().await?;
        let mut answers = vec![opened()];
        answers.extend((0..pieces).map(|_| held(0)));
        answers.push(stored());
        let (outcome, requests) =
            put_in_pieces_answered(&mut peer, &shell, vec![7; len], answers).await?;

        assert_eq!(outcome?.bytes, 5, "{len}");
        let mut expected = vec!["terminal.put_begin"];
        expected.extend((0..pieces).map(|_| "terminal.put_chunk"));
        expected.push("terminal.put_finish");
        assert_eq!(methods_of(&requests), expected, "{len}");

        let transfer = chosen_transfer(&requests)?;
        assert_eq!(transfer.len(), 32);
        assert_eq!(
            requests[0]["params"],
            json!({"bytes": len, "name": "a.bin", "terminal": "t2", "transfer": transfer})
        );
        let mut sent = 0;
        for (piece, request) in requests[1..=pieces].iter().enumerate() {
            assert_eq!(request["params"]["transfer"], transfer.as_str());
            assert_eq!(request["params"]["offset"], piece * 1_048_576, "{len}");
            sent += piece_len(&request["params"]["data"])?;
        }
        assert_eq!(sent, len);
        let finish = requests.last().ok_or("no request")?;
        assert_eq!(finish["params"], json!({"transfer": transfer}));
    }
    Ok(())
}

#[tokio::test]
async fn put_in_pieces_aborts_after_a_refused_piece() -> TestResult {
    let (_core, shell, mut peer) = put_shell().await?;
    let answers = vec![opened(), Answer::Refusal(TRANSFER_FINISHED), dropped()];
    let (outcome, requests) =
        put_in_pieces_answered(&mut peer, &shell, vec![1, 2, 3], answers).await?;

    let error = outcome.err().ok_or("the put succeeded")?;
    assert_eq!(
        error.refusal().map(|refusal| refusal.kind.to_string()),
        Some("finished".to_owned())
    );
    assert_eq!(
        methods_of(&requests),
        [
            "terminal.put_begin",
            "terminal.put_chunk",
            "terminal.put_abort"
        ]
    );
    let transfer = chosen_transfer(&requests)?;
    assert_eq!(requests[2]["params"], json!({"transfer": transfer}));
    peer.nothing_sent().await?;
    Ok(())
}

#[tokio::test]
async fn put_in_pieces_aborts_after_a_refused_finish() -> TestResult {
    let (_core, shell, mut peer) = put_shell().await?;
    let answers = vec![
        opened(),
        held(3),
        Answer::Refusal(FINISH_TOO_EARLY),
        dropped(),
    ];
    let (outcome, requests) =
        put_in_pieces_answered(&mut peer, &shell, vec![1, 2, 3], answers).await?;

    let error = outcome.err().ok_or("the put succeeded")?;
    assert_eq!(
        error.refusal().map(|refusal| refusal.kind.clone()),
        Some(ErrorKind::InvalidParams)
    );
    assert_eq!(
        methods_of(&requests),
        [
            "terminal.put_begin",
            "terminal.put_chunk",
            "terminal.put_finish",
            "terminal.put_abort"
        ]
    );
    let transfer = chosen_transfer(&requests)?;
    assert_eq!(requests[3]["params"], json!({"transfer": transfer}));
    Ok(())
}

#[tokio::test]
async fn put_in_pieces_sends_no_abort_when_begin_is_refused() -> TestResult {
    let (_core, shell, mut peer) = put_shell().await?;
    let answers = vec![Answer::Refusal(MACHINE_UNKNOWN)];
    let (outcome, requests) = put_in_pieces_answered(&mut peer, &shell, vec![1], answers).await?;

    let error = outcome.err().ok_or("the put succeeded")?;
    assert_eq!(
        error.refusal().map(|refusal| refusal.kind.clone()),
        Some(ErrorKind::MachineUnknown)
    );
    assert_eq!(methods_of(&requests), ["terminal.put_begin"]);
    peer.nothing_sent().await?;
    Ok(())
}

#[tokio::test]
async fn put_in_pieces_aborts_when_begin_does_not_decode() -> TestResult {
    let (_core, shell, mut peer) = put_shell().await?;
    let answers = vec![Answer::Result(r#"{"transfer":"x"}"#.to_owned()), dropped()];
    let (outcome, requests) = put_in_pieces_answered(&mut peer, &shell, vec![1], answers).await?;

    assert!(matches!(
        outcome,
        Err(HerdrCoreError::Call(error)) if matches!(*error, Error::Decode { .. })
    ));
    assert_eq!(
        methods_of(&requests),
        ["terminal.put_begin", "terminal.put_abort"]
    );
    let transfer = chosen_transfer(&requests)?;
    assert_eq!(requests[1]["params"], json!({"transfer": transfer}));
    Ok(())
}

#[tokio::test]
async fn put_in_pieces_ignores_a_failed_abort() -> TestResult {
    // herdr-core 0.1.22 answers every `terminal.put_abort` with `{}`, even for an unknown put, so
    // no recorded abort refusal exists; the refusal of the abort here is an arbitrary one.
    let (_core, shell, mut peer) = put_shell().await?;
    let answers = vec![
        opened(),
        Answer::Refusal(TRANSFER_FINISHED),
        Answer::Refusal(MACHINE_UNKNOWN),
    ];
    let (outcome, requests) = put_in_pieces_answered(&mut peer, &shell, vec![1], answers).await?;

    let error = outcome.err().ok_or("the put succeeded")?;
    assert_eq!(
        error.refusal().map(|refusal| refusal.kind.to_string()),
        Some("finished".to_owned())
    );
    assert_eq!(
        methods_of(&requests),
        [
            "terminal.put_begin",
            "terminal.put_chunk",
            "terminal.put_abort"
        ]
    );
    Ok(())
}

fn machine(name: &str) -> Result<MachineName, TestError> {
    Ok(MachineName::new(name)?)
}

/// The kind of the error object a refused call carries.
fn rpc_kind(error: Error) -> Result<(String, ErrorKind), TestError> {
    match error {
        Error::Rpc { method, error } => Ok((method.as_str().to_owned(), error.kind)),
        other => Err(format!("expected Error::Rpc, got {other}").into()),
    }
}

#[tokio::test]
async fn machine_exec_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let line = r"printf 'a\nb'; echo err >&2; exit 3";
    let exec = MachineExec::new(ExecCommand::Line(line.to_owned()));
    let ran = answered(
        &mut peer,
        client.machine_exec(exec),
        "machine.exec",
        json!({"line": line}),
        r#"{"code":3,"ms":15,"signal":null,"stderr":"ZXJyCg==","stdout":"YQpi","timed_out":false,"truncated":false}"#,
    )
    .await?;
    assert_eq!(ran.code, Some(3));
    assert_eq!(ran.stdout, b"a\nb");
    assert_eq!(ran.stderr, b"err\n");
    assert_eq!(ran.ms, 15);
    assert!(!ran.timed_out && !ran.truncated);
    Ok(())
}

#[tokio::test]
async fn machine_exec_decodes_base64_output() -> TestResult {
    let (client, mut peer) = connected().await?;
    let argv = Argv::new(["echo", "hi"].map(str::to_owned).to_vec())?;
    let ran = answered(
        &mut peer,
        client.machine_exec(MachineExec::new(ExecCommand::Argv(argv))),
        "machine.exec",
        json!({"argv": ["echo", "hi"]}),
        r#"{"code":0,"ms":6,"signal":null,"stderr":"","stdout":"aGkK","timed_out":false,"truncated":false}"#,
    )
    .await?;
    assert_eq!(ran.stdout, b"hi\n");
    assert_eq!(ran.code, Some(0));
    assert_eq!(ran.signal, None);
    Ok(())
}

#[tokio::test]
async fn invalid_base64_in_a_reply_is_decode_error() -> TestResult {
    let (client, mut peer) = connected().await?;
    let outcome = exchange(
        &mut peer,
        client.machine_exec(MachineExec::new(ExecCommand::Line("true".to_owned()))),
        "machine.exec",
        json!({"line": "true"}),
        |id| {
            format!(
                r#"{{"id":{id},"jsonrpc":"2.0","result":{{"code":0,"ms":6,"signal":null,"stderr":"","stdout":"@@","timed_out":false,"truncated":false}}}}"#
            )
        },
    )
    .await?;
    match outcome {
        Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "machine.exec"),
        _ => return Err("expected Error::Decode".into()),
    }
    Ok(())
}

#[tokio::test]
async fn file_read_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let path = "/tmp/hfycRyby/files/sub/a.txt";
    let read = answered(
        &mut peer,
        client.file_read(FileRead::new(RemotePath::new(path))),
        "file.read",
        json!({"path": path}),
        r#"{"first":1,"last":3,"lines":3,"text":"line1\nline2\nline3\n"}"#,
    )
    .await?;
    assert_eq!((read.first, read.last, read.lines), (1, 3, 3));
    assert_eq!(read.text, "line1\nline2\nline3\n");
    Ok(())
}

#[tokio::test]
async fn file_write_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let path = "/tmp/hfycRyby/files/sub/a.txt";
    let write = FileWrite::new(RemotePath::new(path), b"line1\nline2\nline3\n".to_vec());
    let written = answered(
        &mut peer,
        client.file_write(write),
        "file.write",
        json!({"data": "bGluZTEKbGluZTIKbGluZTMK", "path": path}),
        r#"{"bytes":18,"path":"/tmp/hfycRyby/files/sub/a.txt"}"#,
    )
    .await?;
    assert_eq!(written.bytes, 18);
    assert_eq!(written.path, path);
    Ok(())
}

#[tokio::test]
async fn file_edit_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let path = "/tmp/hfycRyby/files/sub/a.txt";
    let edit = Edit {
        old: "line2".to_owned(),
        new: "LINE-TWO".to_owned(),
    };
    let edited = answered(
        &mut peer,
        client.file_edit(FileEdit::new(RemotePath::new(path), vec![edit])),
        "file.edit",
        json!({"edits": [{"new": "LINE-TWO", "old": "line2"}], "path": path}),
        r#"{"bytes":21,"path":"/tmp/hfycRyby/files/sub/a.txt"}"#,
    )
    .await?;
    assert_eq!(edited.bytes, 21);
    assert_eq!(edited.path, path);
    Ok(())
}

#[tokio::test]
async fn doc_get_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let got = answered(
        &mut peer,
        client.doc_get(DocGet::new(DocName::new("probe")?)),
        "doc.get",
        json!({"name": "probe"}),
        r#"{"value":{"k":"v"},"version":1}"#,
    )
    .await?;
    assert_eq!(got.value, json!({"k": "v"}));
    assert_eq!(got.version, DocVersion(1));
    Ok(())
}

#[tokio::test]
async fn doc_set_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let set = DocSet::new(DocName::new("probe")?, json!({"k": "v"}), DocVersion(0));
    let stored = answered(
        &mut peer,
        client.doc_set(set),
        "doc.set",
        json!({"name": "probe", "value": {"k": "v"}, "version": 0}),
        r#"{"version":1}"#,
    )
    .await?;
    assert_eq!(stored.version, DocVersion(1));
    Ok(())
}

#[tokio::test]
async fn machine_add_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let target = MachineTarget::Via {
        name: machine("m1")?,
        command: "false".to_owned(),
    };
    let error = refused(
        &mut peer,
        client.machine_add(MachineAdd::new(target)),
        "machine.add",
        json!({"name": "m1", "via": "false"}),
        UNREACHABLE,
    )
    .await?;
    assert_eq!(
        rpc_kind(error)?,
        ("machine.add".to_owned(), ErrorKind::Unreachable)
    );
    Ok(())
}

#[tokio::test]
async fn machine_list_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let listed = answered(
        &mut peer,
        client.machine_list(MachineList::new()),
        "machine.list",
        json!({}),
        r#"{"machines":[]}"#,
    )
    .await?;
    assert!(listed.machines.is_empty());
    Ok(())
}

#[tokio::test]
async fn machine_connect_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let error = refused(
        &mut peer,
        client.machine_connect(MachineConnect::new(machine("nope")?)),
        "machine.connect",
        json!({"machine": "nope"}),
        MACHINE_UNKNOWN,
    )
    .await?;
    assert_eq!(
        rpc_kind(error)?,
        ("machine.connect".to_owned(), ErrorKind::MachineUnknown)
    );
    Ok(())
}

#[tokio::test]
async fn machine_rename_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let error = refused(
        &mut peer,
        client.machine_rename(MachineRename::new(machine("nope")?, machine("x")?)),
        "machine.rename",
        json!({"machine": "nope", "new_name": "x"}),
        MACHINE_UNKNOWN,
    )
    .await?;
    assert_eq!(
        rpc_kind(error)?,
        ("machine.rename".to_owned(), ErrorKind::MachineUnknown)
    );
    Ok(())
}

#[tokio::test]
async fn machine_rm_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let error = refused(
        &mut peer,
        client.machine_rm(MachineRm::new(machine("nope")?)),
        "machine.rm",
        json!({"machine": "nope"}),
        MACHINE_UNKNOWN,
    )
    .await?;
    assert_eq!(
        rpc_kind(error)?,
        ("machine.rm".to_owned(), ErrorKind::MachineUnknown)
    );
    Ok(())
}

/// Fails to compile unless `T` is `Send`.
const fn assert_send<T: Send>(_: &T) {}

/// The futures are built and checked, never awaited: a future that is not `Send` fails the build.
/// Building them needs a connected client, so the test runs inside a runtime.
#[tokio::test]
async fn client_and_its_futures_are_send() -> TestResult {
    let (transport, _unconnected) = fake_pair();
    assert_send(&Client::connect(transport));

    let (client, _peer) = connected().await?;
    assert_send(&client);
    assert_send(&client.call_raw(method("x")?, json!({})));

    assert_send(&client.server_status(ServerStatus::new()));
    assert_send(&client.server_stop(ServerStop::new()));
    assert_send(&client.server_restart(ServerRestart::new()));
    assert_send(&client.templates_list(TemplatesList::new()));
    assert_send(&client.rpc_discover(RpcDiscover::new()));

    let argv = Argv::new(vec!["sh".to_owned()])?;
    assert_send(&client.terminal_run(TerminalRun::new(Command::Argv(argv))));
    assert_send(&client.terminal_list(TerminalList::new()));
    assert_send(&client.terminal_get(TerminalGet::new(terminal("t1")?)));
    assert_send(&client.terminal_kill(TerminalKill::new(terminal("t1")?)));
    assert_send(&client.terminal_rm(TerminalRm::new(terminal("t1")?)));
    assert_send(&client.terminal_keep(TerminalKeep::new(terminal("t1")?, Keep::Off)));
    assert_send(&client.terminal_label(TerminalLabel::new(terminal("t1")?, LabelChanges::new())));
    assert_send(&client.terminal_read(TerminalRead::new(terminal("t1")?)));
    assert_send(&client.terminal_send(TerminalSend::new(terminal("t1")?, String::new())));
    let target = WaitTarget::Terminal(terminal("t1")?);
    assert_send(&client.terminal_wait(TerminalWait::new(target)));
    assert_send(&client.terminal_keys(TerminalKeys::new(terminal("t1")?, Vec::new())));
    assert_send(&client.terminal_interrupt(TerminalInterrupt::new(terminal("t1")?)));
    let put = TerminalPut::new(terminal("t1")?, FileName::new("x"), Vec::new());
    assert_send(&client.terminal_put(put));
    assert_send(&client.terminal_put_begin(TerminalPutBegin::new(
        terminal("t1")?,
        FileName::new("x"),
        0,
        transfer()?,
    )));
    assert_send(&client.terminal_put_chunk(TerminalPutChunk::new(transfer()?, 0, Vec::new())));
    assert_send(&client.terminal_put_finish(TerminalPutFinish::new(transfer()?)));
    assert_send(&client.terminal_put_abort(TerminalPutAbort::new(transfer()?)));
    let (_core, shell, _core_peer) = put_shell().await?;
    assert_send(&shell.put_in_pieces("x", Vec::new()));

    let exec = MachineExec::new(ExecCommand::Line(String::new()));
    assert_send(&client.machine_exec(exec));
    assert_send(&client.file_read(FileRead::new(RemotePath::new("/f"))));
    assert_send(&client.file_write(FileWrite::new(RemotePath::new("/f"), Vec::new())));
    assert_send(&client.file_edit(FileEdit::new(RemotePath::new("/f"), Vec::new())));
    assert_send(&client.doc_get(DocGet::new(DocName::new("d")?)));
    let set = DocSet::new(DocName::new("d")?, Value::Null, DocVersion(0));
    assert_send(&client.doc_set(set));
    let target = MachineTarget::Ssh {
        target: "host".to_owned(),
        name: None,
    };
    assert_send(&client.machine_add(MachineAdd::new(target)));
    assert_send(&client.machine_list(MachineList::new()));
    assert_send(&client.machine_connect(MachineConnect::new(machine("m")?)));
    assert_send(&client.machine_rename(MachineRename::new(machine("m")?, machine("n")?)));
    assert_send(&client.machine_rm(MachineRm::new(machine("m")?)));
    Ok(())
}

// ---- Event subscriptions ----

/// The result of `events.subscribe` as recorded from herdr-core 0.1.21 (`probe-methods`, step M5),
/// with the working directory replaced by a neutral path.
const SUBSCRIBED: &str = r#"{"channel":1,"seq":12,"terminals":[{"address":"t1","argv":["sh","-c","sleep 5"],"cols":200,"created_ms":1795356727649,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":0,"finished_ms":1795356732694,"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"parent":null,"pid":56322,"reason":"exited","reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"finished","status":null,"status_ms":null,"template":null,"title":"","turn":0},{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":1795356754016,"identity":"ghostty","keep":false,"labels":{"b":"2"},"name":null,"notify":true,"parent":null,"pid":56464,"reason":"killed","reported":false,"rm":false,"rows":120,"screen":"exact","signal":1,"state":"finished","status":null,"status_ms":null,"template":null,"title":"","turn":0}]}"#;

/// The channel the tests' subscriptions run on, as the recorded reply names it.
const EVENT_CHANNEL: u32 = 1;

fn stream_frame(channel: u32, kind: FrameKind, payload: &[u8]) -> Frame {
    Frame {
        channel: Channel::new(channel),
        kind,
        payload: payload.to_vec(),
    }
}

/// Sends `bytes` to the client as one DATA frame on the event channel.
fn event_data(peer: &FakePeer, bytes: &[u8]) -> TestResult {
    peer.push_frame(stream_frame(EVENT_CHANNEL, FrameKind::Data, bytes))
}

/// A `title` event line, newline included.
fn title_line(seq: u64, title: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"event","params":{{"data":{{"title":"{title}"}},"seq":{seq},"terminal":"t1","type":"title"}}}}"#
    ) + "\n"
}

/// Subscribes to every terminal while the peer answers with `result`.
async fn subscribe(
    client: &Client<FakeTransport>,
    peer: &mut FakePeer,
    result: &Value,
) -> Result<Result<Subscription, Error>, TestError> {
    let reply = |id| json!({"id": id, "jsonrpc": "2.0", "result": result}).to_string();
    exchange(
        peer,
        client.events_subscribe(EventsSubscribe::new()),
        "events.subscribe",
        json!({}),
        reply,
    )
    .await
}

/// A client with a subscription on channel 1 to a core that listed no terminals.
async fn subscribed() -> Result<(Client<FakeTransport>, Subscription, FakePeer), TestError> {
    let (client, mut peer) = connected().await?;
    let result = json!({"channel": EVENT_CHANNEL, "seq": 12, "terminals": []});
    let subscription = subscribe(&client, &mut peer, &result).await??;
    Ok((client, subscription, peer))
}

/// The next item of `subscription`, bounded.
async fn next(
    subscription: &mut Subscription,
) -> Result<Option<Result<Notification, Error>>, TestError> {
    Ok(timeout(PATIENCE, subscription.next()).await?)
}

/// The title of the next item, which must be a `title` event.
async fn next_title(subscription: &mut Subscription) -> Result<String, TestError> {
    match next(subscription).await? {
        Some(Ok(Notification::Event(event))) => match event.kind {
            EventKind::Title { title } => Ok(title),
            other => Err(format!("expected a title event, got {other:?}").into()),
        },
        other => Err(format!("expected an event, got {other:?}").into()),
    }
}

/// The error of the next item, which must be an error.
async fn next_error(subscription: &mut Subscription) -> Result<Error, TestError> {
    match next(subscription).await? {
        Some(Err(error)) => Ok(error),
        other => Err(format!("expected an error, got {other:?}").into()),
    }
}

/// Reads CREDIT frames from the peer until they add up to `total` bytes.
async fn credited(peer: &mut FakePeer, channel: u32, total: u64) -> TestResult {
    let mut sum = 0_u64;
    while sum < total {
        let frame = peer.frame().await?;
        assert_eq!(frame.kind, FrameKind::Credit);
        assert_eq!(frame.channel, Channel::new(channel));
        sum += u64::from(frame.read_credit()?);
    }
    assert_eq!(sum, total);
    Ok(())
}

#[tokio::test]
async fn events_subscribe_request_and_reply() -> TestResult {
    let (client, mut peer) = connected().await?;
    let subscription = answered(
        &mut peer,
        client.events_subscribe(EventsSubscribe::new()),
        "events.subscribe",
        json!({}),
        SUBSCRIBED,
    )
    .await?;
    assert_eq!(subscription.seq(), Seq(12));
    assert_eq!(subscription.channel(), Channel::new(1));
    let addresses: Vec<&str> = subscription
        .terminals()
        .iter()
        .map(|terminal| terminal.address.as_str())
        .collect();
    assert_eq!(addresses, ["t1", "t2"]);
    Ok(())
}

#[tokio::test]
async fn non_u32_channel_is_decode_error() -> TestResult {
    let (client, mut peer) = connected().await?;
    for channel in [json!("x"), json!(-1), json!(4_294_967_296_u64)] {
        let result = json!({"channel": channel, "seq": 1, "terminals": []});
        match subscribe(&client, &mut peer, &result).await? {
            Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "events.subscribe"),
            other => return Err(format!("{channel}: expected Error::Decode, got {other:?}").into()),
        }
    }
    peer.nothing_sent().await?;

    let call = client.call_raw(method("x")?, json!({}));
    let serve = async {
        let request = peer.request().await?;
        peer.reply(request["id"].as_u64().ok_or("no id")?, &json!(true))?;
        Ok::<(), TestError>(())
    };
    let (outcome, served) = timeout(PATIENCE, async { tokio::join!(call, serve) }).await?;
    served?;
    assert_eq!(outcome?, json!(true));
    Ok(())
}

#[tokio::test]
async fn channel_zero_is_refused_and_opens_nothing() -> TestResult {
    let (client, mut peer) = connected().await?;
    let result = json!({"channel": 0, "seq": 1, "terminals": []});
    match subscribe(&client, &mut peer, &result).await? {
        Err(error @ Error::Decode { .. }) => assert_eq!(
            error.to_string(),
            "cannot decode the events.subscribe result: channel 0 carries calls, not streams"
        ),
        other => return Err(format!("expected Error::Decode, got {other:?}").into()),
    }
    // A CLOSE on channel 0 would be sent here if the client had opened a stream on it.
    peer.nothing_sent().await?;
    answered_call(&client, &mut peer).await
}

#[tokio::test]
async fn a_channel_in_use_is_refused_and_the_first_subscription_keeps_working() -> TestResult {
    let (client, mut first, mut peer) = subscribed().await?;
    let result = json!({"channel": EVENT_CHANNEL, "seq": 2, "terminals": []});
    match subscribe(&client, &mut peer, &result).await? {
        Err(error @ Error::Decode { .. }) => assert_eq!(
            error.to_string(),
            "cannot decode the events.subscribe result: channel 1 is already open for another stream"
        ),
        other => return Err(format!("expected Error::Decode, got {other:?}").into()),
    }
    peer.nothing_sent().await?;

    event_data(&peer, title_line(1, "still mine").as_bytes())?;
    assert_eq!(next_title(&mut first).await?, "still mine");
    assert!(client.close_reason().is_none());

    credited(
        &mut peer,
        EVENT_CHANNEL,
        u64::try_from(title_line(1, "still mine").len())?,
    )
    .await?;
    drop(first);
    assert_eq!(peer.frame().await?, event_close());
    peer.nothing_sent().await
}

/// The error `events_subscribe` returns for `docs` on a core that reports `version`, with the
/// connection ended first when `ended`. Fails if the client sent anything.
async fn docs_refusal(version: &str, ended: bool) -> Result<Error, TestError> {
    let (client, mut peer) = connected_at(version).await?;
    if ended {
        peer.end();
        wait_ended(&client).await?;
    }
    let params = EventsSubscribe::new().docs([DocName::new("probe")?]);
    let outcome = timeout(PATIENCE, client.events_subscribe(params)).await?;
    peer.nothing_sent().await?;
    outcome.err().ok_or_else(|| "subscribe succeeded".into())
}

/// Asserts that `error` is `NeedsNewerCore` for the docs filter on a core that reported `found`.
fn assert_needs_docs_core(error: &Error, found: &str) -> TestResult {
    match error {
        Error::NeedsNewerCore {
            option,
            needs,
            found: seen,
        } => {
            assert_eq!(*option, "events.subscribe docs");
            assert_eq!(*needs, CoreVersion::parse("0.1.22")?);
            assert_eq!(*seen, CoreVersion::parse(found)?);
            Ok(())
        }
        other => Err(format!("expected NeedsNewerCore, got {other:?}").into()),
    }
}

#[tokio::test]
async fn docs_on_0_1_21_is_refused_before_sending() -> TestResult {
    let error = docs_refusal("0.1.21", false).await?;
    assert_needs_docs_core(&error, "0.1.21")?;
    assert_eq!(
        error.to_string(),
        "events.subscribe docs needs herdr-core 0.1.22 or newer; this core is 0.1.21"
    );
    Ok(())
}

#[tokio::test]
async fn docs_on_a_0_1_22_prerelease_is_refused() -> TestResult {
    let error = docs_refusal("0.1.22-rc.1", false).await?;
    assert_needs_docs_core(&error, "0.1.22-rc.1")
}

#[tokio::test]
async fn docs_on_0_1_22_is_sent() -> TestResult {
    let (client, mut peer) = connected_at("0.1.22").await?;
    let result = json!({"channel": EVENT_CHANNEL, "seq": 1, "terminals": []});
    let reply = |id| json!({"id": id, "jsonrpc": "2.0", "result": result}).to_string();
    let params = EventsSubscribe::new().docs([DocName::new("probe")?]);
    let _subscription = exchange(
        &mut peer,
        client.events_subscribe(params),
        "events.subscribe",
        json!({"docs": ["probe"]}),
        reply,
    )
    .await??;
    Ok(())
}

#[tokio::test]
async fn subscribe_without_docs_on_0_1_21_is_sent() -> TestResult {
    let (client, mut peer) = connected_at("0.1.21").await?;
    let result = json!({"channel": EVENT_CHANNEL, "seq": 1, "terminals": []});
    let _subscription = subscribe(&client, &mut peer, &result).await??;
    Ok(())
}

#[tokio::test]
async fn docs_guard_runs_before_the_closed_check() -> TestResult {
    let error = docs_refusal("0.1.21", true).await?;
    assert_needs_docs_core(&error, "0.1.21")
}

#[tokio::test]
async fn reply_without_channel_is_missing_channel() -> TestResult {
    let (client, mut peer) = connected().await?;
    let result = json!({"seq": 1, "terminals": []});
    match subscribe(&client, &mut peer, &result).await? {
        Err(Error::Protocol(error @ ProtocolError::MissingChannel { .. })) => assert_eq!(
            error.to_string(),
            "the events.subscribe reply has no channel on a framed connection"
        ),
        other => return Err(format!("expected MissingChannel, got {other:?}").into()),
    }
    peer.nothing_sent().await
}

#[tokio::test]
async fn first_frame_after_the_reply_is_delivered() -> TestResult {
    let (client, mut peer) = connected().await?;
    let call = client.events_subscribe(EventsSubscribe::new());
    let serve = async {
        let request = peer.request().await?;
        let id = request["id"].as_u64().ok_or("no id")?;
        peer.reply(
            id,
            &json!({"channel": EVENT_CHANNEL, "seq": 1, "terminals": []}),
        )?;
        event_data(&peer, title_line(2, "first").as_bytes())?;
        Ok::<(), TestError>(())
    };
    let (subscription, served) = timeout(PATIENCE, async { tokio::join!(call, serve) }).await?;
    served?;
    let mut subscription = subscription?;
    assert_eq!(next_title(&mut subscription).await?, "first");
    Ok(())
}

#[tokio::test]
async fn events_split_and_packed_lines() -> TestResult {
    let (_client, mut subscription, mut peer) = subscribed().await?;
    let split = title_line(1, "split");
    let (head, tail) = split.as_bytes().split_at(30);
    let packed = [title_line(2, "a"), title_line(3, "b"), title_line(4, "c")].concat();
    event_data(&peer, head)?;
    event_data(&peer, tail)?;
    event_data(&peer, packed.as_bytes())?;

    for title in ["split", "a", "b", "c"] {
        assert_eq!(next_title(&mut subscription).await?, title);
    }
    let sent = u64::try_from(split.len() + packed.len())?;
    credited(&mut peer, EVENT_CHANNEL, sent).await?;
    peer.nothing_sent().await
}

#[tokio::test]
async fn missed_drops_stale_events() -> TestResult {
    let (_client, mut subscription, peer) = subscribed().await?;
    let lines = [
        r#"{"jsonrpc":"2.0","method":"missed","params":{"seq":10,"terminals":[]}}"#.to_owned()
            + "\n",
        title_line(9, "stale"),
        title_line(10, "stale too"),
        title_line(11, "fresh"),
    ]
    .concat();
    event_data(&peer, lines.as_bytes())?;

    match next(&mut subscription).await? {
        Some(Ok(Notification::Missed { seq, terminals })) => {
            assert_eq!(seq, Seq(10));
            assert!(terminals.is_empty());
        }
        other => return Err(format!("expected missed, got {other:?}").into()),
    }
    assert_eq!(next_title(&mut subscription).await?, "fresh");
    Ok(())
}

#[tokio::test]
async fn bad_lines_do_not_end_the_stream() -> TestResult {
    let (_client, mut subscription, peer) = subscribed().await?;
    let lines = [
        "not json\n".to_owned(),
        "[1,2]\n".to_owned(),
        r#"{"id":9,"jsonrpc":"2.0","result":{}}"#.to_owned() + "\n",
        r#"{"jsonrpc":"2.0","method":"heartbeat","params":{}}"#.to_owned() + "\n",
        r#"{"jsonrpc":"2.0","method":"event","params":{"data":5,"seq":3,"terminal":"t1","type":"title"}}"#
            .to_owned()
            + "\n",
        title_line(4, "still here"),
    ]
    .concat();
    event_data(&peer, lines.as_bytes())?;

    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Protocol(ProtocolError::InvalidJson { .. })
    ));
    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Protocol(ProtocolError::NotJsonRpc)
    ));
    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Protocol(ProtocolError::ResponseOnStream)
    ));
    match next_error(&mut subscription).await? {
        Error::Protocol(ProtocolError::InvalidNotification { method, .. }) => {
            assert_eq!(method, "heartbeat");
        }
        other => return Err(format!("expected InvalidNotification, got {other:?}").into()),
    }
    match next_error(&mut subscription).await? {
        Error::Protocol(ProtocolError::InvalidEvent { kind, .. }) => assert_eq!(kind, "title"),
        other => return Err(format!("expected InvalidEvent, got {other:?}").into()),
    }
    assert_eq!(next_title(&mut subscription).await?, "still here");
    Ok(())
}

#[tokio::test]
async fn over_long_line_ends_the_stream() -> TestResult {
    let (_client, mut subscription, peer) = subscribed().await?;
    event_data(&peer, &vec![b'x'; 16_777_216])?;
    event_data(&peer, b"x")?;

    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Protocol(ProtocolError::EventLineTooLong)
    ));
    assert!(next(&mut subscription).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn core_close_ends_the_stream() -> TestResult {
    for payload in [&b""[..], &br#"{"reason":"exited"}"#[..]] {
        let (_client, mut subscription, peer) = subscribed().await?;
        event_data(&peer, title_line(1, "before").as_bytes())?;
        peer.push_frame(stream_frame(EVENT_CHANNEL, FrameKind::Close, payload))?;

        assert_eq!(next_title(&mut subscription).await?, "before");
        assert!(next(&mut subscription).await?.is_none());
        assert!(next(&mut subscription).await?.is_none(), "it stays ended");
    }
    Ok(())
}

#[tokio::test]
async fn resize_on_the_event_channel_is_discarded() -> TestResult {
    let (_client, mut subscription, mut peer) = subscribed().await?;
    let first = title_line(1, "one");
    let second = title_line(2, "two");
    event_data(&peer, first.as_bytes())?;
    peer.push_frame(Frame::resize(Channel::new(EVENT_CHANNEL), 80, 24))?;
    event_data(&peer, second.as_bytes())?;

    assert_eq!(next_title(&mut subscription).await?, "one");
    assert_eq!(next_title(&mut subscription).await?, "two");
    credited(
        &mut peer,
        EVENT_CHANNEL,
        u64::try_from(first.len() + second.len())?,
    )
    .await?;
    peer.nothing_sent().await
}

/// An empty CLOSE on the event channel, as the core or the client sends it.
fn event_close() -> Frame {
    stream_frame(EVENT_CHANNEL, FrameKind::Close, b"")
}

/// Runs one call to completion. The router handles frames in order, so once it resolves, every
/// frame the peer sent before answering it has been routed.
async fn answered_call(client: &Client<FakeTransport>, peer: &mut FakePeer) -> TestResult {
    let call = client.call_raw(method("sync")?, json!({}));
    let serve = async {
        let request = peer.request().await?;
        peer.reply(request["id"].as_u64().ok_or("no id")?, &json!(true))?;
        Ok::<(), TestError>(())
    };
    let (outcome, served) = timeout(PATIENCE, async { tokio::join!(call, serve) }).await?;
    served?;
    outcome?;
    Ok(())
}

#[tokio::test]
async fn close_twice_sends_one_close() -> TestResult {
    let (_client, mut subscription, mut peer) = subscribed().await?;
    subscription.close().await?;
    subscription.close().await?;
    drop(subscription);

    assert_eq!(peer.frame().await?, event_close());
    peer.nothing_sent().await
}

#[tokio::test]
async fn close_after_core_close_sends_nothing() -> TestResult {
    let (_client, mut subscription, mut peer) = subscribed().await?;
    peer.push_frame(event_close())?;
    assert!(next(&mut subscription).await?.is_none());

    subscription.close().await?;
    drop(subscription);
    peer.nothing_sent().await
}

#[tokio::test]
async fn queued_core_close_suppresses_drop_close() -> TestResult {
    let (client, subscription, mut peer) = subscribed().await?;
    peer.push_frame(event_close())?;
    answered_call(&client, &mut peer).await?;

    drop(subscription);
    peer.nothing_sent().await
}

#[tokio::test]
async fn close_after_connection_end_is_closed() -> TestResult {
    let (client, mut subscription, mut peer) = subscribed().await?;
    peer.end();
    wait_ended(&client).await?;

    assert!(matches!(subscription.close().await, Err(Error::Closed)));
    assert!(matches!(subscription.close().await, Err(Error::Closed)));
    Ok(())
}

#[tokio::test]
async fn close_stops_waiting_when_the_connection_ends() -> TestResult {
    let (client, mut subscription, mut peer) = subscribed().await?;
    peer.stall_next_write();
    let closing = tokio::spawn(async move { subscription.close().await });
    tokio::time::sleep(Duration::from_millis(100)).await;

    peer.end();
    wait_ended(&client).await?;
    let closed = timeout(PATIENCE, closing).await??;
    assert!(matches!(closed, Err(Error::Closed)));
    Ok(())
}

#[tokio::test]
async fn next_after_close_yields_until_core_close() -> TestResult {
    let (_client, mut subscription, mut peer) = subscribed().await?;
    event_data(&peer, title_line(1, "late").as_bytes())?;
    subscription.close().await?;
    assert_eq!(peer.frame().await?, event_close());

    assert_eq!(next_title(&mut subscription).await?, "late");
    peer.push_frame(event_close())?;
    assert!(next(&mut subscription).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn decode_failure_closes_the_orphaned_route() -> TestResult {
    let (client, mut peer) = connected().await?;
    let result = json!({"channel": EVENT_CHANNEL, "seq": 1, "terminals": 5});
    match subscribe(&client, &mut peer, &result).await? {
        Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "events.subscribe"),
        other => return Err(format!("expected Error::Decode, got {other:?}").into()),
    }
    // The reply was the peer's last word: the CLOSE it gets back is the client's own.
    assert_eq!(peer.frame().await?, event_close());
    Ok(())
}

#[tokio::test]
async fn dropped_subscribe_future_closes_the_route() -> TestResult {
    let (client, mut peer) = connected().await?;
    let mut subscribe = Box::pin(client.events_subscribe(EventsSubscribe::new()));
    assert!(
        timeout(Duration::from_millis(50), subscribe.as_mut())
            .await
            .is_err(),
        "the core has not replied"
    );
    let request = peer.request().await?;
    let reply = json!({"channel": EVENT_CHANNEL, "seq": 1, "terminals": []});
    peer.reply(request["id"].as_u64().ok_or("no id")?, &reply)?;
    answered_call(&client, &mut peer).await?;

    drop(subscribe);
    assert_eq!(peer.frame().await?, event_close());
    Ok(())
}

#[tokio::test]
async fn subscribe_future_dropped_before_reply_still_closes() -> TestResult {
    let (client, mut peer) = connected().await?;
    let mut subscribe = Box::pin(client.events_subscribe(EventsSubscribe::new()));
    assert!(
        timeout(Duration::from_millis(50), subscribe.as_mut())
            .await
            .is_err(),
        "the core has not replied"
    );
    let request = peer.request().await?;
    drop(subscribe);

    let reply = json!({"channel": EVENT_CHANNEL, "seq": 1, "terminals": []});
    peer.reply(request["id"].as_u64().ok_or("no id")?, &reply)?;
    assert_eq!(peer.frame().await?, event_close());
    Ok(())
}

#[tokio::test]
async fn connection_stays_open_while_a_subscription_lives() -> TestResult {
    let (client, mut subscription, mut peer) = subscribed().await?;
    let clone = client.clone();
    drop(client);
    drop(clone);
    peer.nothing_sent().await?;

    event_data(&peer, title_line(1, "still open").as_bytes())?;
    assert_eq!(next_title(&mut subscription).await?, "still open");
    credited(
        &mut peer,
        EVENT_CHANNEL,
        u64::try_from(title_line(1, "still open").len())?,
    )
    .await?;
    peer.nothing_sent().await?;

    drop(subscription);
    peer.sink_closed().await
}

#[tokio::test]
async fn subscription_sees_connection_loss() -> TestResult {
    // EOF.
    let (client, mut subscription, mut peer) = subscribed().await?;
    peer.end();
    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Closed
    ));
    assert!(next(&mut subscription).await?.is_none());
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::PeerClosed)
    );

    // A write that fails: a call made while the next write is set to fail.
    let (client, mut subscription, peer) = subscribed().await?;
    peer.fail_next_write();
    let refused = timeout(PATIENCE, client.call_raw(method("x")?, json!({}))).await?;
    assert!(matches!(
        refused,
        Err(Error::Transport(TransportError::Io(_)))
    ));
    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Closed
    ));
    assert!(next(&mut subscription).await?.is_none());
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::TransportFailed)
    );

    // A frame the source reports as too large.
    let (client, mut subscription, peer) = subscribed().await?;
    peer.fail_source(TransportError::Frame(FrameError::TooLarge {
        len: 16_777_217,
    }))?;
    assert!(matches!(
        next_error(&mut subscription).await?,
        Error::Closed
    ));
    assert!(next(&mut subscription).await?.is_none());
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::ProtocolViolation)
    );
    Ok(())
}

#[tokio::test]
async fn call_raw_on_events_subscribe_opens_no_stream() -> TestResult {
    let (client, mut peer) = connected().await?;
    let reply = |id| {
        json!({"id": id, "jsonrpc": "2.0",
               "result": {"channel": EVENT_CHANNEL, "seq": 1, "terminals": []}})
        .to_string()
    };
    let result = exchange(
        &mut peer,
        client.call_raw(method("events.subscribe")?, json!({})),
        "events.subscribe",
        json!({}),
        reply,
    )
    .await??;
    assert_eq!(result["channel"], EVENT_CHANNEL);

    event_data(&peer, title_line(2, "unheard").as_bytes())?;
    answered_call(&client, &mut peer).await?;
    peer.nothing_sent().await
}

#[tokio::test]
async fn subscription_and_its_futures_are_send() -> TestResult {
    let (client, mut subscription, _peer) = subscribed().await?;
    assert_send(&subscription);
    assert_send(&client.events_subscribe(EventsSubscribe::new()));
    assert_send(&subscription.next());
    assert_send(&subscription.close());
    Ok(())
}

// ---- Attach streams ----

/// The result of `terminal.attach` as recorded from herdr-core 0.1.21 (`probe-framed`, step B4),
/// with the working directory replaced by the neutral `/work/probe-framed`.
const ATTACHED: &str = r#"{"automatic_input":true,"channel":1,"resize_notifications":false,"terminal":{"address":"t1d","argv":["sh"],"cols":80,"created_ms":1795355450242,"cwd":"/work/probe-framed","die_with_parent":false,"exit_code":null,"finished_ms":null,"foreground":{"argv":["sh"],"cwd":"/work/probe-framed","name":"bash","pid":43803},"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"parent":null,"pid":43803,"reason":null,"reported":false,"rm":false,"rows":24,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}}"#;

/// The core's CLOSE payload when the program exited, as recorded in the same run.
const ENDED_CLOSE: &str =
    r#"{"exit_code":0,"finished_ms":1795355456589,"reason":"exited","signal":null}"#;

/// The channel the tests' attach streams run on, as the recorded reply names it.
const ATTACH_CHANNEL: u32 = 1;

/// A client connected over the fake to a core whose hello lists `features`.
async fn connected_with(features: &[&str]) -> Result<(Client<FakeTransport>, FakePeer), TestError> {
    let (transport, mut peer) = fake_pair();
    let mut result = hello_result("0.1.21", true, 1);
    result["features"] = json!(features);
    peer.reply_line(&hello_reply(&result))?;
    let client = timeout(PATIENCE, Client::connect(transport)).await??;
    let _hello = peer.line().await?;
    Ok((client, peer))
}

/// Attaches to `t1d` at 80 by 24 while the peer answers with `result`.
async fn attach(
    client: &Client<FakeTransport>,
    peer: &mut FakePeer,
    result: &Value,
) -> Result<Result<AttachStream, Error>, TestError> {
    let reply = |id| json!({"id": id, "jsonrpc": "2.0", "result": result}).to_string();
    exchange(
        peer,
        client.terminal_attach(TerminalAttach::new(terminal("t1d")?, 80, 24)),
        "terminal.attach",
        json!({"cols": 80, "rows": 24, "terminal": "t1d"}),
        reply,
    )
    .await
}

/// The recorded attach reply with `channel` and `automatic_input` replaced.
fn attached_result(automatic_input: bool) -> Result<Value, TestError> {
    let mut result: Value = serde_json::from_str(ATTACHED)?;
    result["automatic_input"] = json!(automatic_input);
    Ok(result)
}

/// A client whose hello lists `automatic_input`, attached to `t1d` on channel 1.
async fn attached() -> Result<(Client<FakeTransport>, AttachStream, FakePeer), TestError> {
    let (client, mut peer) = connected_with(&["framed", "automatic_input"]).await?;
    let stream = attach(&client, &mut peer, &attached_result(true)?).await??;
    Ok((client, stream, peer))
}

/// Sends `payload` to the client as one frame of `kind` on the attach channel.
fn attach_frame(peer: &FakePeer, kind: FrameKind, payload: &[u8]) -> TestResult {
    peer.push_frame(stream_frame(ATTACH_CHANNEL, kind, payload))
}

/// An empty CLOSE on the attach channel, as the core or the client sends it.
fn attach_close() -> Frame {
    stream_frame(ATTACH_CHANNEL, FrameKind::Close, b"")
}

/// The next item of `stream`, bounded.
async fn next_attach(
    stream: &mut AttachStream,
) -> Result<Option<Result<AttachEvent, Error>>, TestError> {
    Ok(timeout(PATIENCE, stream.next()).await?)
}

/// The next item, which must be an event.
async fn next_event(stream: &mut AttachStream) -> Result<AttachEvent, TestError> {
    match next_attach(stream).await? {
        Some(Ok(event)) => Ok(event),
        other => Err(format!("expected an attach event, got {other:?}").into()),
    }
}

/// The error of the next item, which must be an error.
async fn next_attach_error(stream: &mut AttachStream) -> Result<Error, TestError> {
    match next_attach(stream).await? {
        Some(Err(error)) => Ok(error),
        other => Err(format!("expected an error, got {other:?}").into()),
    }
}

#[tokio::test]
async fn attach_reply_fields() -> TestResult {
    let (client, mut peer) = connected().await?;
    let stream = attach(&client, &mut peer, &serde_json::from_str(ATTACHED)?).await??;
    assert_eq!(stream.terminal.address.as_str(), "t1d");
    assert_eq!(stream.channel, Channel::new(ATTACH_CHANNEL));
    assert!(stream.automatic_input);
    assert!(!stream.resize_notifications);
    Ok(())
}

#[tokio::test]
async fn output_is_credited() -> TestResult {
    let (_client, mut stream, mut peer) = attached().await?;
    let bytes: Vec<u8> = (0..339_u16)
        .map(|n| u8::try_from(n % 251).unwrap_or(0))
        .collect();
    attach_frame(&peer, FrameKind::Data, &bytes)?;
    assert_eq!(next_event(&mut stream).await?, AttachEvent::Output(bytes));
    credited(&mut peer, ATTACH_CHANNEL, 339).await?;
    peer.nothing_sent().await
}

#[tokio::test]
async fn zero_length_data_is_skipped() -> TestResult {
    let (_client, mut stream, mut peer) = attached().await?;
    attach_frame(&peer, FrameKind::Data, b"")?;
    attach_frame(&peer, FrameKind::Data, b"abc")?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Output(b"abc".to_vec())
    );
    credited(&mut peer, ATTACH_CHANNEL, 3).await?;
    peer.nothing_sent().await
}

#[tokio::test]
async fn resize_events_and_a_malformed_resize() -> TestResult {
    let (_client, mut stream, peer) = attached().await?;
    attach_frame(&peer, FrameKind::Resize, &[80, 0, 24, 0])?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Resize { cols: 80, rows: 24 }
    );

    attach_frame(&peer, FrameKind::Resize, &[80, 0, 24])?;
    attach_frame(&peer, FrameKind::Data, b"after")?;
    match next_attach_error(&mut stream).await? {
        Error::Protocol(ProtocolError::Frame(FrameError::BadControlPayload {
            kind: FrameKind::Resize,
            expected: 4,
            got: 3,
        })) => {}
        other => return Err(format!("expected BadControlPayload, got {other:?}").into()),
    }
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Output(b"after".to_vec())
    );
    Ok(())
}

#[tokio::test]
async fn close_reasons() -> TestResult {
    // An empty CLOSE.
    let (_client, mut stream, peer) = attached().await?;
    peer.push_frame(attach_close())?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Closed(CloseReason::Detached)
    );
    assert!(next_attach(&mut stream).await?.is_none());

    // The program ended.
    let (_client, mut stream, peer) = attached().await?;
    attach_frame(&peer, FrameKind::Close, ENDED_CLOSE.as_bytes())?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Closed(CloseReason::Ended {
            reason: EndReason::Exited,
            exit_code: Some(0),
            signal: None,
            finished_ms: Some(1_795_355_456_589),
        })
    );
    assert!(next_attach(&mut stream).await?.is_none());

    // Anything else.
    let (_client, mut stream, peer) = attached().await?;
    attach_frame(&peer, FrameKind::Close, b"garbage")?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Closed(CloseReason::Unrecognized {
            payload: b"garbage".to_vec()
        })
    );
    assert!(next_attach(&mut stream).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn non_u32_attach_channel_is_decode_error() -> TestResult {
    let (client, mut peer) = connected().await?;
    for channel in [json!("x"), json!(-1), json!(4_294_967_296_u64)] {
        let mut result: Value = serde_json::from_str(ATTACHED)?;
        result["channel"] = channel.clone();
        match attach(&client, &mut peer, &result).await? {
            Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "terminal.attach"),
            other => return Err(format!("{channel}: expected Error::Decode, got {other:?}").into()),
        }
    }
    // A CLOSE would be sent here if the client had opened a stream.
    peer.nothing_sent().await?;
    answered_call(&client, &mut peer).await
}

#[tokio::test]
async fn attach_reply_without_channel_is_missing_channel() -> TestResult {
    let (client, mut peer) = connected().await?;
    let mut result: Value = serde_json::from_str(ATTACHED)?;
    if let Some(object) = result.as_object_mut() {
        object.remove("channel");
    }
    match attach(&client, &mut peer, &result).await? {
        Err(Error::Protocol(error @ ProtocolError::MissingChannel { .. })) => assert_eq!(
            error.to_string(),
            "the terminal.attach reply has no channel on a framed connection"
        ),
        other => return Err(format!("expected MissingChannel, got {other:?}").into()),
    }
    peer.nothing_sent().await?;

    // A channel that is null names none, like one that is absent.
    let mut result: Value = serde_json::from_str(ATTACHED)?;
    result["channel"] = Value::Null;
    match attach(&client, &mut peer, &result).await? {
        Err(Error::Protocol(ProtocolError::MissingChannel { .. })) => {}
        other => {
            return Err(format!("null channel: expected MissingChannel, got {other:?}").into());
        }
    }
    peer.nothing_sent().await?;

    // An object whose only fault is the missing channel still reports MissingChannel.
    match attach(&client, &mut peer, &json!({"channel": null})).await? {
        Err(Error::Protocol(ProtocolError::MissingChannel { .. })) => {}
        other => {
            return Err(
                format!("bare null channel: expected MissingChannel, got {other:?}").into(),
            );
        }
    }
    peer.nothing_sent().await
}

#[tokio::test]
async fn attach_reply_that_is_not_an_object_is_a_decode_error() -> TestResult {
    let (client, mut peer) = connected().await?;
    for result in [json!(null), json!([]), json!("ok"), json!(true)] {
        match attach(&client, &mut peer, &result).await? {
            Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "terminal.attach"),
            other => return Err(format!("{result}: expected Error::Decode, got {other:?}").into()),
        }
        peer.nothing_sent().await?;
    }
    Ok(())
}

/// A frame the client is expected to send on the attach channel.
fn sent(kind: FrameKind, payload: &[u8]) -> Frame {
    stream_frame(ATTACH_CHANNEL, kind, payload)
}

#[tokio::test]
async fn writes_send_their_frames() -> TestResult {
    let (_client, mut stream, mut peer) = attached().await?;
    stream.input(b"ls\n").await?;
    assert_eq!(peer.frame().await?, sent(FrameKind::Data, b"ls\n"));

    stream.resize(100, 30).await?;
    assert_eq!(
        peer.frame().await?,
        sent(FrameKind::Resize, &[100, 0, 30, 0])
    );

    stream.focus(Focus::Gained).await?;
    assert_eq!(peer.frame().await?, sent(FrameKind::Focus, b"\x01"));
    stream.focus(Focus::Lost).await?;
    assert_eq!(peer.frame().await?, sent(FrameKind::Focus, b"\x00"));

    let facts = DisplayFacts {
        bg: Some("#101010".to_owned()),
        cell: Some([8, 16]),
        cursor: None,
        fg: Some("#e0e0e0".to_owned()),
    };
    stream.facts(facts).await?;
    let frame = peer.frame().await?;
    assert_eq!(frame.channel, Channel::new(ATTACH_CHANNEL));
    assert_eq!(frame.kind, FrameKind::Facts);
    assert_eq!(
        serde_json::from_slice::<Value>(&frame.payload)?,
        json!({"bg": "#101010", "cell": [8, 16], "fg": "#e0e0e0"})
    );
    peer.nothing_sent().await
}

#[tokio::test]
async fn automatic_input_needs_both_sides() -> TestResult {
    for (in_hello, in_reply) in [(true, true), (true, false), (false, true), (false, false)] {
        let features: &[&str] = if in_hello {
            &["framed", "automatic_input"]
        } else {
            &["framed"]
        };
        let (client, mut peer) = connected_with(features).await?;
        let mut stream = attach(&client, &mut peer, &attached_result(in_reply)?).await??;
        assert_eq!(stream.automatic_input, in_reply);

        stream.automatic_input(b"\x1b[?1;2c").await?;
        let expected = if in_hello && in_reply {
            FrameKind::AutomaticInput
        } else {
            FrameKind::Data
        };
        assert_eq!(
            peer.frame().await?,
            sent(expected, b"\x1b[?1;2c"),
            "hello {in_hello}, reply {in_reply}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn write_after_core_close_is_stream_closed() -> TestResult {
    let (client, mut stream, mut peer) = attached().await?;
    peer.push_frame(attach_close())?;
    answered_call(&client, &mut peer).await?;

    match stream.input(b"late").await {
        Err(error @ Error::StreamClosed) => {
            assert_eq!(error.to_string(), "the terminal.attach stream is closed");
        }
        other => return Err(format!("expected StreamClosed, got {other:?}").into()),
    }
    assert!(matches!(
        stream.resize(1, 1).await,
        Err(Error::StreamClosed)
    ));
    peer.nothing_sent().await?;

    stream.close().await?;
    peer.nothing_sent().await?;
    // The CLOSE the core sent is still there to read.
    assert!(matches!(
        next_event(&mut stream).await?,
        AttachEvent::Closed(CloseReason::Detached)
    ));
    Ok(())
}

#[tokio::test]
async fn write_after_connection_end_is_closed() -> TestResult {
    // The connection ends while the stream is open.
    let (client, mut stream, mut peer) = attached().await?;
    peer.end();
    wait_ended(&client).await?;
    assert!(matches!(stream.input(b"x").await, Err(Error::Closed)));

    // The stream ended too: the connection's refusal still wins.
    let (client, mut stream, mut peer) = attached().await?;
    peer.push_frame(attach_close())?;
    peer.end();
    wait_ended(&client).await?;
    assert!(matches!(stream.input(b"x").await, Err(Error::Closed)));
    assert!(matches!(stream.close().await, Err(Error::Closed)));
    Ok(())
}

#[tokio::test]
async fn oversized_input_is_refused() -> TestResult {
    let (_client, mut stream, mut peer) = attached().await?;
    let big = vec![b'x'; 16_777_217];
    match stream.input(&big).await {
        Err(Error::PayloadTooLarge { len }) => assert_eq!(len, 16_777_217),
        other => return Err(format!("expected PayloadTooLarge, got {other:?}").into()),
    }
    assert!(matches!(
        stream.automatic_input(&big).await,
        Err(Error::PayloadTooLarge { .. })
    ));
    peer.nothing_sent().await?;

    stream.input(b"ok").await?;
    assert_eq!(peer.frame().await?, sent(FrameKind::Data, b"ok"));
    Ok(())
}

#[tokio::test]
async fn close_then_detached() -> TestResult {
    let (_client, mut stream, mut peer) = attached().await?;
    stream.close().await?;
    assert_eq!(peer.frame().await?, attach_close());
    stream.close().await?;
    peer.nothing_sent().await?;
    assert!(matches!(stream.input(b"x").await, Err(Error::StreamClosed)));

    // The stream keeps yielding until the core answers.
    attach_frame(&peer, FrameKind::Data, b"late")?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Output(b"late".to_vec())
    );
    peer.push_frame(attach_close())?;
    assert_eq!(
        next_event(&mut stream).await?,
        AttachEvent::Closed(CloseReason::Detached)
    );
    assert!(next_attach(&mut stream).await?.is_none());

    drop(stream);
    credited(&mut peer, ATTACH_CHANNEL, 4).await?;
    peer.nothing_sent().await
}

#[tokio::test]
async fn drop_follows_the_close_rule() -> TestResult {
    // An open stream sends one CLOSE when it drops.
    let (_client, stream, mut peer) = attached().await?;
    drop(stream);
    assert_eq!(peer.frame().await?, attach_close());
    peer.nothing_sent().await?;

    // A stream the core closed sends none, though nobody has read the CLOSE.
    let (client, stream, mut peer) = attached().await?;
    peer.push_frame(attach_close())?;
    answered_call(&client, &mut peer).await?;
    drop(stream);
    peer.nothing_sent().await
}

#[tokio::test]
async fn attach_sees_connection_loss() -> TestResult {
    // EOF.
    let (client, mut stream, mut peer) = attached().await?;
    peer.end();
    assert!(matches!(
        next_attach_error(&mut stream).await?,
        Error::Closed
    ));
    assert!(next_attach(&mut stream).await?.is_none());
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::PeerClosed)
    );

    // A write that fails: a call made while the next write is set to fail.
    let (client, mut stream, peer) = attached().await?;
    peer.fail_next_write();
    let refused = timeout(PATIENCE, client.call_raw(method("x")?, json!({}))).await?;
    assert!(matches!(
        refused,
        Err(Error::Transport(TransportError::Io(_)))
    ));
    assert!(matches!(
        next_attach_error(&mut stream).await?,
        Error::Closed
    ));
    assert!(next_attach(&mut stream).await?.is_none());
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::TransportFailed)
    );

    // A frame the source reports as too large.
    let (client, mut stream, peer) = attached().await?;
    peer.fail_source(TransportError::Frame(FrameError::TooLarge {
        len: 16_777_217,
    }))?;
    assert!(matches!(
        next_attach_error(&mut stream).await?,
        Error::Closed
    ));
    assert!(next_attach(&mut stream).await?.is_none());
    assert_eq!(
        client.close_reason().map(|end| end.kind),
        Some(ConnectionEndKind::ProtocolViolation)
    );
    Ok(())
}

#[tokio::test]
async fn attach_stream_and_its_futures_are_send() -> TestResult {
    let (client, mut stream, _peer) = attached().await?;
    assert_send(&stream);
    assert_send(&client.terminal_attach(TerminalAttach::new(terminal("t1d")?, 80, 24)));
    assert_send(&stream.next());
    assert_send(&stream.input(b""));
    assert_send(&stream.automatic_input(b""));
    assert_send(&stream.resize(80, 24));
    assert_send(&stream.focus(Focus::Gained));
    assert_send(&stream.facts(DisplayFacts {
        bg: None,
        cell: None,
        cursor: None,
        fg: None,
    }));
    assert_send(&stream.close());
    Ok(())
}
