//! A scripted [`Transport`] for the entrypoint's unit tests.
//!
//! The entrypoint's tests need to see which invocations a call ran and to feed it replies, without
//! starting a program. [`Script`] answers each run with the next reply it was given and records the
//! argv it was asked to run. It is `Clone`, and every clone shares the same replies and record, so a
//! test keeps one clone to read what the app's own clone ran.
//!
//! Responsibilities:
//!
//! - [`Script`], the scripted transport, and [`success`], a successful run with a given stdout
//! - [`app`], an app over a script that answers every command, and [`assert_sent`], which checks
//!   the command it last ran
//!
//! Non-responsibilities:
//!
//! - anything the tests assert, which each test module does
//!
//! This file is compiled only for tests. It depends on [`port`](crate::port) and
//! [`protocol`](crate::protocol), and its own test checks that the script answers in order and
//! records what it ran.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;

use super::HerdrApp;
use crate::driver::{Client, Error};
use crate::port::{Transport, TransportError};
use crate::protocol::{Exit, Invocation, RunOutput};

/// A reply every command's output decodes from: an acknowledgement that also carries the fields of
/// `space.new`, `tab.new`, `pane.split`, `terminal.open`, `terminal.keep` and `terminal.park`.
const ANY_REPLY: &str =
    r#"{"ok":true,"space":"sp1","tab":"t1","terminal":"t2","pane":"p1","pending":true}"#;

/// A transport that answers with the replies it was given, in order, and records every argv.
///
/// A run past the last reply is [`TransportError::NotFound`], unless the script was told to
/// [`repeat`](Script::repeating) a reply.
#[derive(Clone)]
pub(super) struct Script {
    replies: Arc<Mutex<VecDeque<RunOutput>>>,
    rest: Option<RunOutput>,
    argvs: Arc<Mutex<Vec<Vec<String>>>>,
}

impl Script {
    /// A script that answers with `replies`, one per run.
    pub(super) fn new(replies: impl IntoIterator<Item = RunOutput>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into_iter().collect())),
            rest: None,
            argvs: Arc::default(),
        }
    }

    /// The same script, answering every run past its last reply with `reply`.
    pub(super) fn repeating(mut self, reply: RunOutput) -> Self {
        self.rest = Some(reply);
        self
    }

    /// The argv of every run so far, in order.
    pub(super) fn argvs(&self) -> Vec<Vec<String>> {
        self.argvs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Transport for Script {
    fn run(
        &self,
        invocation: &Invocation,
    ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
        self.argvs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(invocation.argv());
        let reply = self
            .replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .or_else(|| self.rest.clone());
        async move {
            reply.ok_or_else(|| TransportError::NotFound {
                program: "scripted".to_owned(),
            })
        }
    }
}

/// A run that exited with code 0 and wrote `stdout`.
pub(super) fn success(stdout: &str) -> RunOutput {
    RunOutput {
        exit: Exit::Code(0),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

/// An app over a script that reports version 0.1.17 and then answers every command with
/// [`ANY_REPLY`], and the script, to read what the app ran.
///
/// # Errors
///
/// Fails when the version check does.
pub(super) async fn app() -> Result<(HerdrApp<Script>, Script), Error> {
    let script = Script::new([success("herdr-app 0.1.17\n")]).repeating(success(ANY_REPLY));
    let app = HerdrApp::from_client(Client::new(script.clone())).await?;
    Ok((app, script))
}

/// Asserts that the last run of `script` was `do <name> <json>` with `json` equal to `expected`
/// when parsed.
pub(super) fn assert_sent(script: &Script, name: &str, expected: &Value) {
    let last = script.argvs().pop().unwrap_or_default();
    assert_eq!(last.len(), 3, "not a `do` run: {last:?}");
    assert_eq!(last[..2], ["do", name], "{last:?}");
    let sent: Option<Value> = serde_json::from_str(&last[2]).ok();
    assert_eq!(sent.as_ref(), Some(expected), "{last:?}");
}

mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn answers_in_order_records_every_run_and_runs_dry() -> Result<(), TransportError> {
        let script = Script::new([success("one"), success("two")]);
        let shared = script.clone();

        let first = script.run(&Invocation::ls()).await?;
        let second = shared.run(&Invocation::status()).await?;
        assert_eq!(first.stdout, b"one");
        assert_eq!(second.stdout, b"two");
        assert_eq!(script.argvs(), [["ls"], ["status"]]);

        let dry = script.run(&Invocation::commands()).await;
        assert!(matches!(dry, Err(TransportError::NotFound { .. })));
        assert_eq!(shared.argvs().len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn the_app_helper_checks_the_version_and_answers_every_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        assert_eq!(script.argvs(), [["--version"]]);
        for _ in 0..3 {
            app.client().run(crate::protocol::TabNext::new()).await?;
        }
        assert_sent(&script, "tab.next", &json!({}));
        assert_eq!(script.argvs().len(), 4);
        Ok(())
    }
}
