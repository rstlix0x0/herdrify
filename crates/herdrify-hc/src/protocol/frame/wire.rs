//! Frames: the units a framed herdr-core connection is made of, and their byte encoding.
//!
//! After the hello handshake a connection carries frames instead of lines. A frame names a
//! channel, says what kind of frame it is, and carries a payload; this file builds those values and
//! turns them into bytes, so nothing here reads from or writes to a connection.
//!
//! Responsibilities:
//!
//! - [`Frame`], [`FrameKind`], [`Channel`] and [`Focus`]
//! - the nine-byte header and the 16777216-byte payload limit on encoding
//! - the typed constructors and readers for the credit, resize and focus payloads
//!
//! Non-responsibilities:
//!
//! - cutting frames out of a byte stream, which the frame decoder does
//! - deciding what a payload means on a channel, which the client does

use std::fmt;

use super::FrameError;

/// The most payload bytes one frame may carry: 16777216.
pub const MAX_PAYLOAD_LEN: usize = 16_777_216;

/// The size of the header in front of every payload: channel, kind and length.
pub(super) const HEADER_LEN: usize = 9;

/// A channel number on a framed connection.
///
/// Channel 0 carries JSON-RPC both ways. Every other channel belongs to one stream the core opened.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::Channel;
///
/// assert_eq!(Channel::RPC.get(), 0);
/// assert_eq!(Channel::new(7).get(), 7);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Channel(u32);

impl Channel {
    /// The channel that carries JSON-RPC requests, responses and notifications.
    pub const RPC: Self = Self(0);

    /// Wraps a channel number.
    #[must_use]
    pub const fn new(number: u32) -> Self {
        Self(number)
    }

    /// The channel number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// What a frame is for.
///
/// A byte outside the seven named kinds is [`FrameKind::Unknown`], which keeps the byte so it can
/// be encoded again unchanged; the core drops such frames rather than failing the connection.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::FrameKind;
///
/// assert_eq!(FrameKind::from(1), FrameKind::Credit);
/// assert_eq!(u8::from(FrameKind::Credit), 1);
/// assert_eq!(FrameKind::from(9), FrameKind::Unknown(9));
/// assert_eq!(FrameKind::Credit.to_string(), "CREDIT");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameKind {
    /// Bytes for the channel: JSON-RPC on channel 0, a stream's bytes elsewhere. Wire byte 0.
    Data,
    /// A grant of stream bytes the receiver may send. Wire byte 1.
    Credit,
    /// The end of a channel. Wire byte 2.
    Close,
    /// A terminal size change. Wire byte 3.
    Resize,
    /// A focus change. Wire byte 4.
    Focus,
    /// JSON display facts from the viewer. Wire byte 5.
    Facts,
    /// Reply bytes the terminal generated itself, not human activity. Wire byte 6.
    AutomaticInput,
    /// Any other wire byte.
    Unknown(u8),
}

impl From<u8> for FrameKind {
    fn from(byte: u8) -> Self {
        match byte {
            0 => Self::Data,
            1 => Self::Credit,
            2 => Self::Close,
            3 => Self::Resize,
            4 => Self::Focus,
            5 => Self::Facts,
            6 => Self::AutomaticInput,
            other => Self::Unknown(other),
        }
    }
}

impl From<FrameKind> for u8 {
    fn from(kind: FrameKind) -> Self {
        match kind {
            FrameKind::Data => 0,
            FrameKind::Credit => 1,
            FrameKind::Close => 2,
            FrameKind::Resize => 3,
            FrameKind::Focus => 4,
            FrameKind::Facts => 5,
            FrameKind::AutomaticInput => 6,
            FrameKind::Unknown(other) => other,
        }
    }
}

impl fmt::Display for FrameKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data => formatter.write_str("DATA"),
            Self::Credit => formatter.write_str("CREDIT"),
            Self::Close => formatter.write_str("CLOSE"),
            Self::Resize => formatter.write_str("RESIZE"),
            Self::Focus => formatter.write_str("FOCUS"),
            Self::Facts => formatter.write_str("FACTS"),
            Self::AutomaticInput => formatter.write_str("AUTOMATIC_INPUT"),
            Self::Unknown(byte) => write!(formatter, "UNKNOWN({byte})"),
        }
    }
}

