//! The client that runs herdr-app invocations and returns typed replies.
//!
//! A [`Client`] wraps a [`Transport`] and gives one method per kind of invocation. Each call builds
//! the [`Invocation`], runs it through the transport, sorts the finished run with
//! [`classify`], and decodes stdout into the type the call promises. A failure at any step is an
//! [`Error`].
//!
//! Responsibilities:
//!
//! - [`Client`], the calls for `--version`, `commands`, `ls`, `status`, `do` and any other invocation
//! - checking the version against the oldest one this crate supports
//!
//! Non-responsibilities:
//!
//! - starting the program, which the [`Transport`] does
//! - the rules that tell a failed run from a successful one, which [`classify`] holds

use std::future::Future;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::Error;
use crate::files::CommandName;
use crate::port::Transport;
use crate::protocol::{
    AppVersion, Command, CommandInfo, Invocation, Listing, WindowStatus, classify,
};

/// Runs herdr-app invocations through a [`Transport`] and returns typed replies.
///
/// The client holds its transport behind an `Arc`, so cloning it is cheap and every clone shares
/// the one transport, whether or not the transport itself is `Clone`. Calls may run at the same
/// time. Each call is one run of the transport, with no promise about their order.
///
/// # Examples
///
/// ```no_run
/// # use std::future::Future;
/// # use herdrify_app::port::{Transport, TransportError};
/// # use herdrify_app::protocol::{Invocation, RunOutput};
/// # struct Process;
/// # impl Transport for Process {
/// #     fn run(
/// #         &self,
/// #         _invocation: &Invocation,
/// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
/// #         async { Err(TransportError::NotFound { program: "herdr-app".to_owned() }) }
/// #     }
/// # }
/// use herdrify_app::driver::Client;
///
/// let client = Client::new(Process);
/// let other = client.clone();
/// ```
#[derive(Debug)]
pub struct Client<T: Transport> {
    transport: Arc<T>,
}

impl<T: Transport> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            transport: Arc::clone(&self.transport),
        }
    }
}

