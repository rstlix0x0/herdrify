//! Lines: the plain-mode unit of a herdr-core connection, the decoder that cuts them, and the error
//! for text that is not one line.
//!
//! A connection starts in plain mode, one JSON text per line, and the hello reply that switches it
//! to frames is itself a line. The decoder yields those lines and, once the connection is framed,
//! hands every byte it has not consumed to a [`FrameDecoder`], so frames that arrived in the same
//! read as the hello reply are not lost.
//!
//! Responsibilities:
//!
//! - [`Line`], one JSON text without its terminating `\n`
//! - [`LineDecoder`], which buffers bytes in any chunking and yields complete lines
//! - the hand-off of unconsumed bytes to a [`FrameDecoder`]
//! - [`LineError`], which names where the offending newline sits in text that is not one line
//!
//! Non-responsibilities:
//!
//! - parsing a line as JSON, which the envelope does
//! - capping a line's length; the caller trusts the peer it chose, and the stream decoder that
//!   faces event data applies its own cap

use thiserror::Error;

use crate::protocol::FrameDecoder;

/// One JSON text, without the `\n` that ended it.
///
/// A `Line` never holds a `\n`, so a transport can write it and add the terminator itself.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Line, LineError};
///
/// # fn main() -> Result<(), LineError> {
/// let line = Line::new("{}")?;
/// assert_eq!(line.as_str(), "{}");
/// assert_eq!(line.into_string(), "{}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line(String);

impl Line {
    /// Wraps the text of one line. The empty text is a valid line.
    ///
    /// # Errors
    ///
    /// Returns [`LineError`] when the text holds a `\n`.
    pub fn new(text: impl Into<String>) -> Result<Self, LineError> {
        let text = text.into();
        text.find('\n')
            .map_or_else(|| Ok(Self(text)), |index| Err(LineError { index }))
    }

    /// The line's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The line's text, by value.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Cuts complete [`Line`]s out of a byte stream, splitting on `\n` only.
///
/// Push every chunk read, then call [`next_line`](Self::next_line) until it returns `None`. A line
/// is decoded as UTF-8, with each maximal invalid byte sequence replaced by one U+FFFD; herdr-core
/// writes valid UTF-8. Lines have no length limit.
///
/// After the hello reply the connection is framed. Stop calling `next_line` and call
/// [`into_frame_decoder`](Self::into_frame_decoder): the bytes after the reply line, including
/// ones that look like newlines, belong to frames.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::LineDecoder;
///
/// let mut decoder = LineDecoder::new();
/// decoder.push(b"{\"a\":1}\n{\"b\"");
/// assert_eq!(decoder.next_line().map(|line| line.into_string()), Some("{\"a\":1}".to_owned()));
/// assert_eq!(decoder.next_line(), None);
/// assert!(decoder.has_partial());
/// ```
#[derive(Debug, Default)]
pub struct LineDecoder {
    buffer: Vec<u8>,
    consumed: usize,
    complete: usize,
    partial: usize,
}

impl LineDecoder {
    /// An empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds bytes read from the connection, however they happen to be cut.
    pub fn push(&mut self, bytes: &[u8]) {
        let mut pieces = 0;
        let mut tail = 0;
        for piece in bytes.split(|&byte| byte == b'\n') {
            pieces += 1;
            tail = piece.len();
        }
        let newlines = usize::saturating_sub(pieces, 1);
        if newlines > 0 {
            self.complete += newlines;
            self.partial = tail;
        } else {
            self.partial += bytes.len();
        }
        self.buffer.extend_from_slice(bytes);
    }

    /// The next complete line, or `None` when no complete line is buffered.
    pub fn next_line(&mut self) -> Option<Line> {
        if self.complete == 0 {
            return None;
        }
        let pending = &self.buffer[self.consumed..];
        let end = pending.iter().position(|&byte| byte == b'\n')?;
        let line = String::from_utf8_lossy(&pending[..end]).into_owned();
        self.consumed += end + 1;
        self.complete -= 1;
        self.compact();
        Some(Line(line))
    }

    /// Whether a complete line is buffered.
    #[must_use]
    pub const fn has_line(&self) -> bool {
        self.complete > 0
    }

    /// Whether bytes of an unfinished line are buffered.
    #[must_use]
    pub const fn has_partial(&self) -> bool {
        self.partial > 0
    }

    /// How many bytes of the unfinished line are buffered.
    #[must_use]
    pub const fn partial_len(&self) -> usize {
        self.partial
    }

    /// Hands every byte not yet returned as a line to a [`FrameDecoder`].
    #[must_use]
    pub fn into_frame_decoder(mut self) -> FrameDecoder {
        FrameDecoder::with_bytes(self.buffer.split_off(self.consumed))
    }

