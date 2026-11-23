//! The machine methods: add, list, connect, rename and remove the machines the core reaches.
//!
//! `machine.exec` is not here: it runs a command and is reached from
//! [`HerdrCore::exec`](super::HerdrCore::exec).
//!
//! Responsibilities:
//!
//! - [`MachineOps`] and the options of `machine.add`
//!
//! Non-responsibilities:
//!
//! - `machine.exec`, which `entry` defines

use std::fmt;

use super::HerdrCoreError;
use super::call::{Call, invoke_by};
use crate::adapter::ProcessTransport;
use crate::connection::Client;
use crate::port::Transport;
use crate::protocol::{
    Machine, MachineAdd, MachineConnect, MachineList, MachineListResult, MachineName,
    MachineRename, MachineRm, MachineTarget, MachineView, RemoveResult, SessionName,
};

/// The methods that manage the machines the core reaches.
///
/// Reached through [`HerdrCore::machines`](super::HerdrCore::machines). A machine is named by a
/// string; an empty one is reported by the `.await` as [`HerdrCoreError::Empty`].
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// core.machines().add_ssh_as("box", "user@box").await?;
/// for machine in core.machines().list().await?.machines {
///     println!("{} is {}", machine.name, machine.state);
/// }
/// # Ok(())
/// # }
/// ```
pub struct MachineOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

impl<T: Transport> MachineOps<T> {
    /// The machine methods over `client`.
    pub(super) const fn new(client: Client<T>) -> Self {
        Self { client }
    }

    /// A call with `params`, or the error building them recorded.
    fn call<P>(&self, params: Result<P, HerdrCoreError>) -> Call<T, P> {
        Call::new(self.client.clone(), params)
    }

    /// `machine.add`: a machine reached over ssh as `target`: a host, `user@host`,
    /// `user@host:port` or a `Host` from the ssh config. The core calls it by the target's host.
    pub fn add_ssh(&self, target: impl Into<String>) -> Call<T, MachineAdd> {
        self.call(Ok(MachineAdd::new(MachineTarget::Ssh {
            target: target.into(),
            name: None,
        })))
    }

    /// `machine.add`: a machine called `name`, reached over ssh as `target`.
    pub fn add_ssh_as(
        &self,
        name: impl Into<String>,
        target: impl Into<String>,
    ) -> Call<T, MachineAdd> {
        let target = target.into();
        self.call(machine_name(name).map(|name| {
            MachineAdd::new(MachineTarget::Ssh {
                target,
                name: Some(name),
            })
        }))
    }

    /// `machine.add`: a machine called `name`, reached by `command` instead of ssh. The core
    /// gives the command a shell line to run on the machine as its last argument.
    pub fn add_via(
        &self,
        name: impl Into<String>,
        command: impl Into<String>,
    ) -> Call<T, MachineAdd> {
        let command = command.into();
        self.call(
            machine_name(name).map(|name| MachineAdd::new(MachineTarget::Via { name, command })),
        )
    }

    /// `machine.list`: the machines the core reaches, with their state.
    pub fn list(&self) -> Call<T, MachineList> {
        self.call(Ok(MachineList::new()))
    }

    /// `machine.connect`: connects to the machine called `machine`.
    pub fn connect(&self, machine: impl Into<String>) -> Call<T, MachineConnect> {
        self.call(machine_name(machine).map(MachineConnect::new))
    }

    /// `machine.rename`: renames the machine called `machine` to `new_name`.
    pub fn rename(
        &self,
        machine: impl Into<String>,
        new_name: impl Into<String>,
    ) -> Call<T, MachineRename> {
        let new_name = machine_name(new_name);
        self.call(
            machine_name(machine).and_then(|machine| Ok(MachineRename::new(machine, new_name?))),
        )
    }

    /// `machine.rm`: forgets the machine called `machine`.
    pub fn rm(&self, machine: impl Into<String>) -> Call<T, MachineRm> {
        self.call(machine_name(machine).map(MachineRm::new))
    }
}

impl<T: Transport> fmt::Debug for MachineOps<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MachineOps").finish_non_exhaustive()
    }
}

/// A machine name from a string, refusing an empty one.
fn machine_name(name: impl Into<String>) -> Result<MachineName, HerdrCoreError> {
    Ok(MachineName::new(name)?)
}

