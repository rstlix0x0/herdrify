//! The transport traits: how a client sends and receives messages without knowing what carries them.
//!
//! This file holds three traits and no logic, so it carries no colocated unit tests (a pure trait
//! file, exempt under the unit-test mandate's third structural exemption). Each implementation is
//! tested where it is defined.
//!
//! A connection moves through two states. Before the handshake it moves lines, through
//! [`Transport`]. [`Transport::into_framed`] consumes the transport and returns a [`FrameSink`] and
//! a [`FrameSource`], which move frames and nothing else, so a line cannot be written once frames
//! have begun.
//!
//! Responsibilities:
//!
//! - [`Transport`], the line-moving state and the step into the frame-moving state
//! - [`FrameSink`] and [`FrameSource`], the write half and the read half of the frame-moving state
//!
//! Non-responsibilities:
//!
//! - carrying bytes over a process or a stream, which an adapter does
//! - deciding what to send or what a reply means, which the client does

use super::TransportError;
use crate::protocol::{Frame, Line};

/// A connection to herdr-core that moves one line at a time, until it is split into frames.
///
/// Lines are the plain-mode messages and the handshake of a framed connection. Calling
/// [`into_framed`](Transport::into_framed) consumes the transport and returns the two frame halves,
/// so after that point only frames can be sent or received.
pub trait Transport: Send + 'static {
    /// The half that writes frames once the transport has been split.
    type Sink: FrameSink;
    /// The half that reads frames once the transport has been split.
    type Source: FrameSource;

    /// Sends one line.
    fn send_line(&mut self, line: Line) -> impl Future<Output = Result<(), TransportError>> + Send;

    /// Receives the next line.
    ///
    /// `Ok(None)` means the peer closed the connection.
    fn recv_line(&mut self) -> impl Future<Output = Result<Option<Line>, TransportError>> + Send;

    /// Consumes the transport and returns its frame halves.
    ///
    /// Bytes already received but not yet returned by [`recv_line`](Transport::recv_line) are
    /// carried into the source and decoded as frames.
    fn into_framed(self) -> (Self::Sink, Self::Source);
}

/// The write half of a framed connection.
pub trait FrameSink: Send + 'static {
    /// Sends one frame.
    fn send(&mut self, frame: Frame) -> impl Future<Output = Result<(), TransportError>> + Send;

    /// Closes the write side so the peer sees the end of the stream.
    fn close(&mut self) -> impl Future<Output = Result<(), TransportError>> + Send;
}

/// The read half of a framed connection.
pub trait FrameSource: Send + 'static {
    /// Receives the next frame.
    ///
    /// `Ok(None)` means the peer closed the connection.
    fn recv(&mut self) -> impl Future<Output = Result<Option<Frame>, TransportError>> + Send;
}