impl<T: Transport> Client<T> {
    /// A client over `transport`. It does no I/O.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Invocation, RunOutput};
    /// # struct Process;
    /// # impl Transport for Process {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         async { Err(TransportError::NotFound { program: "herdr-app".to_owned() }) }
    /// #     }
    /// # }
    /// use herdrify_app::driver::Client;
    ///
    /// let client = Client::new(Process);
    /// ```
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
        }
    }

    /// The version herdr-app reports, from `--version`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the program could not be started, the error of the failed
    /// run when it did not succeed, and [`Error::InvalidVersion`] when its output is not a version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Exit, Invocation, RunOutput};
    /// # struct Fixed(&'static str);
    /// # impl Transport for Fixed {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         let stdout = self.0.as_bytes().to_vec();
    /// #         async move {
    /// #             Ok(RunOutput { exit: Exit::Code(0), stdout, stderr: Vec::new() })
    /// #         }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new(Fixed("herdr-app 0.1.17\n"));
    /// assert_eq!(client.version().await?.to_string(), "0.1.17");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn version(&self) -> Result<AppVersion, Error> {
        let invocation = Invocation::version();
        let stdout = self.stdout(&invocation).await?;
        Ok(AppVersion::parse(&String::from_utf8_lossy(&stdout))?)
    }

    /// The version herdr-app reports, refused when it is older than [`AppVersion::MINIMUM`].
    ///
    /// A newer version than [`AppVersion::PROVEN`] is accepted and returned, so the caller can
    /// report it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedVersion`] when the version is below the minimum, and the errors
    /// of [`version`](Self::version) otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Exit, Invocation, RunOutput};
    /// # struct Fixed(&'static str);
    /// # impl Transport for Fixed {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         let stdout = self.0.as_bytes().to_vec();
    /// #         async move {
    /// #             Ok(RunOutput { exit: Exit::Code(0), stdout, stderr: Vec::new() })
    /// #         }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let old = Client::new(Fixed("herdr-app 0.1.15\n"));
    /// assert_eq!(
    ///     old.check_version().await.err().map(|error| error.to_string()),
    ///     Some(
    ///         "herdr-app 0.1.15 is older than 0.1.16, the oldest version this crate supports"
    ///             .to_owned()
    ///     ),
    /// );
    ///
    /// let current = Client::new(Fixed("herdr-app 0.1.17\n"));
    /// assert_eq!(current.check_version().await?.to_string(), "0.1.17");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_version(&self) -> Result<AppVersion, Error> {
        let found = self.version().await?;
        if found < AppVersion::MINIMUM {
            return Err(Error::UnsupportedVersion {
                found,
                minimum: AppVersion::MINIMUM,
            });
        }
        Ok(found)
    }

    /// The commands herdr-app offers, from `commands`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the program could not be started, the error of the failed
    /// run when it did not succeed, and [`Error::Decode`] when its output is not a command list.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Exit, Invocation, RunOutput};
    /// # struct Fixed(&'static str);
    /// # impl Transport for Fixed {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         let stdout = self.0.as_bytes().to_vec();
    /// #         async move {
    /// #             Ok(RunOutput { exit: Exit::Code(0), stdout, stderr: Vec::new() })
    /// #         }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new(Fixed(
    ///     r#"[{"name":"tab.next","args":[],"about":"Show the next tab."}]"#,
    /// ));
    /// let commands = client.commands().await?;
    /// assert_eq!(commands[0].name.as_str(), "tab.next");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn commands(&self) -> Result<Vec<CommandInfo>, Error> {
        self.read(&Invocation::commands()).await
    }

    /// The spaces, tabs and terminals of the window, from `ls`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the program could not be started, the error of the failed
    /// run when it did not succeed, and [`Error::Decode`] when its output is not a listing.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Exit, Invocation, RunOutput};
    /// # struct Fixed(&'static str);
    /// # impl Transport for Fixed {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         let stdout = self.0.as_bytes().to_vec();
    /// #         async move {
    /// #             Ok(RunOutput { exit: Exit::Code(0), stdout, stderr: Vec::new() })
    /// #         }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new(Fixed(r#"{"spaces":[],"recent":[]}"#));
    /// assert!(client.ls().await?.spaces.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ls(&self) -> Result<Listing, Error> {
        self.read(&Invocation::ls()).await
    }

    /// The state of the window, from `status`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the program could not be started, the error of the failed
    /// run when it did not succeed, and [`Error::Decode`] when its output is not a window status.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Exit, Invocation, RunOutput};
    /// # struct Fixed(&'static str);
    /// # impl Transport for Fixed {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         let stdout = self.0.as_bytes().to_vec();
    /// #         async move {
    /// #             Ok(RunOutput { exit: Exit::Code(0), stdout, stderr: Vec::new() })
    /// #         }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new(Fixed(
    ///     r#"{
    ///         "zoom": 1.0, "font_size": 14.0, "ui_font_size": 13.0,
    ///         "space": null, "tab": null, "focused_pane": null,
    ///         "layout": "main", "agent": null,
    ///         "floats": {}, "overlays": [], "errors": []
    ///     }"#,
    /// ));
    /// assert_eq!(client.status().await?.layout.as_str(), "main");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn status(&self) -> Result<WindowStatus, Error> {
        self.read(&Invocation::status()).await
    }

    /// Runs `command` and returns its typed reply, from `do <name> <json>`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`], without running anything, when the command does not
    /// serialize, does not serialize to a JSON object, or has a name herdr-app cannot be asked for
    /// (an empty one). Otherwise returns [`Error::Transport`] when the program could not
    /// be started, the error of the failed run when it did not succeed (such as
    /// [`Error::Refused`] when the window refuses the command), and [`Error::Decode`] when the
    /// reply is not the command's output.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Invocation, RunOutput};
    /// # struct Process;
    /// # impl Transport for Process {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         async { Err(TransportError::NotFound { program: "herdr-app".to_owned() }) }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    /// use herdrify_app::protocol::TabNext;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new(Process);
    /// client.run(TabNext::new()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn run<C: Command>(
        &self,
        command: C,
    ) -> impl Future<Output = Result<C::Output, Error>> + Send + '_ {
        let invocation = invocation_of(command);
        async move { self.read(&invocation?).await }
    }

    /// Runs any invocation and returns its stdout as JSON, with no typed reply.
    ///
    /// This reaches what the typed calls do not name, such as a mod's own `<mod>.<command>`, and
    /// reads fields a typed reply leaves out. `raw(Invocation::version())` is
    /// [`Error::Decode`], because the stdout of `--version` is not JSON. Use
    /// [`version`](Self::version) for that.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the program could not be started, the error of the failed
    /// run when it did not succeed, and [`Error::Decode`] when its stdout is not JSON.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::RunOutput;
    /// # struct Process;
    /// # impl Transport for Process {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &herdrify_app::protocol::Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         async { Err(TransportError::NotFound { program: "herdr-app".to_owned() }) }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    /// use herdrify_app::protocol::Invocation;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new(Process);
    /// let status = client.raw(Invocation::status()).await?;
    /// println!("{}", status["layout"]);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn raw(&self, invocation: Invocation) -> Result<Value, Error> {
        self.read(&invocation).await
    }

    /// Runs `invocation` and decodes the stdout of a successful run as `D`.
    async fn read<D: DeserializeOwned>(&self, invocation: &Invocation) -> Result<D, Error> {
        let stdout = self.stdout(invocation).await?;
        serde_json::from_slice(&stdout).map_err(|error| Error::Decode {
            invocation: invocation.label(),
            detail: error.to_string(),
        })
    }

    /// Runs `invocation` and returns the stdout of a successful run.
    async fn stdout(&self, invocation: &Invocation) -> Result<Vec<u8>, Error> {
        let output = self.transport.run(invocation).await?;
        if let Err(failure) = classify(&output) {
            return Err(Error::from_failure(&invocation.label(), failure));
        }
        Ok(output.stdout)
    }
}