impl<T: Transport> Call<T, MachineAdd> {
    /// The `hc` session on that machine. Without one, the machine's default.
    pub fn session(self, session: impl Into<String>) -> Self {
        let session = SessionName::new(session);
        self.try_with(|add| Ok(add.session(session?)))
    }

    /// Marks the machine as short-lived, so clients can show it apart from machines that stay.
    pub fn temporary(self) -> Self {
        self.with(MachineAdd::temporary)
    }
}

invoke_by! {
    MachineAdd => Machine, machine_add;
    MachineList => MachineListResult, machine_list;
    MachineConnect => MachineView, machine_connect;
    MachineRename => Machine, machine_rename;
    MachineRm => RemoveResult, machine_rm;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::herdr_core::fake::{TestError, connected, request_of, serve};
    use crate::herdr_core::{HerdrCore, HerdrCoreError};
    use crate::protocol::{
        MachineAdd, MachineConnect, MachineList, MachineName, MachineRename, MachineRm,
        MachineTarget, SessionName,
    };

    type TestResult = Result<(), TestError>;

    fn name(value: &str) -> Result<MachineName, TestError> {
        Ok(MachineName::new(value)?)
    }

    #[tokio::test]
    async fn add_builds_each_target_with_its_options() -> TestResult {
        let (client, _peer) = connected().await?;
        let machines = HerdrCore::from_client(client).machines();
        let ssh = machines.add_ssh("user@box").session("work").temporary();
        let expected = MachineAdd::new(MachineTarget::Ssh {
            target: "user@box".to_owned(),
            name: None,
        })
        .session(SessionName::new("work")?)
        .temporary();
        assert_eq!(request_of(&ssh)?, &expected);
        let named = machines.add_ssh_as("box", "user@box");
        let expected = MachineAdd::new(MachineTarget::Ssh {
            target: "user@box".to_owned(),
            name: Some(name("box")?),
        });
        assert_eq!(request_of(&named)?, &expected);
        let via = machines.add_via("pod", "kubectl exec -i pod --");
        let expected = MachineAdd::new(MachineTarget::Via {
            name: name("pod")?,
            command: "kubectl exec -i pod --".to_owned(),
        });
        assert_eq!(request_of(&via)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn the_named_methods_build_their_requests() -> TestResult {
        let (client, _peer) = connected().await?;
        let machines = HerdrCore::from_client(client).machines();
        assert_eq!(request_of(&machines.list())?, &MachineList::new());
        assert_eq!(
            request_of(&machines.connect("box"))?,
            &MachineConnect::new(name("box")?)
        );
        assert_eq!(
            request_of(&machines.rename("box", "big"))?,
            &MachineRename::new(name("box")?, name("big")?)
        );
        assert_eq!(
            request_of(&machines.rm("box"))?,
            &MachineRm::new(name("box")?)
        );
        Ok(())
    }

    #[tokio::test]
    async fn empty_names_are_refused() -> TestResult {
        let (client, _peer) = connected().await?;
        let machines = HerdrCore::from_client(client).machines();
        let refused = [
            machines
                .connect("")
                .request()
                .err()
                .map(ToString::to_string),
            machines
                .rename("box", "")
                .request()
                .err()
                .map(ToString::to_string),
            machines
                .rename("", "big")
                .request()
                .err()
                .map(ToString::to_string),
            machines.rm("").request().err().map(ToString::to_string),
            machines
                .add_via("", "cmd")
                .request()
                .err()
                .map(ToString::to_string),
            machines
                .add_ssh_as("", "box")
                .request()
                .err()
                .map(ToString::to_string),
        ];
        for refusal in refused {
            assert_eq!(refusal.as_deref(), Some("machine name must not be empty"));
        }
        assert!(matches!(
            machines.add_ssh("box").session("").request(),
            Err(HerdrCoreError::Empty(error)) if error.what == "session name"
        ));
        Ok(())
    }

    #[tokio::test]
    async fn requests_are_sent_as_their_method() -> TestResult {
        let (client, mut peer) = connected().await?;
        let machines = HerdrCore::from_client(client).machines();
        let (sent, listed) = serve(&mut peer, machines.list(), json!({"machines": []})).await?;
        assert_eq!(sent["method"], "machine.list");
        assert!(listed?.machines.is_empty());
        let (sent, removed) =
            serve(&mut peer, machines.rm("box"), json!({"removed": "box"})).await?;
        assert_eq!(sent["method"], "machine.rm");
        assert_eq!(removed?.removed, "box");
        Ok(())
    }
}
