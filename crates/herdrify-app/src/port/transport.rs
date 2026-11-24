//! The capability of running one herdr-app invocation and returning what it produced.
//!
//! This file holds one trait and no executable logic, so it carries no unit tests
//! (unit-test-mandate exemption #3, pure trait definition). The bounds the trait promises are
//! checked by its doctests.
//!
//! The client above this layer builds an invocation and decodes a reply, but does not start a
//! process itself. Putting the run behind a trait lets the client work over the real program or
//! over a stand-in, and keeps every choice of how a process is started out of the layers that only
//! need its output.
//!
//! Responsibilities:
//!
//! - [`Transport`], run an [`Invocation`] to completion and return its [`RunOutput`]
//!
//! Non-responsibilities:
//!
//! - telling a successful run from a failed one, which [`classify`](crate::protocol::classify) does
//! - choosing how the program is started, which an implementor does

use std::future::Future;

use super::TransportError;
use crate::protocol::{Invocation, RunOutput};

/// Something that can run a herdr-app invocation to completion.
///
/// A run that starts returns its [`RunOutput`] whatever its exit status, so a refusal by the window
/// is an `Ok`. Only a run that could not start is a [`TransportError`]. Implementors are shared
/// between tasks, so they are `Send + Sync + 'static`, and the future they return is `Send`.
///
/// # Examples
///
/// A transport that answers every invocation with the same output:
///
/// ```
/// use std::future::Future;
///
/// use herdrify_app::port::{Transport, TransportError};
/// use herdrify_app::protocol::{Exit, Invocation, RunOutput};
///
/// struct Fixed(RunOutput);
///
/// impl Transport for Fixed {
///     fn run(
///         &self,
///         _invocation: &Invocation,
///     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
///         let output = self.0.clone();
///         async move { Ok(output) }
///     }
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), TransportError> {
/// let transport = Fixed(RunOutput {
///     exit: Exit::Code(0),
///     stdout: b"{}".to_vec(),
///     stderr: Vec::new(),
/// });
/// let output = transport.run(&Invocation::status()).await?;
/// assert_eq!(output.exit, Exit::Code(0));
/// # Ok(())
/// # }
/// ```
///
/// The bounds hold for every implementor, so generic code can share a transport between tasks and
/// spawn the future of a run. This fails to compile if the trait stops promising them:
///
/// ```
/// use herdrify_app::port::Transport;
/// use herdrify_app::protocol::Invocation;
///
/// fn shared_between_tasks<T: Transport>() {
///     fn require<X: Send + Sync + 'static>() {}
///     require::<T>();
/// }
///
/// fn run_is_spawnable<T: Transport>(transport: &T, invocation: &Invocation) {
///     fn require_send<F: Send>(_future: &F) {}
///     require_send(&transport.run(invocation));
/// }
/// ```
pub trait Transport: Send + Sync + 'static {
    /// Runs `invocation` and returns what the program produced.
    ///
    /// # Errors
    ///
    /// Returns a [`TransportError`] when the program could not be started.
    fn run(
        &self,
        invocation: &Invocation,
    ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send;
}
