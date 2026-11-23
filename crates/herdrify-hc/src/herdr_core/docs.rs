//! The doc methods: read and write the JSON documents the core keeps by name.
//!
//! Responsibilities:
//!
//! - [`DocOps`] and the options of `doc.get` and `doc.set`
//!
//! Non-responsibilities:
//!
//! - files on a machine, which [`FileOps`](super::FileOps) reads and writes

use std::fmt;

use serde_json::Value;

use super::HerdrCoreError;
use super::call::{Call, invoke_by};
use crate::adapter::ProcessTransport;
use crate::connection::Client;
use crate::port::Transport;
use crate::protocol::{
    DocGet, DocGetResult, DocName, DocSet, DocSetResult, DocVersion, MachineName,
};

/// The methods that read and write the core's docs.
///
/// Reached through [`HerdrCore::docs`](super::HerdrCore::docs). An empty doc name is reported by
/// the `.await` as [`HerdrCoreError::Empty`].
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
/// use serde_json::json;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// let current = core.docs().get("settings").await?;
/// core.docs()
///     .set("settings", json!({"theme": "dark"}), current.version.get())
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct DocOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

impl<T: Transport> DocOps<T> {
    /// The doc methods over `client`.
    pub(super) const fn new(client: Client<T>) -> Self {
        Self { client }
    }

    /// `doc.get`: the doc called `name` and its version. A name never written is `null` at
    /// version 0.
    pub fn get(&self, name: impl Into<String>) -> Call<T, DocGet> {
        let params = doc_name(name).map(DocGet::new);
        Call::new(self.client.clone(), params)
    }

    /// `doc.set`: stores `value` as `name` when `version` is its current version. `null` deletes
    /// the doc, and a name never written is at version 0.
    pub fn set(&self, name: impl Into<String>, value: Value, version: u64) -> Call<T, DocSet> {
        let params = doc_name(name).map(|name| DocSet::new(name, value, DocVersion(version)));
        Call::new(self.client.clone(), params)
    }
}

impl<T: Transport> fmt::Debug for DocOps<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocOps").finish_non_exhaustive()
    }
}

/// A doc name from a string, refusing an empty one.
fn doc_name(name: impl Into<String>) -> Result<DocName, HerdrCoreError> {
    Ok(DocName::new(name)?)
}

impl<T: Transport> Call<T, DocGet> {
    /// Reads on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|get| Ok(get.on(machine?)))
    }
}

impl<T: Transport> Call<T, DocSet> {
    /// Writes on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|set| Ok(set.on(machine?)))
    }
}

invoke_by! {
    DocGet => DocGetResult, doc_get;
    DocSet => DocSetResult, doc_set;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::herdr_core::HerdrCore;
    use crate::herdr_core::fake::{TestError, connected, request_of, serve};
    use crate::protocol::{DocGet, DocName, DocSet, DocVersion, MachineName};

    type TestResult = Result<(), TestError>;

    #[tokio::test]
    async fn get_and_set_build_their_requests() -> TestResult {
        let (client, _peer) = connected().await?;
        let docs = HerdrCore::from_client(client).docs();
        let name = DocName::new("settings")?;
        let box_ = MachineName::new("box")?;
        assert_eq!(
            request_of(&docs.get("settings").on("box"))?,
            &DocGet::new(name.clone()).on(box_.clone())
        );
        let set = docs.set("settings", json!({"a": 1}), 3).on("box");
        let expected = DocSet::new(name, json!({"a": 1}), DocVersion(3)).on(box_);
        assert_eq!(request_of(&set)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn empty_names_are_refused() -> TestResult {
        let (client, _peer) = connected().await?;
        let docs = HerdrCore::from_client(client).docs();
        for refusal in [
            docs.get("").request().err().map(ToString::to_string),
            docs.set("", json!(null), 0)
                .request()
                .err()
                .map(ToString::to_string),
            docs.get("a")
                .on("")
                .request()
                .err()
                .map(ToString::to_string),
            docs.set("a", json!(null), 0)
                .on("")
                .request()
                .err()
                .map(ToString::to_string),
        ] {
            assert!(refusal.is_some_and(|message| message.ends_with("must not be empty")));
        }
        Ok(())
    }

    #[tokio::test]
    async fn requests_are_sent_as_their_method() -> TestResult {
        let (client, mut peer) = connected().await?;
        let docs = HerdrCore::from_client(client).docs();
        let (sent, read_back) = serve(
            &mut peer,
            docs.get("settings"),
            json!({"value": null, "version": 0}),
        )
        .await?;
        assert_eq!(sent["method"], "doc.get");
        assert_eq!(read_back?.version, DocVersion(0));
        let (sent, stored) = serve(
            &mut peer,
            docs.set("settings", json!(1), 0),
            json!({"version": 1}),
        )
        .await?;
        assert_eq!(sent["method"], "doc.set");
        assert_eq!(stored?.version, DocVersion(1));
        Ok(())
    }
}
