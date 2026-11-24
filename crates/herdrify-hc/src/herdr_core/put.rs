//! `TerminalHandle::put_in_pieces`: stores a file for a terminal as several calls, in pieces of at
//! most 1 MiB, so a big file never holds up the connection.
//!
//! The method opens a transfer, sends the file in pieces of at most [`PIECE_BYTES`], closes the
//! transfer, and drops it when any step fails.
//!
//! Responsibilities:
//!
//! - [`TerminalHandle::put_in_pieces`], the call order and the rules for when to abort
//! - the split of a length into pieces, and telling the core's refusal from other errors
//!
//! Non-responsibilities:
//!
//! - the four calls themselves, which `terminals` builds and `connection` sends
//! - the size limit of a put, which the core enforces
//!
//! Tests here cover the split at the piece size and the test for a refusal. The requests that
//! reach the core, and the abort that follows a failure, are covered by `tests/client_fake.rs`.

use std::ops::Range;

use super::HerdrCoreError;
use super::terminals::{TerminalHandle, TerminalOps};
use crate::port::Transport;
use crate::protocol::{PutResult, TransferId};

/// The most bytes one piece carries: 1 MiB, the largest piece herdr-core 0.1.22 accepts. A piece
/// of 1048577 bytes is refused with "at most 1 MB per piece".
const PIECE_BYTES: usize = 1_048_576;

/// The slices, in order, of `len` bytes of at most [`PIECE_BYTES`] each. No bytes give no slice.
fn piece_ranges(len: usize) -> impl Iterator<Item = Range<usize>> {
    (0..len)
        .step_by(PIECE_BYTES)
        .map(move |start| start..len.min(start.saturating_add(PIECE_BYTES)))
}

/// Whether the core refused the call, as opposed to the call failing before or after the core
/// answered it.
fn refused_by_core(error: &HerdrCoreError) -> bool {
    error.refusal().is_some()
}

impl<T: Transport> TerminalHandle<T> {
    /// Stores `data` as a file called `name` for the terminal, sent in pieces of at most 1 MiB.
    ///
    /// This is `terminal.put_begin`, one `terminal.put_chunk` per piece in order, then
    /// `terminal.put_finish`, so the file is not limited by the size of one request. The core
    /// still enforces its limit of 8 MiB for a put, and refuses a file over it. The terminal must
    /// be on this core: a handle for `m/t7` is refused by the core with `machine_unknown`. Use
    /// [`put`](Self::put) for such a terminal.
    ///
    /// An empty `data` sends no piece. When a step fails after the core opened the transfer, the
    /// transfer is dropped with `terminal.put_abort` and its own failure, if any, is ignored. A
    /// `terminal.put_begin` that fails without the core refusing it, by a transport error, a
    /// closed connection or a reply that does not decode, is dropped the same way, since the core
    /// may have opened the transfer. A refusal of `terminal.put_begin` is returned as it is, since
    /// no transfer was opened. A future that is dropped sends no abort. The core drops the put
    /// when its terminal ends; its refusal for an unknown put says the same happens when the
    /// connection goes: "it was aborted, or its terminal or connection went".
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_hc::HerdrCore;
    ///
    /// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
    /// let core = HerdrCore::local().await?;
    /// let shell = core.terminals().handle("t1")?;
    /// let stored = shell.put_in_pieces("big.bin", vec![0_u8; 3 << 20]).await?;
    /// println!("stored {} bytes at {}", stored.bytes, stored.path);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// The first [`HerdrCoreError`] any of the calls returns, after the abort rules above.
    pub async fn put_in_pieces(
        &self,
        name: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Result<PutResult, HerdrCoreError> {
        let data = data.into();
        let transfer = TransferId::random();
        let ops = self.ops();
        let bytes = data.len() as u64;
        if let Err(error) = self.put_begin(name, bytes, transfer.clone()).await {
            if !refused_by_core(&error) {
                abort(&ops, transfer).await;
            }
            return Err(error);
        }
        match send_and_finish(&ops, &transfer, &data).await {
            Ok(stored) => Ok(stored),
            Err(error) => {
                abort(&ops, transfer).await;
                Err(error)
            }
        }
    }
}

/// Sends `data` of the open transfer `transfer` in pieces, then closes the transfer.
async fn send_and_finish<T: Transport>(
    ops: &TerminalOps<T>,
    transfer: &TransferId,
    data: &[u8],
) -> Result<PutResult, HerdrCoreError> {
    for range in piece_ranges(data.len()) {
        let offset = range.start as u64;
        ops.put_chunk(transfer.clone(), offset, &data[range])
            .await?;
    }
    ops.put_finish(transfer.clone()).await
}

/// Drops the open transfer `transfer`. Its failure is ignored: the caller already has an error to
/// return, and a transfer the core cannot abort is one it drops with the connection.
async fn abort<T: Transport>(ops: &TerminalOps<T>, transfer: TransferId) {
    let _ = ops.put_abort(transfer).await;
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::{PIECE_BYTES, piece_ranges, refused_by_core};
    use crate::connection::Error;
    use crate::herdr_core::HerdrCoreError;
    use crate::protocol::{EmptyValueError, MethodName, RpcError};

    type TestResult = Result<(), Box<dyn StdError>>;

    /// The start and end of every slice `piece_ranges` gives for `len` bytes.
    fn bounds(len: usize) -> Vec<(usize, usize)> {
        piece_ranges(len)
            .map(|range| (range.start, range.end))
            .collect()
    }

    #[test]
    fn piece_ranges_split_at_one_mebibyte() {
        assert_eq!(PIECE_BYTES, 1_048_576);
        assert!(bounds(0).is_empty());
        assert_eq!(bounds(1), [(0, 1)]);
        assert_eq!(bounds(1_048_576), [(0, 1_048_576)]);
        assert_eq!(bounds(1_048_577), [(0, 1_048_576), (1_048_576, 1_048_577)]);
    }

    #[test]
    fn refused_by_core_tells_rpc_from_other_errors() -> TestResult {
        let error: RpcError = serde_json::from_str(
            r#"{"code":-32000,"message":"unknown machine","data":{"kind":"machine_unknown"}}"#,
        )?;
        let rpc = HerdrCoreError::from(Error::Rpc {
            method: MethodName::new("terminal.put_begin")?,
            error,
        });
        assert!(refused_by_core(&rpc));

        let source = serde_json::from_str::<u8>("x")
            .err()
            .ok_or("x parsed as a number")?;
        let decode = HerdrCoreError::from(Error::Decode {
            method: MethodName::new("terminal.put_begin")?,
            source,
        });
        assert!(!refused_by_core(&decode));
        assert!(!refused_by_core(&HerdrCoreError::from(Error::Closed)));
        assert!(!refused_by_core(&HerdrCoreError::from(EmptyValueError {
            what: "file name"
        })));
        Ok(())
    }
}