    /// Drops the bytes of lines already returned, once they are most of the buffer.
    fn compact(&mut self) {
        if self.consumed == self.buffer.len() {
            self.buffer.clear();
            self.consumed = 0;
        } else if self.consumed > self.buffer.len() / 2 {
            self.buffer.drain(..self.consumed);
            self.consumed = 0;
        }
    }
}

/// Text given to [`Line::new`] held a newline.
///
/// `index` is the byte offset of the first `\n` in the text.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Line, LineError};
///
/// assert_eq!(Line::new("a\nb"), Err(LineError { index: 1 }));
/// assert_eq!(
///     LineError { index: 1 }.to_string(),
///     "a line must not contain a newline (found at byte 1)",
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a line must not contain a newline (found at byte {index})")]
pub struct LineError {
    /// The byte offset of the first newline in the refused text.
    pub index: usize,
}

#[cfg(test)]
mod tests {
    use super::{Line, LineDecoder, LineError};
    use crate::protocol::{Channel, Frame, FrameError, FrameKind};

    #[test]
    fn new_keeps_text_without_a_newline() -> Result<(), LineError> {
        assert_eq!(Line::new("{\"a\":1}")?.as_str(), "{\"a\":1}");
        assert_eq!(Line::new("a\r")?.as_str(), "a\r");
        Ok(())
    }

    #[test]
    fn new_allows_empty_text() -> Result<(), LineError> {
        assert_eq!(Line::new("")?.into_string(), "");
        Ok(())
    }

    #[test]
    fn new_refuses_a_newline_and_names_the_first_one() {
        assert_eq!(Line::new("\n"), Err(LineError { index: 0 }));
        assert_eq!(Line::new("ab\ncd\n"), Err(LineError { index: 2 }));
        assert_eq!(Line::new("{}\n".to_owned()), Err(LineError { index: 2 }));
    }

    #[test]
    fn yields_lines_across_chunks() {
        let mut decoder = LineDecoder::new();
        decoder.push(b"{\"a\":1}\n{\"b\"");
        assert_eq!(
            decoder.next_line().as_ref().map(Line::as_str),
            Some("{\"a\":1}")
        );
        assert_eq!(decoder.next_line(), None);
        decoder.push(b":2}\n");
        assert_eq!(
            decoder.next_line().map(Line::into_string),
            Some("{\"b\":2}".to_owned())
        );
        assert_eq!(decoder.next_line(), None);
    }

    #[test]
    fn hands_leftover_bytes_to_the_frame_decoder() -> Result<(), FrameError> {
        let frame = Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: b"{\"id\":1}".to_vec(),
        };
        let mut bytes = b"{\"id\":0,\"result\":true}\n".to_vec();
        bytes.extend(frame.encode()?);
        let mut decoder = LineDecoder::new();
        decoder.push(&bytes);
        assert_eq!(
            decoder.next_line().as_ref().map(Line::as_str),
            Some("{\"id\":0,\"result\":true}")
        );
        let mut frames = decoder.into_frame_decoder();
        assert_eq!(frames.next_frame()?, Some(frame));
        assert_eq!(frames.next_frame()?, None);
        Ok(())
    }

    #[test]
    fn reports_a_partial_line() {
        let mut decoder = LineDecoder::new();
        assert!(!decoder.has_partial());
        decoder.push(b"{\"a\"");
        assert!(decoder.has_partial());
        decoder.push(b":1}\n");
        assert!(decoder.next_line().is_some());
        assert!(!decoder.has_partial());
    }

    #[test]
    fn splits_on_newline_only() {
        let mut decoder = LineDecoder::new();
        decoder.push(b"a\r\nb\n");
        assert_eq!(decoder.next_line().as_ref().map(Line::as_str), Some("a\r"));
        assert_eq!(decoder.next_line().as_ref().map(Line::as_str), Some("b"));
    }

    #[test]
    fn an_empty_push_changes_nothing() {
        let mut decoder = LineDecoder::new();
        decoder.push(b"");
        assert!(!decoder.has_line());
        assert!(!decoder.has_partial());
        decoder.push(b"ab");
        decoder.push(b"");
        assert_eq!(decoder.partial_len(), 2);
    }

    #[test]
    fn counts_complete_lines_and_partial_bytes() {
        let mut decoder = LineDecoder::new();
        decoder.push(b"a\nb\ncc");
        assert!(decoder.has_line());
        assert_eq!(decoder.partial_len(), 2);
        assert!(decoder.next_line().is_some());
        assert!(decoder.next_line().is_some());
        assert!(!decoder.has_line());
        assert_eq!(decoder.partial_len(), 2);
    }

    #[test]
    fn display_names_the_byte_index() {
        let error = LineError { index: 7 };
        assert_eq!(
            error.to_string(),
            "a line must not contain a newline (found at byte 7)"
        );
    }

    #[test]
    fn line_error_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<LineError>();
    }
}
