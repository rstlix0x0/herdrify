//! `Call`: one request with its options, sent to herdr-core when it is awaited.
//!
//! A `HerdrCore` operation returns a `Call` holding the request it built from plain values. Each
//! option is a method on the call that returns it again, so options chain; a value an option
//! cannot accept is kept as the call's error and reported when the call is awaited. Awaiting the
//! call sends the request through the [`Client`] it was made from.
//!
//! Responsibilities:
//!
//! - [`Call`], its options' plumbing and its [`IntoFuture`] implementation
//! - [`Invoke`], which ties each request type to the [`Client`] method that sends it, and is
//!   public only because [`Call`]'s `.await` names it
//! - the conversions from plain values that several groups share: [`argv`], [`environment`] and
//!   [`labels`]
//!
//! Non-responsibilities:
//!
//! - the options of each request, which the file of its group defines
//! - sending the request, which [`Client`] does

use std::fmt;
use std::future::{Future, IntoFuture};
use std::pin::Pin;

use super::HerdrCoreError;
use crate::connection::Client;
use crate::port::Transport;
use crate::protocol::{Argv, Environment, LabelKey, LabelValue, Labels};

/// A request to herdr-core and its options, sent when awaited.
///
/// Every [`HerdrCore`](super::HerdrCore) operation returns one. Chain its options, then `.await`
/// it; nothing is sent before that. A value an option could not accept, such as an empty machine
/// name, is reported by the `.await` as [`HerdrCoreError::Empty`] and nothing is sent.
///
/// [`with`](Call::with) applies an option of the underlying `protocol` request type directly.
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
///
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// let counted = core
///     .exec(["wc", "-c"])
///     .input("hello")
///     .timeout(Duration::from_secs(5))
///     .await?;
/// assert_eq!(String::from_utf8_lossy(&counted.stdout).trim(), "5");
/// # Ok(())
/// # }
/// ```
#[must_use = "a call sends nothing until it is awaited"]
pub struct Call<T: Transport, P> {
    client: Client<T>,
    params: Result<P, HerdrCoreError>,
}

impl<T: Transport, P> Call<T, P> {
    /// A call that sends `params` through `client`, or reports their error when awaited.
    pub(super) const fn new(client: Client<T>, params: Result<P, HerdrCoreError>) -> Self {
        Self { client, params }
    }

    /// Applies an option of the request type itself, such as
    /// [`TerminalRun::display`](crate::protocol::TerminalRun::display).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_hc::HerdrCore;
    /// use herdrify_hc::protocol::DisplayFacts;
    ///
    /// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
    /// let core = HerdrCore::local().await?;
    /// let facts = DisplayFacts { bg: None, cell: None, cursor: None, fg: Some("#ffffff".to_owned()) };
    /// let started = core.terminals().run(["sh"]).with(|run| run.display(facts)).await?;
    /// # let _ = started;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with(self, option: impl FnOnce(P) -> P) -> Self {
        Self {
            client: self.client,
            params: self.params.map(option),
        }
    }

    /// Applies an option that checks its value, keeping the first error.
    pub(super) fn try_with(self, option: impl FnOnce(P) -> Result<P, HerdrCoreError>) -> Self {
        Self {
            client: self.client,
            params: self.params.and_then(option),
        }
    }

    /// The request as built so far, or the error an option recorded.
    ///
    /// # Errors
    ///
    /// Returns the error of the first option or argument that could not be accepted.
    pub const fn request(&self) -> Result<&P, &HerdrCoreError> {
        self.params.as_ref()
    }
}

impl<T: Transport, P: fmt::Debug> fmt::Debug for Call<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call")
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

/// How a request type is sent: the [`Client`] method that sends it and what awaiting a [`Call`]
/// carrying it returns.
///
/// Every request type a [`HerdrCore`](super::HerdrCore) method builds implements it, which is
/// what lets that [`Call`] be awaited. Programs do not need to name it.
pub trait Invoke<T: Transport>: Sized {
    /// What awaiting the call returns.
    type Output;

    /// Sends the request through `client` and returns its answer.
    fn invoke(
        self,
        client: &Client<T>,
    ) -> impl Future<Output = Result<Self::Output, HerdrCoreError>> + Send;
}

/// Implements [`Invoke`] for request types whose `Client` method's result is the call's output.
macro_rules! invoke_by {
    ($($params:ty => $output:ty, $method:ident;)+) => {
        $(
            impl<T: Transport> $crate::herdr_core::Invoke<T> for $params {
                type Output = $output;

                async fn invoke(
                    self,
                    client: &$crate::connection::Client<T>,
                ) -> Result<Self::Output, $crate::herdr_core::HerdrCoreError> {
                    Ok(client.$method(self).await?)
                }
            }
        )+
    };
}

pub(super) use invoke_by;

// dyn: `IntoFuture` must name its future type, and the future of an `async fn` cannot be named on
// stable Rust, so the future is boxed: the same type erasure as async-in-trait (clause 5). It is
// one allocation per call, next to a round trip to another process.
type BoxedCall<O> = Pin<Box<dyn Future<Output = Result<O, HerdrCoreError>> + Send>>;

