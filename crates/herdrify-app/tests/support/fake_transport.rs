//! A scripted `Transport` for driving a `Client` or a `HerdrApp` from outside the crate.
//!
//! The transport implements the public trait only, which also shows that a caller can write one.
//! It answers each run with the next output it was given, in order, and records the argv of every
//! run. It is `Clone`, and every clone shares the same queue and record, so a test keeps one clone
//! to read what the client's own clone ran. A run past the last output is a
//! [`TransportError::NotFound`], so a call the test did not expect fails instead of hanging.
//!
//! It depends on the public API of `herdrify-app` only. `tests/client_fake.rs` includes it and is
//! what exercises it.
//!
//! Responsibilities:
//!
//! - replaying the scripted outputs in order, one per run
//! - recording the argv of every run, shared across clones
//! - failing a run past the last output with a [`TransportError::NotFound`]
//! - the `success` and `ended` helpers that build a [`RunOutput`]
//!
//! Non-responsibilities:
//!
//! - judging an output: what a run's exit and streams mean is the client's job
//! - scripting a failure of the transport itself, other than running out of outputs

use std::collections::VecDeque;
use std::error::Error;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};

use herdrify_app::protocol::{Exit, Invocation, RunOutput};
use herdrify_app::{Transport, TransportError};

/// Anything a test can fail with.
pub type TestError = Box<dyn Error + Send + Sync>;

/// A run that exited with code 0 and printed `stdout`.
#[must_use]
pub fn success(stdout: &str) -> RunOutput {
    RunOutput {
        exit: Exit::Code(0),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

/// A run that ended with `exit` and printed `stdout` and `stderr`.
#[must_use]
pub fn ended(exit: Exit, stdout: &str, stderr: &str) -> RunOutput {
    RunOutput {
        exit,
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.as_bytes().to_vec(),
    }
}

/// A transport that replays scripted outputs and records every argv it was asked to run.
#[derive(Clone, Default)]
pub struct FakeTransport {
    outputs: Arc<Mutex<VecDeque<RunOutput>>>,
    argvs: Arc<Mutex<Vec<Vec<String>>>>,
}

impl FakeTransport {
    /// A transport that answers with `outputs`, one per run.
    #[must_use]
    pub fn new(outputs: impl IntoIterator<Item = RunOutput>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(outputs.into_iter().collect())),
            argvs: Arc::default(),
        }
    }

    /// The argv of every run so far, in order.
    #[must_use]
    pub fn argvs(&self) -> Vec<Vec<String>> {
        self.argvs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Transport for FakeTransport {
    fn run(
        &self,
        invocation: &Invocation,
    ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
        self.argvs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(invocation.argv());
        let next = self
            .outputs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front();
        async move {
            next.ok_or_else(|| TransportError::NotFound {
                program: "fake herdr-app: no output was scripted for this run".to_owned(),
            })
        }
    }
}
