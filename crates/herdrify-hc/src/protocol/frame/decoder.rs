//! The frame decoder: bytes in any chunking in, whole frames out.
//!
//! A read from a connection can end anywhere: inside a header, inside a payload, or after several
//! frames. The decoder buffers what it is given and hands back a frame only once all of it has
//! arrived, so callers never see a partial frame and never track offsets themselves.
//!
//! Responsibilities:
//!
//! - [`FrameDecoder`], which buffers bytes and yields complete frames in order
//! - refusing a header that announces a payload over 16777216 bytes, for good
//!
//! Non-responsibilities:
//!
//! - reading the bytes from a connection, which the transport does
//! - interpreting a frame's payload, which the client does

use super::wire::{HEADER_LEN, MAX_PAYLOAD_LEN};
use super::{Channel, Frame, FrameError, FrameKind};

/// Cuts complete [`Frame`]s out of a byte stream.
///
/// Push every chunk read, then call [`next_frame`](Self::next_frame) until it returns `Ok(None)`.
/// A header announcing a payload over 16777216 bytes makes the decoder unusable: that call and every
/// later one return the same [`FrameError::TooLarge`], because the bytes that follow can no longer
/// be told apart from payload.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Channel, Frame, FrameDecoder, FrameError, FrameKind};
///
/// # fn main() -> Result<(), FrameError> {
/// let frame = Frame {
///     channel: Channel::RPC,
///     kind: FrameKind::Data,
///     payload: b"{}".to_vec(),
/// };
/// let bytes = frame.encode()?;
///
/// let mut decoder = FrameDecoder::new();
/// decoder.push(&bytes[..4]);
/// assert_eq!(decoder.next_frame()?, None);
/// decoder.push(&bytes[4..]);
/// assert_eq!(decoder.next_frame()?, Some(frame));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    consumed: usize,
    refused: Option<usize>,
}

impl FrameDecoder {
    /// An empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A decoder that starts with bytes already read from the connection.
    #[must_use]
    pub fn with_bytes(bytes: Vec<u8>) -> Self {
        Self {
            buffer: bytes,
            ..Self::default()
        }
    }

    /// Adds bytes read from the connection, however they happen to be cut.
    ///
    /// A decoder that has refused a header ignores every later push, so it holds no more bytes than
    /// it had when it refused.
    pub fn push(&mut self, bytes: &[u8]) {
        if self.refused.is_none() {
            self.buffer.extend_from_slice(bytes);
        }
    }

    /// The next complete frame, or `None` when more bytes are needed.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::TooLarge`] when a header announces a payload over 16777216 bytes, and
    /// returns it again on every later call.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, FrameError> {
        if let Some(len) = self.refused {
            return Err(FrameError::TooLarge { len });
        }
        let pending = &self.buffer[self.consumed..];
        let Some(header) = pending.first_chunk::<HEADER_LEN>() else {
            return Ok(None);
        };
        let [c0, c1, c2, c3, kind, l0, l1, l2, l3] = *header;
        let len = u32::from_le_bytes([l0, l1, l2, l3]) as usize;
        if len > MAX_PAYLOAD_LEN {
            self.refused = Some(len);
            return Err(FrameError::TooLarge { len });
        }
        let Some(payload) = pending.get(HEADER_LEN..HEADER_LEN + len) else {
            return Ok(None);
        };
        let frame = Frame {
            channel: Channel::new(u32::from_le_bytes([c0, c1, c2, c3])),
            kind: FrameKind::from(kind),
            payload: payload.to_vec(),
        };
        self.consumed += HEADER_LEN + len;
        self.compact();
        Ok(Some(frame))
    }

    /// Whether bytes of an unfinished frame are buffered.
    #[must_use]
    pub const fn has_partial(&self) -> bool {
        self.consumed < self.buffer.len()
    }

    /// Drops the bytes of frames already returned, once they are most of the buffer.
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

#[cfg(test)]
mod tests {
    use super::FrameDecoder;
    use crate::protocol::{Channel, Frame, FrameError, FrameKind};