impl<T, P> IntoFuture for Call<T, P>
where
    T: Transport,
    P: Invoke<T> + Send + 'static,
{
    type Output = Result<P::Output, HerdrCoreError>;
    type IntoFuture = BoxedCall<P::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.params?.invoke(&self.client).await })
    }
}

/// An [`Argv`] from words: the first is the program and the rest are its arguments.
pub(super) fn argv<S: Into<String>>(
    words: impl IntoIterator<Item = S>,
) -> Result<Argv, HerdrCoreError> {
    Ok(Argv::new(words.into_iter().map(Into::into).collect())?)
}

/// An [`Environment`] from name and value pairs; a later pair replaces an earlier one's value.
pub(super) fn environment<K: Into<String>, V: Into<String>>(
    pairs: impl IntoIterator<Item = (K, V)>,
) -> Environment {
    let mut environment = Environment::new();
    for (name, value) in pairs {
        environment.insert(name, value);
    }
    environment
}

/// [`Labels`] from key and value pairs, refusing an empty key or value.
pub(super) fn labels<K: Into<String>, V: Into<String>>(
    pairs: impl IntoIterator<Item = (K, V)>,
) -> Result<Labels, HerdrCoreError> {
    let mut labels = Labels::new();
    for (key, value) in pairs {
        labels.insert(LabelKey::new(key)?, LabelValue::new(value)?);
    }
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Call, argv, environment, labels};
    use crate::herdr_core::HerdrCoreError;
    use crate::herdr_core::fake::{TestError, connected, serve};
    use crate::protocol::{EmptyValueError, LabelKey, LabelValue, ServerStop, StopResult};

    type TestResult = Result<(), TestError>;

    #[tokio::test]
    async fn awaiting_sends_the_request_and_returns_its_result() -> TestResult {
        let (client, mut peer) = connected().await?;
        let call = Call::new(client, Ok(ServerStop::new()));
        let (sent, stopped) = serve(&mut peer, call, json!({"stopped": true})).await?;
        assert_eq!(sent, json!({"method": "server.stop", "params": {}}));
        let stopped: StopResult = stopped?;
        assert!(stopped.stopped);
        Ok(())
    }

    #[tokio::test]
    async fn a_recorded_error_is_returned_and_nothing_is_sent() -> TestResult {
        let (client, mut peer) = connected().await?;
        let empty = EmptyValueError { what: "argv" };
        let call: Call<_, ServerStop> = Call::new(client, Err(empty.clone().into()));
        let refused = call.await;
        assert!(matches!(refused, Err(HerdrCoreError::Empty(error)) if error == empty));
        assert!(!peer.has_unread());
        Ok(())
    }

    #[tokio::test]
    async fn try_with_keeps_the_first_error_and_with_skips_a_failed_request() -> TestResult {
        let (client, _peer) = connected().await?;
        let first = EmptyValueError { what: "first" };
        let call = Call::new(client, Ok(ServerStop::new()))
            .try_with(|_| Err(first.clone().into()))
            .try_with(|_| Err(EmptyValueError { what: "second" }.into()))
            .with(|stop| stop);
        assert!(matches!(call.request(), Err(HerdrCoreError::Empty(error)) if *error == first));
        Ok(())
    }

    #[tokio::test]
    async fn debug_shows_the_request() -> TestResult {
        let (client, _peer) = connected().await?;
        let call = Call::new(client, Ok(ServerStop::new()));
        assert!(format!("{call:?}").starts_with("Call { params: Ok(ServerStop"));
        Ok(())
    }

    #[test]
    fn argv_takes_the_program_first_and_refuses_no_words() -> TestResult {
        let built = argv(["sh", "-c", "exit 3"])?;
        assert_eq!(built.program(), "sh");
        assert_eq!(built.args(), ["-c", "exit 3"]);
        let none: [&str; 0] = [];
        assert!(matches!(argv(none), Err(HerdrCoreError::Empty(error)) if error.what == "argv"));
        Ok(())
    }

    #[test]
    fn environment_keeps_the_last_value_of_a_name() {
        let built = environment([("A", "1"), ("B", "2"), ("A", "3")]);
        assert_eq!(built.get("A"), Some("3"));
        assert_eq!(built.len(), 2);
    }

    #[test]
    fn labels_refuse_an_empty_key_or_value() -> TestResult {
        let built = labels([("role", "build")])?;
        assert_eq!(
            built.get(&LabelKey::new("role")?).map(LabelValue::as_str),
            Some("build")
        );
        assert!(matches!(
            labels([("", "x")]),
            Err(HerdrCoreError::Empty(error)) if error.what == "label key"
        ));
        assert!(matches!(
            labels([("k", "")]),
            Err(HerdrCoreError::Empty(error)) if error.what == "label value"
        ));
        Ok(())
    }
}
