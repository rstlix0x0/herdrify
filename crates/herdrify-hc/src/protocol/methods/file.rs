//! The requests that read and change files on a machine.
//!
//! Paths refer to the machine the core runs on, or to the machine `on` names, not to the local
//! machine.
//!
//! Responsibilities:
//!
//! - [`FileRead`], [`FileWrite`] and [`FileEdit`], each with its wire name and reply type
//!
//! Non-responsibilities:
//!
//! - sending the request or reading the reply, which the client does
//! - deciding which paths the core accepts, which the core decides

use serde::Serialize;

use crate::protocol::{Edit, FileReadResult, FileResult, MachineName, Method, RemotePath};

/// The parameters of `file.read`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileRead {
    path: RemotePath,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
}

impl FileRead {
    /// A `file.read` request for `path`.
    #[must_use]
    pub const fn new(path: RemotePath) -> Self {
        Self {
            path,
            cwd: None,
            limit: None,
            offset: None,
            on: None,
        }
    }

    /// Where a relative path starts.
    #[must_use]
    pub fn cwd(mut self, cwd: RemotePath) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// At most this many lines.
    #[must_use]
    pub const fn limit(mut self, lines: u64) -> Self {
        self.limit = Some(lines);
        self
    }

    /// The first line to give, counting from 1.
    #[must_use]
    pub const fn offset(mut self, line: u64) -> Self {
        self.offset = Some(line);
        self
    }

    /// Reads on another machine.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }
}

impl Method for FileRead {
    const NAME: &'static str = "file.read";
    type Params = Self;
    type Output = FileReadResult;
}

/// The parameters of `file.write`.
///
/// The data is sent as base64 text. Nothing else is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileWrite {
    path: RemotePath,
    #[serde(serialize_with = "crate::protocol::base64::serialize")]
    data: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
}

impl FileWrite {
    /// A `file.write` request that replaces the content of `path` with `data`.
    #[must_use]
    pub const fn new(path: RemotePath, data: Vec<u8>) -> Self {
        Self {
            path,
            data,
            cwd: None,
            on: None,
        }
    }

    /// Where a relative path starts.
    #[must_use]
    pub fn cwd(mut self, cwd: RemotePath) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Writes on another machine.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }
}

impl Method for FileWrite {
    const NAME: &'static str = "file.write";
    type Params = Self;
    type Output = FileResult;
}

/// The parameters of `file.edit`.
///
/// Every edit is made or none is. Nothing is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileEdit {
    path: RemotePath,
    edits: Vec<Edit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
}

impl FileEdit {
    /// A `file.edit` request that makes `edits` in `path`.
    #[must_use]
    pub const fn new(path: RemotePath, edits: Vec<Edit>) -> Self {
        Self {
            path,
            edits,
            cwd: None,
            on: None,
        }
    }

    /// Where a relative path starts.
    #[must_use]
    pub fn cwd(mut self, cwd: RemotePath) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Edits on another machine.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }
}

impl Method for FileEdit {
    const NAME: &'static str = "file.edit";
    type Params = Self;
    type Output = FileResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use crate::protocol::{Edit, FileEdit, FileRead, FileWrite, Method, RemotePath};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn file_write_encodes_data() -> TestResult {
        let write = FileWrite::new(RemotePath::new("f.txt"), b"x\n".to_vec());
        assert_eq!(
            serde_json::to_value(write)?,
            json!({"path": "f.txt", "data": "eAo="})
        );
        assert_eq!(FileWrite::NAME, "file.write");
        Ok(())
    }

    #[test]
    fn file_read_and_edit_required_only() -> TestResult {
        assert_eq!(
            serde_json::to_value(FileRead::new(RemotePath::new("f.txt")))?,
            json!({"path": "f.txt"})
        );
        let edit = FileEdit::new(
            RemotePath::new("f.txt"),
            vec![Edit {
                old: "a".to_owned(),
                new: "b".to_owned(),
            }],
        );
        assert_eq!(
            serde_json::to_value(edit)?,
            json!({"path": "f.txt", "edits": [{"old": "a", "new": "b"}]})
        );
        assert_eq!(FileRead::NAME, "file.read");
        assert_eq!(FileEdit::NAME, "file.edit");
        Ok(())
    }
}