/// The `do <name> <json>` invocation for `command`, built before anything runs.
fn invocation_of<C: Command>(command: C) -> Result<Invocation, Error> {
    let invalid = |detail: String| Error::InvalidArgument {
        argument: "command".to_owned(),
        detail,
    };
    let Value::Object(arguments) =
        serde_json::to_value(command).map_err(|error| invalid(error.to_string()))?
    else {
        return Err(invalid(
            "a command must serialize to a JSON object".to_owned(),
        ));
    };
    let name = CommandName::new(C::NAME).map_err(|error| invalid(error.to_string()))?;
    Ok(Invocation::run(&name, &arguments))
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Mutex, PoisonError};

    use serde::{Serialize, Serializer};

    use super::*;
    use crate::port::TransportError;
    use crate::protocol::{Ack, Exit, RunOutput, TabNext};

    /// A transport that answers with the given outputs in order and records every argv it is asked
    /// to run.
    struct ScriptedTransport {
        replies: Mutex<VecDeque<RunOutput>>,
        argvs: Mutex<Vec<Vec<String>>>,
    }

    impl ScriptedTransport {
        fn new(replies: impl IntoIterator<Item = RunOutput>) -> Self {
            Self {
                replies: Mutex::new(replies.into_iter().collect()),
                argvs: Mutex::new(Vec::new()),
            }
        }

        fn argvs(&self) -> Vec<Vec<String>> {
            self.argvs
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    impl Transport for ScriptedTransport {
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
                .pop_front();
            async move {
                reply.ok_or_else(|| TransportError::NotFound {
                    program: "scripted".to_owned(),
                })
            }
        }
    }

    fn success(stdout: &str) -> RunOutput {
        RunOutput {
            exit: Exit::Code(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    fn client(replies: impl IntoIterator<Item = RunOutput>) -> Client<ScriptedTransport> {
        Client::new(ScriptedTransport::new(replies))
    }

    fn assert_send<F: Send>(_future: &F) {}

    /// The error of a call that must have failed.
    fn failure<V>(result: Result<V, Error>) -> Result<Error, &'static str> {
        match result {
            Err(error) => Ok(error),
            Ok(_) => Err("the call should have failed"),
        }
    }

    /// A command whose arguments serialize to the number 7, which is not an object.
    struct NotAnObject;

    impl Serialize for NotAnObject {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_u8(7)
        }
    }

    impl Command for NotAnObject {
        const NAME: &'static str = "test.number";
        type Output = Ack;
    }

    /// A command whose arguments cannot be serialized at all.
    struct CannotSerialize;

    impl Serialize for CannotSerialize {
        fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(<S::Error as serde::ser::Error>::custom("cannot be written"))
        }
    }

    impl Command for CannotSerialize {
        const NAME: &'static str = "test.broken";
        type Output = Ack;
    }

    /// A command with an empty name, which herdr-app could not be asked for.
    #[derive(Serialize)]
    struct Unnamed {}

    impl Command for Unnamed {
        const NAME: &'static str = "";
        type Output = Ack;
    }

    /// Fails to compile unless the future of every public call is `Send` for any transport, and the
    /// future of `run` is `Send` for any command, `Send` or not.
    fn assert_calls_are_send<T: Transport, C: Command>(client: &Client<T>, command: C) {
        assert_send(&client.version());
        assert_send(&client.check_version());
        assert_send(&client.commands());
        assert_send(&client.ls());
        assert_send(&client.status());
        assert_send(&client.run(command));
        assert_send(&client.raw(Invocation::status()));
    }

    #[tokio::test]
    async fn version_parses_stdout() -> Result<(), Error> {
        let client = client([success("herdr-app 0.1.17\n")]);
        let version = client.version().await?;
        assert_eq!(version, AppVersion::parse("herdr-app 0.1.17")?);
        assert_eq!(client.transport.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn version_unreadable_is_invalid_version() -> Result<(), &'static str> {
        let client = client([success("herdr-app 0.1")]);
        let error = failure(client.version().await)?;
        assert_eq!(
            error.to_string(),
            r#"cannot read a herdr-app version from "herdr-app 0.1""#
        );
        assert!(matches!(error, Error::InvalidVersion(_)));
        Ok(())
    }

    #[tokio::test]
    async fn check_version_accepts_a_supported_version() -> Result<(), Error> {
        let client = client([success("herdr-app 0.1.16\n")]);
        assert_eq!(client.check_version().await?, AppVersion::MINIMUM);
        Ok(())
    }

    #[tokio::test]
    async fn check_version_accepts_a_newer_version_and_returns_it() -> Result<(), Error> {
        let client = client([success("herdr-app 0.2.0\n")]);
        let found = client.check_version().await?;
        assert!(found > AppVersion::PROVEN);
        assert_eq!(found.to_string(), "0.2.0");
        Ok(())
    }

    #[tokio::test]
    async fn check_version_refuses_old() -> Result<(), &'static str> {
        let client = client([success("herdr-app 0.1.15")]);
        let error = failure(client.check_version().await)?;
        assert_eq!(
            error.to_string(),
            "herdr-app 0.1.15 is older than 0.1.16, the oldest version this crate supports"
        );
        assert!(matches!(error, Error::UnsupportedVersion { .. }));
        Ok(())
    }

    #[tokio::test]
    async fn commands_decodes_the_catalog() -> Result<(), Error> {
        let client = client([success(
            r#"[
                {"name":"tab.next","args":[],"about":"Show the next tab."},
                {"name":"tab.prev","args":[],"about":"Show the previous tab."}
            ]"#,
        )]);
        let commands = client.commands().await?;
        let names: Vec<&str> = commands.iter().map(|info| info.name.as_str()).collect();
        assert_eq!(names, ["tab.next", "tab.prev"]);
        assert_eq!(client.transport.argvs(), [["commands"]]);
        Ok(())
    }

    #[tokio::test]
    async fn ls_and_status_decode() -> Result<(), Error> {
        let client = client([
            success(r#"{"spaces":[],"recent":[]}"#),
            success(
                r#"{
                    "zoom": 1.0, "font_size": 14.0, "ui_font_size": 13.0,
                    "space": null, "tab": null, "focused_pane": null,
                    "layout": "main", "agent": null,
                    "floats": {}, "overlays": ["frame-welcome"], "errors": []
                }"#,
            ),
        ]);
        let listing = client.ls().await?;
        assert!(listing.spaces.is_empty());
        assert!(listing.recent.is_empty());

        let status = client.status().await?;
        assert_eq!(status.layout.as_str(), "main");
        assert_eq!(status.overlays.len(), 1);
        assert_eq!(client.transport.argvs(), [["ls"], ["status"]]);
        Ok(())
    }

    #[tokio::test]
    async fn run_sends_the_command_and_decodes() -> Result<(), Error> {
        let client = client([success(r#"{"ok":true}"#)]);
        let reply = client.run(TabNext::new()).await?;
        assert_eq!(reply, Ack);
        assert_eq!(client.transport.argvs(), [["do", "tab.next", "{}"]]);
        Ok(())
    }

    #[tokio::test]
    async fn run_refuses_a_non_object_command() -> Result<(), &'static str> {
        let client = client([success(r#"{"ok":true}"#)]);
        let error = failure(client.run(NotAnObject).await)?;
        assert_eq!(
            error.to_string(),
            "invalid command: a command must serialize to a JSON object"
        );
        assert!(client.transport.argvs().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn run_refuses_a_command_that_cannot_serialize() -> Result<(), &'static str> {
        let client = client([success(r#"{"ok":true}"#)]);
        let error = failure(client.run(CannotSerialize).await)?;
        match error {
            Error::InvalidArgument { argument, detail } => {
                assert_eq!(argument, "command");
                assert_eq!(detail, "cannot be written");
            }
            _ => return Err("expected an invalid argument"),
        }
        assert!(client.transport.argvs().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn run_refuses_an_empty_command_name() -> Result<(), &'static str> {
        let client = client([success(r#"{"ok":true}"#)]);
        let error = failure(client.run(Unnamed {}).await)?;
        match error {
            Error::InvalidArgument { argument, detail } => {
                assert_eq!(argument, "command");
                assert_eq!(detail, "a command name cannot be empty");
            }
            _ => return Err("expected an invalid argument"),
        }
        assert!(client.transport.argvs().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn run_maps_refusals() -> Result<(), &'static str> {
        let client = client([RunOutput {
            exit: Exit::Code(1),
            stdout: br#"{"error":"x"}"#.to_vec(),
            stderr: Vec::new(),
        }]);
        let error = failure(client.run(TabNext::new()).await)?;
        match error {
            Error::Refused {
                invocation,
                message,
            } => {
                assert_eq!(invocation, "do tab.next");
                assert_eq!(message, "x");
                Ok(())
            }
            _ => Err("expected a refusal"),
        }
    }

    #[tokio::test]
    async fn transport_failure_is_transport() -> Result<(), &'static str> {
        let client = client([]);
        let error = failure(client.ls().await)?;
        assert!(matches!(
            error,
            Error::Transport(TransportError::NotFound { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn ls_decode_failure_is_decode() -> Result<(), &'static str> {
        let client = client([success(r#"{"recent":[]}"#)]);
        let error = failure(client.ls().await)?;
        assert_eq!(
            error.to_string(),
            "cannot read the output of herdr-app ls: missing field `spaces` at line 1 column 13"
        );
        assert!(matches!(error, Error::Decode { .. }));
        Ok(())
    }

    #[tokio::test]
    async fn raw_returns_unnamed_fields() -> Result<(), Error> {
        let client = client([success(r#"{"layout":"main","extra":{"deep":[1,2]}}"#)]);
        let value = client.raw(Invocation::status()).await?;
        assert_eq!(value["layout"], "main");
        assert_eq!(value["extra"]["deep"][1], 2);
        Ok(())
    }

    #[tokio::test]
    async fn raw_version_is_decode() -> Result<(), &'static str> {
        let client = client([success("herdr-app 0.1.17\n")]);
        let error = failure(client.raw(Invocation::version()).await)?;
        match error {
            Error::Decode { invocation, .. } => {
                assert_eq!(invocation, "--version");
                Ok(())
            }
            _ => Err("expected a decode error"),
        }
    }

    #[tokio::test]
    async fn client_is_clone_and_calls_are_send() -> Result<(), Error> {
        let client = client([success(r#"{"ok":true}"#)]);
        let copy = client.clone();
        assert!(Arc::ptr_eq(&client.transport, &copy.transport));

        assert_calls_are_send(&copy, TabNext::new());
        copy.run(TabNext::new()).await?;
        assert_eq!(client.transport.argvs(), [["do", "tab.next", "{}"]]);
        Ok(())
    }
}
