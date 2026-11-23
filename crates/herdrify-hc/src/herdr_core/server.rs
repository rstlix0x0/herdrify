//! The server methods: the core's status, and stopping or restarting it.
//!
//! Responsibilities:
//!
//! - [`ServerOps`] and the options of `server.restart`
//!
//! Non-responsibilities:
//!
//! - what a stop or restart does to this connection, which [`Client`] reports

use std::fmt;

use super::call::{Call, environment, invoke_by};
use crate::adapter::ProcessTransport;
use crate::connection::Client;
use crate::port::Transport;
use crate::protocol::{ServerRestart, ServerStatus, ServerStop, StatusResult, StopResult};

/// The methods about the core's own process.
///
/// Reached through [`HerdrCore::server`](super::HerdrCore::server). Stopping or restarting the
/// core ends this connection.
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// let status = core.server().status().await?;
/// println!("core {} has run for {} s", status.pid, status.uptime_ms / 1000);
/// # Ok(())
/// # }
/// ```
pub struct ServerOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

impl<T: Transport> ServerOps<T> {
    /// The server methods over `client`.
    pub(super) const fn new(client: Client<T>) -> Self {
        Self { client }
    }

    /// `server.status`: the core's identity, version, uptime and terminal counts.
    pub fn status(&self) -> Call<T, ServerStatus> {
        Call::new(self.client.clone(), Ok(ServerStatus::new()))
    }

    /// `server.stop`: stops the core.
    pub fn stop(&self) -> Call<T, ServerStop> {
        Call::new(self.client.clone(), Ok(ServerStop::new()))
    }

    /// `server.restart`: restarts the core, keeping its environment unless
    /// [`environment`](Call::environment) replaces it.
    pub fn restart(&self) -> Call<T, ServerRestart> {
        Call::new(self.client.clone(), Ok(ServerRestart::new()))
    }
}

impl<T: Transport> fmt::Debug for ServerOps<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerOps").finish_non_exhaustive()
    }
}

impl<T: Transport> Call<T, ServerRestart> {
    /// Replaces the environment the restarted server runs with.
    pub fn environment<K: Into<String>, V: Into<String>>(
        self,
        variables: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let variables = environment(variables);
        self.with(|restart| restart.environment(variables))
    }
}

invoke_by! {
    ServerStatus => StatusResult, server_status;
    ServerStop => StopResult, server_stop;
    ServerRestart => StopResult, server_restart;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::herdr_core::HerdrCore;
    use crate::herdr_core::fake::{TestError, connected, request_of, serve};
    use crate::protocol::{Environment, ServerRestart, ServerStatus, ServerStop};

    type TestResult = Result<(), TestError>;

    #[tokio::test]
    async fn each_method_builds_its_request() -> TestResult {
        let (client, _peer) = connected().await?;
        let server = HerdrCore::from_client(client).server();
        assert_eq!(request_of(&server.status())?, &ServerStatus::new());
        assert_eq!(request_of(&server.stop())?, &ServerStop::new());
        let mut env = Environment::new();
        env.insert("PATH", "/bin");
        assert_eq!(
            request_of(&server.restart().environment([("PATH", "/bin")]))?,
            &ServerRestart::new().environment(env)
        );
        Ok(())
    }

    #[tokio::test]
    async fn stop_and_restart_are_sent_as_their_method() -> TestResult {
        let (client, mut peer) = connected().await?;
        let server = HerdrCore::from_client(client).server();
        let (sent, stopped) = serve(&mut peer, server.restart(), json!({"stopped": true})).await?;
        assert_eq!(sent["method"], "server.restart");
        assert!(stopped?.stopped);
        let (sent, stopped) = serve(&mut peer, server.stop(), json!({"stopped": true})).await?;
        assert_eq!(sent["method"], "server.stop");
        assert!(stopped?.stopped);
        Ok(())
    }
}
