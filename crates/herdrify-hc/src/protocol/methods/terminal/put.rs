//! The requests that send a file to a terminal, whole or in pieces.
//!
//! File data travels as base64 text, so `TerminalPut` and `TerminalPutChunk` hold the raw bytes and
//! encode them when they are serialized.
//!
//! Responsibilities:
//!
//! - the `Params` type of `terminal.put`, with its wire name and reply type
//! - the `Params` types of the four calls that send a file in pieces: `terminal.put_begin`,
//!   `terminal.put_chunk`, `terminal.put_finish` and `terminal.put_abort`
//!
//! Non-responsibilities:
//!
//! - the requests that read from, attach to, send to or wait on a terminal, which `interaction`
//!   holds
//! - sending the request or reading the reply, which the client does
//! - deciding which names or sizes the core accepts, which the core decides

use serde::Serialize;

use crate::protocol::{
    EmptyResult, FileName, Method, PutBeginResult, PutPieceResult, PutResult, TerminalRef,
    TransferId,
};

/// The parameters of `terminal.put`.
///
/// The data is sent as base64 text.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{FileName, Method, TerminalPut, TerminalRef};
///
/// let put = TerminalPut::new(
///     TerminalRef::new("t7")?,
///     FileName::new("a.txt"),
///     b"hello".to_vec(),
/// );
/// assert_eq!(TerminalPut::NAME, "terminal.put");
/// assert_eq!(serde_json::to_value(&put)?["data"], "aGVsbG8=");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPut {
    terminal: TerminalRef,
    name: FileName,
    #[serde(serialize_with = "crate::protocol::base64::serialize")]
    data: Vec<u8>,
}

impl TerminalPut {
    /// A `terminal.put` request that stores `data` in `terminal` under `name`.
    #[must_use]
    pub const fn new(terminal: TerminalRef, name: FileName, data: Vec<u8>) -> Self {
        Self {
            terminal,
            name,
            data,
        }
    }
}

impl Method for TerminalPut {
    const NAME: &'static str = "terminal.put";
    type Params = Self;
    type Output = PutResult;
}

/// The parameters of `terminal.put_begin`, which opens a transfer sent in pieces.
///
/// The terminal must be on this core. For a terminal on another machine, send the whole file with
/// [`TerminalPut`].
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{FileName, Method, TerminalPutBegin, TerminalRef, TransferId};
///
/// let begin = TerminalPutBegin::new(
///     TerminalRef::new("t7")?,
///     FileName::new("a.png"),
///     5,
///     TransferId::new("0123456789abcdef")?,
/// );
/// assert_eq!(TerminalPutBegin::NAME, "terminal.put_begin");
/// assert_eq!(serde_json::to_value(&begin)?["bytes"], 5);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPutBegin {
    terminal: TerminalRef,
    name: FileName,
    bytes: u64,
    transfer: TransferId,
}

impl TerminalPutBegin {
    /// A `terminal.put_begin` request that opens `transfer` to store `bytes` bytes in `terminal`
    /// under `name`.
    #[must_use]
    pub const fn new(
        terminal: TerminalRef,
        name: FileName,
        bytes: u64,
        transfer: TransferId,
    ) -> Self {
        Self {
            terminal,
            name,
            bytes,
            transfer,
        }
    }
}

impl Method for TerminalPutBegin {
    const NAME: &'static str = "terminal.put_begin";
    type Params = Self;
    type Output = PutBeginResult;
}

/// The parameters of `terminal.put_chunk`, which sends one piece of an open transfer.
///
/// The data is sent as base64 text.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Method, TerminalPutChunk, TransferId};
///
/// let chunk = TerminalPutChunk::new(TransferId::new("0123456789abcdef")?, 0, b"hello".to_vec());
/// assert_eq!(TerminalPutChunk::NAME, "terminal.put_chunk");
/// assert_eq!(serde_json::to_value(&chunk)?["data"], "aGVsbG8=");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPutChunk {
    transfer: TransferId,
    offset: u64,
    #[serde(serialize_with = "crate::protocol::base64::serialize")]
    data: Vec<u8>,
}

impl TerminalPutChunk {
    /// A `terminal.put_chunk` request that sends `data` for `transfer`, starting `offset` bytes
    /// into the file.
    #[must_use]
    pub const fn new(transfer: TransferId, offset: u64, data: Vec<u8>) -> Self {
        Self {
            transfer,
            offset,
            data,
        }
    }
}