    fn data(channel: u32, payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::new(channel),
            kind: FrameKind::Data,
            payload: payload.to_vec(),
        }
    }

    fn header(len: u32) -> Vec<u8> {
        let mut bytes = vec![0, 0, 0, 0, 0];
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes
    }

    #[test]
    fn decodes_a_frame_split_across_every_byte() -> Result<(), FrameError> {
        let frame = data(4, b"hello");
        let mut decoder = FrameDecoder::new();
        let mut frames = Vec::new();
        for byte in frame.encode()? {
            decoder.push(&[byte]);
            while let Some(found) = decoder.next_frame()? {
                frames.push(found);
            }
        }
        assert_eq!(frames, [frame]);
        Ok(())
    }

    #[test]
    fn decodes_several_frames_from_one_push() -> Result<(), FrameError> {
        let first = data(0, b"{}");
        let second = Frame::credit(Channel::new(2), 9);
        let mut bytes = first.encode()?;
        bytes.extend(second.encode()?);
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes);
        assert_eq!(decoder.next_frame()?, Some(first));
        assert_eq!(decoder.next_frame()?, Some(second));
        assert_eq!(decoder.next_frame()?, None);
        Ok(())
    }

    #[test]
    fn oversized_header_poisons_the_decoder() -> Result<(), FrameError> {
        let mut decoder = FrameDecoder::new();
        decoder.push(&header(16_777_217));
        let refused = Err(FrameError::TooLarge { len: 16_777_217 });
        assert_eq!(decoder.next_frame(), refused);
        decoder.push(&data(0, b"{}").encode()?);
        assert_eq!(decoder.next_frame(), refused);
        Ok(())
    }

    #[test]
    fn a_refused_decoder_does_not_buffer_later_pushes() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&header(16_777_217));
        let refused = Err(FrameError::TooLarge { len: 16_777_217 });
        assert_eq!(decoder.next_frame(), refused);
        let held = decoder.buffer.len();
        decoder.push(&[0; 4096]);
        assert_eq!(decoder.buffer.len(), held);
        assert_eq!(decoder.next_frame(), refused);
    }

    #[test]
    fn accepts_a_16777216_byte_payload() -> Result<(), FrameError> {
        let frame = data(1, &vec![7; 16_777_216]);
        let mut decoder = FrameDecoder::new();
        decoder.push(&frame.encode()?);
        assert_eq!(decoder.next_frame()?, Some(frame));
        Ok(())
    }

    #[test]
    fn reports_partial_bytes() -> Result<(), FrameError> {
        let mut decoder = FrameDecoder::new();
        assert!(!decoder.has_partial());
        let bytes = data(0, b"{}").encode()?;
        decoder.push(&bytes[..5]);
        assert!(decoder.has_partial());
        decoder.push(&bytes[5..]);
        assert!(decoder.next_frame()?.is_some());
        assert!(!decoder.has_partial());
        Ok(())
    }

    #[test]
    fn with_bytes_starts_from_bytes_already_read() -> Result<(), FrameError> {
        let frame = data(0, b"{}");
        let mut decoder = FrameDecoder::with_bytes(frame.encode()?);
        assert_eq!(decoder.next_frame()?, Some(frame));
        Ok(())
    }

    #[test]
    fn decodes_the_rest_of_a_frame_after_the_buffer_was_compacted() -> Result<(), FrameError> {
        let short = data(1, &[1; 20]);
        let long = data(2, &[2; 100]);
        let long_bytes = long.encode()?;
        let mut bytes = short.encode()?;
        bytes.extend_from_slice(&long_bytes[..10]);
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes);
        assert_eq!(decoder.next_frame()?, Some(short));
        assert_eq!(decoder.next_frame()?, None);
        decoder.push(&long_bytes[10..]);
        assert_eq!(decoder.next_frame()?, Some(long));
        assert!(!decoder.has_partial());
        Ok(())
    }
}
