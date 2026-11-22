//! The NDJSON decoder: a stream channel's DATA bytes in, complete lines out.
//!
//! An event stream is a newline-delimited JSON byte stream carried inside DATA frames, but it is not
//! aligned to them: the core splits a line longer than 16384 bytes over several frames and packs
//! several short lines into one. The decoder joins and splits as needed so the caller sees only
//! whole lines.
//!
//! Responsibilities:
//!
//! - [`NdjsonDecoder`], which carries a partial line across pushes and yields every complete line
//! - refusing an unfinished line that passes 16777216 bytes
//!
//! Non-responsibilities:
//!
//! - parsing a line as JSON, which the envelope does
//! - reading frames or granting credit, which the client does

use super::{Line, LineDecoder, MAX_PAYLOAD_LEN, ProtocolError};

/// Turns a stream channel's DATA bytes into complete [`Line`]s.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Line, NdjsonDecoder, ProtocolError};
///
/// # fn main() -> Result<(), ProtocolError> {
/// let mut decoder = NdjsonDecoder::new();
/// decoder.push(b"{\"seq\":1}\n{\"se")?;
/// assert_eq!(decoder.next_line().map(Line::into_string), Some("{\"seq\":1}".to_owned()));
/// assert_eq!(decoder.next_line(), None);
/// decoder.push(b"q\":2}\n")?;
/// assert_eq!(decoder.next_line().map(Line::into_string), Some("{\"seq\":2}".to_owned()));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct NdjsonDecoder {
    lines: LineDecoder,
}

impl NdjsonDecoder {
    /// An empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the bytes of one DATA frame.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::EventLineTooLong`] when the unfinished line, the bytes after the
    /// last newline, is over 16777216 bytes. Complete lines buffered before it are not counted.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), ProtocolError> {
        self.lines.push(bytes);
        if self.lines.partial_len() > MAX_PAYLOAD_LEN {
            return Err(ProtocolError::EventLineTooLong);
        }
        Ok(())
    }

    /// The next complete line, or `None` when no complete line is buffered.
    pub fn next_line(&mut self) -> Option<Line> {
        self.lines.next_line()
    }

    /// Whether a complete line is buffered.
    #[must_use]
    pub const fn has_line(&self) -> bool {
        self.lines.has_line()
    }
}

#[cfg(test)]
mod tests {
    use super::NdjsonDecoder;
    use crate::protocol::{Line, ProtocolError};

    fn long_line(len: usize) -> Vec<u8> {
        let mut bytes = vec![b'a'; len];
        bytes.push(b'\n');
        bytes
    }

    #[test]
    fn ndjson_joins_a_line_split_across_frames() -> Result<(), ProtocolError> {
        let bytes = long_line(20_000);
        let mut decoder = NdjsonDecoder::new();
        decoder.push(&bytes[..16_384])?;
        assert_eq!(decoder.next_line(), None);
        decoder.push(&bytes[16_384..])?;
        let line = decoder.next_line();
        assert_eq!(line.as_ref().map(|line| line.as_str().len()), Some(20_000));
        assert_eq!(decoder.next_line(), None);
        Ok(())
    }

    #[test]
    fn ndjson_yields_packed_lines() -> Result<(), ProtocolError> {
        let mut decoder = NdjsonDecoder::new();
        decoder.push(b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}\n")?;
        assert!(decoder.has_line());
        assert_eq!(
            decoder.next_line().as_ref().map(Line::as_str),
            Some("{\"a\":1}")
        );
        assert_eq!(
            decoder.next_line().as_ref().map(Line::as_str),
            Some("{\"b\":2}")
        );
        assert_eq!(
            decoder.next_line().as_ref().map(Line::as_str),
            Some("{\"c\":3}")
        );
        assert_eq!(decoder.next_line(), None);
        assert!(!decoder.has_line());
        Ok(())
    }

    #[test]
    fn ndjson_refuses_a_partial_line_over_16_mib() -> Result<(), ProtocolError> {
        let mut at_limit = NdjsonDecoder::new();
        at_limit.push(&vec![b'a'; 16_777_216])?;

        let mut over = NdjsonDecoder::new();
        assert!(matches!(
            over.push(&vec![b'a'; 16_777_217]),
            Err(ProtocolError::EventLineTooLong)
        ));
        Ok(())
    }

    #[test]
    fn ndjson_cap_counts_only_the_unfinished_line() -> Result<(), ProtocolError> {
        let mut decoder = NdjsonDecoder::new();
        let mut bytes = long_line(16_777_216);
        bytes.extend_from_slice(b"{}");
        decoder.push(&bytes)?;
        assert_eq!(
            decoder.next_line().map(|line| line.as_str().len()),
            Some(16_777_216)
        );
        Ok(())
    }
}