/// Whether a viewer has focus, as a FOCUS frame carries it.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Channel, Focus, Frame};
///
/// # fn main() -> Result<(), herdrify_hc::protocol::FrameError> {
/// let frame = Frame::focus(Channel::new(3), Focus::Gained);
/// assert_eq!(frame.read_focus()?, Focus::Gained);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    /// The viewer gained focus. Wire byte `1`.
    Gained,
    /// The viewer lost focus. Wire byte `0`.
    Lost,
}

/// One frame: a channel, a kind and a payload.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Channel, Frame, FrameKind};
///
/// # fn main() -> Result<(), herdrify_hc::protocol::FrameError> {
/// let frame = Frame {
///     channel: Channel::RPC,
///     kind: FrameKind::Data,
///     payload: b"{}".to_vec(),
/// };
/// assert_eq!(frame.encode()?, [0, 0, 0, 0, 0, 2, 0, 0, 0, b'{', b'}']);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The channel the frame belongs to.
    pub channel: Channel,
    /// What the frame is for.
    pub kind: FrameKind,
    /// The frame's bytes, at most 16777216 of them.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Writes the frame as `[channel u32 LE][kind u8][len u32 LE][payload]`.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::TooLarge`] when the payload is over 16777216 bytes; nothing is
    /// produced in that case.
    pub fn encode(&self) -> Result<Vec<u8>, FrameError> {
        let len = self.payload.len();
        let wire_len = u32::try_from(len)
            .ok()
            .filter(|_| len <= MAX_PAYLOAD_LEN)
            .ok_or(FrameError::TooLarge { len })?;
        let mut bytes = Vec::with_capacity(HEADER_LEN + len);
        bytes.extend_from_slice(&self.channel.get().to_le_bytes());
        bytes.push(u8::from(self.kind));
        bytes.extend_from_slice(&wire_len.to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        Ok(bytes)
    }

    /// A CREDIT frame granting `bytes` more stream bytes.
    #[must_use]
    pub fn credit(channel: Channel, bytes: u32) -> Self {
        Self {
            channel,
            kind: FrameKind::Credit,
            payload: bytes.to_le_bytes().to_vec(),
        }
    }

    /// A RESIZE frame carrying a terminal size.
    #[must_use]
    pub fn resize(channel: Channel, cols: u16, rows: u16) -> Self {
        let mut payload = Vec::with_capacity(4);
        payload.extend_from_slice(&cols.to_le_bytes());
        payload.extend_from_slice(&rows.to_le_bytes());
        Self {
            channel,
            kind: FrameKind::Resize,
            payload,
        }
    }

    /// A FOCUS frame carrying a focus change.
    #[must_use]
    pub fn focus(channel: Channel, focus: Focus) -> Self {
        let byte = match focus {
            Focus::Gained => 1,
            Focus::Lost => 0,
        };
        Self {
            channel,
            kind: FrameKind::Focus,
            payload: vec![byte],
        }
    }

    /// Reads the payload as a CREDIT amount, in bytes.
    ///
    /// The frame's own kind is not checked: the caller has already matched on it.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::BadControlPayload`] when the payload is not 4 bytes.
    pub fn read_credit(&self) -> Result<u32, FrameError> {
        let bytes: [u8; 4] = self.control_payload(FrameKind::Credit)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Reads the payload as a RESIZE size, as columns then rows.
    ///
    /// The frame's own kind is not checked: the caller has already matched on it.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::BadControlPayload`] when the payload is not 4 bytes.
    pub fn read_resize(&self) -> Result<(u16, u16), FrameError> {
        let [c0, c1, r0, r1] = self.control_payload(FrameKind::Resize)?;
        Ok((u16::from_le_bytes([c0, c1]), u16::from_le_bytes([r0, r1])))
    }

    /// Reads the payload as a FOCUS change; any non-zero byte is [`Focus::Gained`].
    ///
    /// The frame's own kind is not checked: the caller has already matched on it.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::BadControlPayload`] when the payload is not 1 byte.
    pub fn read_focus(&self) -> Result<Focus, FrameError> {
        let [byte] = self.control_payload(FrameKind::Focus)?;
        Ok(if byte == 0 {
            Focus::Lost
        } else {
            Focus::Gained
        })
    }

    /// The payload as exactly `N` bytes, or the refusal naming `kind`.
    fn control_payload<const N: usize>(&self, kind: FrameKind) -> Result<[u8; N], FrameError> {
        <[u8; N]>::try_from(self.payload.as_slice()).map_err(|_| FrameError::BadControlPayload {
            kind,
            expected: N,
            got: self.payload.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Channel, Focus, Frame, FrameKind};
    use crate::protocol::FrameError;

    #[test]
    fn encode_writes_the_nine_byte_header() -> Result<(), FrameError> {
        let frame = Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: b"{}".to_vec(),
        };
        assert_eq!(frame.encode()?, [0, 0, 0, 0, 0, 2, 0, 0, 0, 0x7b, 0x7d]);
        Ok(())
    }

    #[test]
    fn kind_bytes_map_both_ways() {
        let kinds = [
            FrameKind::Data,
            FrameKind::Credit,
            FrameKind::Close,
            FrameKind::Resize,
            FrameKind::Focus,
            FrameKind::Facts,
            FrameKind::AutomaticInput,
        ];
        for (byte, kind) in (0u8..).zip(kinds) {
            assert_eq!(FrameKind::from(byte), kind);
            assert_eq!(u8::from(kind), byte);
        }
        assert_eq!(FrameKind::from(9), FrameKind::Unknown(9));
        assert_eq!(u8::from(FrameKind::Unknown(9)), 9);
    }

    #[test]
    fn encode_accepts_16777216_and_refuses_16777217() {
        let at_limit = Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: vec![0; 16_777_216],
        };
        assert_eq!(
            at_limit.encode().map(|bytes| bytes.len()),
            Ok(9 + 16_777_216)
        );

        let over = Frame {
            payload: vec![0; 16_777_217],
            ..at_limit
        };
        assert_eq!(over.encode(), Err(FrameError::TooLarge { len: 16_777_217 }));
    }

    #[test]
    fn control_payloads_round_trip() -> Result<(), FrameError> {
        let channel = Channel::new(7);
        assert_eq!(Frame::credit(channel, 262_144).read_credit()?, 262_144);
        assert_eq!(Frame::resize(channel, 80, 24).read_resize()?, (80, 24));
        assert_eq!(
            Frame::focus(channel, Focus::Gained).read_focus()?,
            Focus::Gained
        );
        assert_eq!(
            Frame::focus(channel, Focus::Lost).read_focus()?,
            Focus::Lost
        );
        assert_eq!(Frame::credit(channel, 1).kind, FrameKind::Credit);
        assert_eq!(Frame::credit(channel, 1).channel, channel);
        Ok(())
    }

    #[test]
    fn control_reader_refuses_wrong_length() {
        let frame = Frame {
            channel: Channel::new(1),
            kind: FrameKind::Credit,
            payload: vec![0; 3],
        };
        let error = frame.read_credit().err();
        assert_eq!(
            error,
            Some(FrameError::BadControlPayload {
                kind: FrameKind::Credit,
                expected: 4,
                got: 3
            })
        );
        assert_eq!(
            error.map(|e| e.to_string()).as_deref(),
            Some("CREDIT frame payload must be 4 bytes, got 3")
        );
    }

    #[test]
    fn frame_error_display_too_large() {
        assert_eq!(
            FrameError::TooLarge { len: 16_777_217 }.to_string(),
            "frame payload of 16777217 bytes exceeds the 16777216-byte limit"
        );
    }
}