impl Method for TerminalPutChunk {
    const NAME: &'static str = "terminal.put_chunk";
    type Params = Self;
    type Output = PutPieceResult;
}

/// The parameters of `terminal.put_finish`, which closes a transfer and stores the file.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Method, TerminalPutFinish, TransferId};
///
/// let finish = TerminalPutFinish::new(TransferId::new("0123456789abcdef")?);
/// assert_eq!(TerminalPutFinish::NAME, "terminal.put_finish");
/// assert_eq!(
///     serde_json::to_string(&finish)?,
///     r#"{"transfer":"0123456789abcdef"}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPutFinish {
    transfer: TransferId,
}

impl TerminalPutFinish {
    /// A `terminal.put_finish` request that closes `transfer`.
    #[must_use]
    pub const fn new(transfer: TransferId) -> Self {
        Self { transfer }
    }
}

impl Method for TerminalPutFinish {
    const NAME: &'static str = "terminal.put_finish";
    type Params = Self;
    type Output = PutResult;
}

/// The parameters of `terminal.put_abort`, which drops a transfer and what it received.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Method, TerminalPutAbort, TransferId};
///
/// let abort = TerminalPutAbort::new(TransferId::new("0123456789abcdef")?);
/// assert_eq!(TerminalPutAbort::NAME, "terminal.put_abort");
/// assert_eq!(
///     serde_json::to_string(&abort)?,
///     r#"{"transfer":"0123456789abcdef"}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPutAbort {
    transfer: TransferId,
}

impl TerminalPutAbort {
    /// A `terminal.put_abort` request that drops `transfer`.
    #[must_use]
    pub const fn new(transfer: TransferId) -> Self {
        Self { transfer }
    }
}

impl Method for TerminalPutAbort {
    const NAME: &'static str = "terminal.put_abort";
    type Params = Self;
    type Output = EmptyResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use crate::protocol::{
        FileName, Method, TerminalPut, TerminalPutAbort, TerminalPutBegin, TerminalPutChunk,
        TerminalPutFinish, TerminalRef, TransferId,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn put_begin_serializes_its_four_params() -> TestResult {
        let begin = TerminalPutBegin::new(
            TerminalRef::new("t7")?,
            FileName::new("a.png"),
            42,
            TransferId::new("0123456789abcdef")?,
        );
        assert_eq!(
            serde_json::to_value(begin)?,
            json!({
                "terminal": "t7",
                "name": "a.png",
                "bytes": 42,
                "transfer": "0123456789abcdef",
            })
        );
        assert_eq!(TerminalPutBegin::NAME, "terminal.put_begin");
        Ok(())
    }

    #[test]
    fn put_chunk_encodes_data_as_base64() -> TestResult {
        let chunk =
            TerminalPutChunk::new(TransferId::new("0123456789abcdef")?, 10, b"hello".to_vec());
        assert_eq!(
            serde_json::to_value(chunk)?,
            json!({"transfer": "0123456789abcdef", "offset": 10, "data": "aGVsbG8="})
        );
        assert_eq!(TerminalPutChunk::NAME, "terminal.put_chunk");
        Ok(())
    }

    #[test]
    fn put_finish_and_abort_send_only_the_transfer() -> TestResult {
        let id = TransferId::new("0123456789abcdef")?;
        assert_eq!(
            serde_json::to_value(TerminalPutFinish::new(id.clone()))?,
            json!({"transfer": "0123456789abcdef"})
        );
        assert_eq!(
            serde_json::to_value(TerminalPutAbort::new(id))?,
            json!({"transfer": "0123456789abcdef"})
        );
        assert_eq!(TerminalPutFinish::NAME, "terminal.put_finish");
        assert_eq!(TerminalPutAbort::NAME, "terminal.put_abort");
        Ok(())
    }

    #[test]
    fn terminal_put_encodes_data() -> TestResult {
        let put = TerminalPut::new(
            TerminalRef::new("t7")?,
            FileName::new("a.png"),
            b"x\n".to_vec(),
        );
        assert_eq!(
            serde_json::to_value(put)?,
            json!({"terminal": "t7", "name": "a.png", "data": "eAo="})
        );
        assert_eq!(TerminalPut::NAME, "terminal.put");
        Ok(())
    }
}
