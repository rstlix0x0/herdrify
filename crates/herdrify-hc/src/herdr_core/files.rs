//! The file methods: read, write and edit files on the core's machine or another one.
//!
//! Responsibilities:
//!
//! - [`FileOps`] and the options of `file.read`, `file.write` and `file.edit`
//!
//! Non-responsibilities:
//!
//! - `terminal.put`, which stores a file for one terminal and is on
//!   [`TerminalHandle`](super::TerminalHandle)

use std::fmt;

use super::call::{Call, invoke_by};
use crate::adapter::ProcessTransport;
use crate::connection::Client;
use crate::port::Transport;
use crate::protocol::{
    Edit, FileEdit, FileRead, FileReadResult, FileResult, FileWrite, MachineName, RemotePath,
};

/// The methods that read, write and edit files.
///
/// Reached through [`HerdrCore::files`](super::HerdrCore::files). A relative path starts at the
/// core's working directory unless `cwd` names another.
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// let files = core.files();
/// files.write("/tmp/notes.txt", "first line\nsecond line\n").await?;
/// files.edit("/tmp/notes.txt", [("second line", "2nd line")]).await?;
/// let line = files.read("/tmp/notes.txt").offset(2).limit(1).await?;
/// assert_eq!(line.text, "2nd line\n");
/// # Ok(())
/// # }
/// ```
pub struct FileOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

impl<T: Transport> FileOps<T> {
    /// The file methods over `client`.
    pub(super) const fn new(client: Client<T>) -> Self {
        Self { client }
    }

    /// `file.read`: the text of `path`.
    pub fn read(&self, path: impl Into<String>) -> Call<T, FileRead> {
        Call::new(
            self.client.clone(),
            Ok(FileRead::new(RemotePath::new(path))),
        )
    }

    /// `file.write`: replaces the content of `path` with `data`.
    pub fn write(&self, path: impl Into<String>, data: impl Into<Vec<u8>>) -> Call<T, FileWrite> {
        let params = FileWrite::new(RemotePath::new(path), data.into());
        Call::new(self.client.clone(), Ok(params))
    }

    /// `file.edit`: replaces each `old` text in `path` with its `new` text, in order.
    pub fn edit<O: Into<String>, N: Into<String>>(
        &self,
        path: impl Into<String>,
        edits: impl IntoIterator<Item = (O, N)>,
    ) -> Call<T, FileEdit> {
        let edits = edits
            .into_iter()
            .map(|(old, new)| Edit {
                old: old.into(),
                new: new.into(),
            })
            .collect();
        let params = FileEdit::new(RemotePath::new(path), edits);
        Call::new(self.client.clone(), Ok(params))
    }
}

impl<T: Transport> fmt::Debug for FileOps<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileOps").finish_non_exhaustive()
    }
}

impl<T: Transport> Call<T, FileRead> {
    /// Where a relative path starts.
    pub fn cwd(self, cwd: impl Into<String>) -> Self {
        self.with(|read| read.cwd(RemotePath::new(cwd)))
    }

    /// At most this many lines.
    pub fn limit(self, lines: u64) -> Self {
        self.with(|read| read.limit(lines))
    }

    /// The first line to give, counting from 1.
    pub fn offset(self, line: u64) -> Self {
        self.with(|read| read.offset(line))
    }

    /// Reads on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|read| Ok(read.on(machine?)))
    }
}

impl<T: Transport> Call<T, FileWrite> {
    /// Where a relative path starts.
    pub fn cwd(self, cwd: impl Into<String>) -> Self {
        self.with(|write| write.cwd(RemotePath::new(cwd)))
    }

    /// Writes on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|write| Ok(write.on(machine?)))
    }
}

impl<T: Transport> Call<T, FileEdit> {
    /// Where a relative path starts.
    pub fn cwd(self, cwd: impl Into<String>) -> Self {
        self.with(|edit| edit.cwd(RemotePath::new(cwd)))
    }

    /// Edits on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|edit| Ok(edit.on(machine?)))
    }
}

invoke_by! {
    FileRead => FileReadResult, file_read;
    FileWrite => FileResult, file_write;
    FileEdit => FileResult, file_edit;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::herdr_core::fake::{TestError, connected, request_of, serve};
    use crate::herdr_core::{HerdrCore, HerdrCoreError};
    use crate::protocol::{Edit, FileEdit, FileRead, FileWrite, MachineName, RemotePath};

    type TestResult = Result<(), TestError>;

    #[tokio::test]
    async fn each_method_builds_its_request_with_every_option() -> TestResult {
        let (client, _peer) = connected().await?;
        let files = HerdrCore::from_client(client).files();
        let box_ = MachineName::new("box")?;
        let read = files.read("a.txt").cwd("/w").limit(1).offset(2).on("box");
        let expected = FileRead::new(RemotePath::new("a.txt"))
            .cwd(RemotePath::new("/w"))
            .limit(1)
            .offset(2)
            .on(box_.clone());
        assert_eq!(request_of(&read)?, &expected);
        let write = files.write("a.txt", "hi").cwd("/w").on("box");
        let expected = FileWrite::new(RemotePath::new("a.txt"), b"hi".to_vec())
            .cwd(RemotePath::new("/w"))
            .on(box_.clone());
        assert_eq!(request_of(&write)?, &expected);
        let edit = files
            .edit("a.txt", [("a", "b"), ("c", "d")])
            .cwd("/w")
            .on("box");
        let edits = vec![
            Edit {
                old: "a".to_owned(),
                new: "b".to_owned(),
            },
            Edit {
                old: "c".to_owned(),
                new: "d".to_owned(),
            },
        ];
        let expected = FileEdit::new(RemotePath::new("a.txt"), edits)
            .cwd(RemotePath::new("/w"))
            .on(box_);
        assert_eq!(request_of(&edit)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn an_empty_machine_is_refused() -> TestResult {
        let (client, _peer) = connected().await?;
        let files = HerdrCore::from_client(client).files();
        for call in [
            files
                .read("a")
                .on("")
                .request()
                .err()
                .map(ToString::to_string),
            files
                .write("a", "")
                .on("")
                .request()
                .err()
                .map(ToString::to_string),
            files
                .edit("a", [("x", "y")])
                .on("")
                .request()
                .err()
                .map(ToString::to_string),
        ] {
            assert_eq!(call.as_deref(), Some("machine name must not be empty"));
        }
        assert!(matches!(
            files.read("a").on("").request(),
            Err(HerdrCoreError::Empty(_))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn requests_are_sent_as_their_method() -> TestResult {
        let (client, mut peer) = connected().await?;
        let files = HerdrCore::from_client(client).files();
        let wrote = json!({"path": "/w/a.txt", "bytes": 2});
        let (sent, written) = serve(&mut peer, files.write("a.txt", "hi"), wrote.clone()).await?;
        assert_eq!(sent["method"], "file.write");
        assert_eq!(written?.bytes, 2);
        let (sent, _) = serve(&mut peer, files.edit("a.txt", [("h", "H")]), wrote).await?;
        assert_eq!(sent["method"], "file.edit");
        let text = json!({"text": "hi", "first": 1, "last": 1, "lines": 1});
        let (sent, read) = serve(&mut peer, files.read("a.txt"), text).await?;
        assert_eq!(sent["method"], "file.read");
        assert_eq!(read?.text, "hi");
        Ok(())
    }
}
