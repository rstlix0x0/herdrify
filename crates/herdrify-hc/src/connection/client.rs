//! `Client`: a framed connection to herdr-core that runs concurrent calls.
//!
//! `Client::connect` speaks the handshake as lines, checks the reply, and then splits the transport
//! into its frame halves and starts the writer and the router. A call allocates an id, registers
//! where its reply goes, queues its frame for the writer and waits for the reply, so any number of
//! calls run side by side on the one connection.
//!
//! Responsibilities:
//!
//! - [`Client`], generic over the [`Transport`] it was connected over and cheap to clone
//! - the handshake and each way it can be refused
//! - [`Client::call_raw`], a call with untyped parameters and an untyped result
//! - the typed methods, one for each request-reply method of herdr-core, which send the method's
//!   parameters and read its reply into the method's output type
//! - [`Client::events_subscribe`], which opens a [`Subscription`] on an event channel
//! - [`Client::terminal_attach`], which opens an [`AttachStream`] on a terminal
//! - [`Client::close_reason`], why the connection ended
//!
//! Non-responsibilities:
//!
//! - carrying bytes, which the transport does
//! - routing replies and writing frames, which the router and the writer do

use std::fmt;
use std::future::{Future, poll_fn};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use serde_json::Value;
use tokio::sync::oneshot;

use super::error::EndRecord;
use super::error::unencodable;
use super::pending::{Answer, Pending};
use super::router;
use super::stream::Opening;
use super::writer::{self, Command, Commands};
use super::{AttachStream, ConnectionEnd, Error, Subscription};
use crate::port::Transport;
use crate::protocol::{
    AttachResult, Channel, CoreVersion, DocGet, DocGetResult, DocSet, DocSetResult, ErrorKind,
    EventsSubscribe, ExecResult, FileEdit, FileRead, FileReadResult, FileResult, FileWrite, Frame,
    FrameKind, Generation, HelloParams, HelloResult, Incoming, InterruptResult, KeysResult, Line,
    ListResult, MAX_PAYLOAD_LEN, Machine, MachineAdd, MachineConnect, MachineExec, MachineList,
    MachineListResult, MachineRename, MachineRm, MachineView, Method, MethodName, ProtocolError,
    PutResult, ReadResult, RemoveResult, Request, RequestId, RpcDiscover, RunResult, SendResult,
    ServerRestart, ServerStatus, ServerStop, StatusResult, StopResult, SubscribeResult,
    TemplatesList, TemplatesResult, Terminal, TerminalAttach, TerminalGet, TerminalInterrupt,
    TerminalKeep, TerminalKeys, TerminalKill, TerminalLabel, TerminalList, TerminalPut,
    TerminalRead, TerminalRm, TerminalRun, TerminalSend, TerminalWait, UnexpectedReply, WaitResult,
};

/// A framed connection to herdr-core.
///
/// A `Client` is a handle: cloning it is cheap, and every clone talks over the same connection.
/// The connection stays open while any clone exists, and while a [`Subscription`] or an
/// [`AttachStream`] opened through it does. When the last one drops, the transport's write side is
/// closed. The connection itself ends when the core ends its stream.
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::{Client, Error, Transport};
///
/// async fn open<T: Transport>(transport: T) -> Result<(), Error> {
///     let client = Client::connect(transport).await?;
///     println!("connected to herdr-core {}", client.hello().version);
///     Ok(())
/// }
/// ```
pub struct Client<T: Transport> {
    inner: Arc<Inner>,
    transport: PhantomData<fn() -> T>,
}

struct Inner {
    hello: HelloResult,
    commands: Commands,
    pending: Arc<Pending>,
    end: Arc<EndRecord>,
}

impl<T: Transport> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            transport: PhantomData,
        }
    }
}

impl<T: Transport> fmt::Debug for Client<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl<T: Transport> Client<T> {
    /// Opens a framed connection over `transport`.
    ///
    /// The transport is dropped if the handshake fails.
    ///
    /// # Panics
    ///
    /// Panics when called outside a Tokio runtime, because it spawns the tasks that read and write
    /// frames.
    ///
    /// # Errors
    ///
    /// - [`Error::Transport`] when a line cannot be sent or received.
    /// - [`Error::Closed`] when the core closes the connection before it replies.
    /// - [`Error::Protocol`] when the reply is not JSON, is not JSON-RPC, is not the reply to the
    ///   handshake, or does not confirm a framed connection at generation 1.
    /// - [`Error::UnsupportedGeneration`] when the core refuses generation 1.
    /// - [`Error::Rpc`] when the core refuses the handshake for another reason.
    /// - [`Error::Decode`] when the reply's result is not a handshake result.
    /// - [`Error::InvalidVersion`] when the core's version is not `MAJOR.MINOR.PATCH`.
    /// - [`Error::UnsupportedVersion`] when the core is older than 0.1.21.
    pub async fn connect(mut transport: T) -> Result<Self, Error> {
        let hello = MethodName::new(HelloParams::NAME).map_err(unencodable)?;
        let params = HelloParams::new().generation(Generation::ONE).framed();
        let request = Request::new(
            RequestId(0),
            HelloParams::NAME,
            serde_json::to_value(params).map_err(unencodable)?,
        );
        let text = serde_json::to_string(&request).map_err(unencodable)?;
        transport
            .send_line(Line::new(text).map_err(unencodable)?)
            .await?;

        let line = transport.recv_line().await?.ok_or(Error::Closed)?;
        let result = match Incoming::classify(line.as_str().as_bytes())? {
            Incoming::Notification { .. } => return Err(unexpected(UnexpectedReply::Notification)),
            Incoming::Response { id: None, .. } => {
                return Err(unexpected(UnexpectedReply::NullIdResponse));
            }
            Incoming::Response { id: Some(id), .. } if id != RequestId(0) => {
                return Err(unexpected(UnexpectedReply::Response { id }));
            }
            Incoming::Response {
                outcome: Err(error),
                ..
            } => {
                return Err(if error.kind == ErrorKind::UnsupportedGeneration {
                    Error::UnsupportedGeneration {
                        message: error.message,
                    }
                } else {
                    Error::Rpc {
                        method: hello,
                        error,
                    }
                });
            }
            Incoming::Response {
                outcome: Ok(result),
                ..
            } => result,
        };
        let hello_result: HelloResult =
            serde_json::from_value(result).map_err(|source| Error::Decode {
                method: hello,
                source,
            })?;
        if !hello_result.framed || hello_result.generation != Generation::ONE {
            return Err(Error::Protocol(ProtocolError::HelloMismatch {
                framed: hello_result.framed,
                generation: hello_result.generation.get(),
            }));
        }
        let found = CoreVersion::parse(&hello_result.version)
            .map_err(|error| Error::InvalidVersion { found: error.found })?;
        if found < CoreVersion::MINIMUM {
            return Err(Error::UnsupportedVersion {
                found,
                minimum: CoreVersion::MINIMUM,
            });
        }

        let (sink, source) = transport.into_framed();
        let pending = Arc::new(Pending::new());
        let end = Arc::new(EndRecord::new());
        let router = router::spawn(source, Arc::clone(&pending), Arc::clone(&end));
        let commands = writer::spawn(sink, Arc::clone(&pending), Arc::clone(&end), router);
        Ok(Self {
            inner: Arc::new(Inner {
                hello: hello_result,
                commands,
                pending,
                end,
            }),
            transport: PhantomData,
        })
    }

    /// The core's reply to the handshake.
    #[must_use]
    pub fn hello(&self) -> &HelloResult {
        &self.inner.hello
    }

    /// Why the connection ended, or `None` while it is open.
    ///
    /// The first reason is kept. A connection that has ended stays ended: every later call returns
    /// [`Error::Closed`] without sending anything.
    #[must_use]
    pub fn close_reason(&self) -> Option<ConnectionEnd> {
        self.inner.end.get()
    }

    /// Calls `method` with `params` and returns the reply's `result` as it came.
    ///
    /// Calls run concurrently: any number can be waiting at once, and each gets its own reply.
    /// Dropping the returned future abandons the call, and a reply that arrives later is
    /// discarded. There is no timeout.
    ///
    /// # Streams
    ///
    /// A raw call opens no stream. With `events.subscribe` or `terminal.attach` the reply is
    /// returned, naming a channel, but the channel's frames are discarded:
    ///
    /// - The core stops sending at its credit window, and a stalled stream never blocks the other
    ///   channels.
    /// - The caller cannot close the channel, which stays open for the life of the connection.
    /// - A raw `terminal.attach` makes this connection an active viewer, which sets the terminal's
    ///   size.
    ///
    /// Use [`Client::events_subscribe`] and [`Client::terminal_attach`] for streams.
    ///
    /// # Errors
    ///
    /// - [`Error::Rpc`] when the core refuses the call.
    /// - [`Error::Closed`] when the connection has ended, before or while the call waits. This is
    ///   checked before the size of the request.
    /// - [`Error::PayloadTooLarge`] when the request is longer than a frame may carry. Nothing is
    ///   sent, and the connection is unaffected.
    /// - [`Error::Transport`] when writing this request fails, which also ends the connection.
    pub async fn call_raw(&self, method: MethodName, params: Value) -> Result<Value, Error> {
        let Answer { outcome, .. } = self.exchange(&method, params, None).await?;
        outcome.map_err(|error| Error::Rpc { method, error })
    }

    /// Sends a request and waits for its answer.
    ///
    /// With `opening` set, the call registers to open a stream: when its successful reply names a
    /// channel the router can open, the answer carries the stream's handle. The call's entry then
    /// outlives the future once the request has been handed to the writer, so a reply that arrives
    /// after the future is dropped still opens the route, and the handle is dropped with the
    /// answer.
    async fn exchange(
        &self,
        method: &MethodName,
        params: Value,
        opening: Option<Opening>,
    ) -> Result<Answer, Error> {
        let opens = opening.is_some();
        let (id, reply) = opening.map_or_else(
            || self.inner.pending.register(),
            |opening| self.inner.pending.register_opening(opening),
        );
        let mut entry = Entry {
            pending: &self.inner.pending,
            id,
            keep: false,
        };
        let request = Request::new(id, method.as_str(), params);
        let payload = serde_json::to_vec(&request).map_err(unencodable)?;
        if self.inner.end.is_ended() {
            return Err(Error::Closed);
        }
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(Error::PayloadTooLarge { len: payload.len() });
        }
        let (ack, written) = oneshot::channel();
        self.inner
            .commands
            .send(Command {
                frame: Frame {
                    channel: Channel::RPC,
                    kind: FrameKind::Data,
                    payload,
                },
                ack: Some(ack),
            })
            .map_err(|_| Error::Closed)?;
        entry.keep = opens;
        wait(written, reply).await
    }

    /// What a call that opens a stream hands the router.
    fn opening(&self) -> Opening {
        Opening {
            commands: self.inner.commands.clone(),
            end: Arc::clone(&self.inner.end),
        }
    }
}

impl<T: Transport> Client<T> {
    /// Calls the method `M` with `params` and reads the reply into `M::Output`.
    async fn call<M: Method>(&self, params: M::Params) -> Result<M::Output, Error> {
        let method = MethodName::new(M::NAME).map_err(unencodable)?;
        let params = serde_json::to_value(params).map_err(unencodable)?;
        let result = self.call_raw(method.clone(), params).await?;
        serde_json::from_value(result).map_err(|source| Error::Decode { method, source })
    }

    /// Asks the core how it is doing: its identity, version, uptime and terminal counts.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`StatusResult`].
    pub async fn server_status(&self, params: ServerStatus) -> Result<StatusResult, Error> {
        self.call::<ServerStatus>(params).await
    }

    /// Stops the core.
    ///
    /// The core replies and then ends the connection, so [`Client::close_reason`] soon reports
    /// that the peer closed it.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`StopResult`].
    pub async fn server_stop(&self, params: ServerStop) -> Result<StopResult, Error> {
        self.call::<ServerStop>(params).await
    }

    /// Restarts the core, optionally with a replacement environment.
    ///
    /// A planned restart: screens are saved, programs keep running and the next core adopts them.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`StopResult`].
    pub async fn server_restart(&self, params: ServerRestart) -> Result<StopResult, Error> {
        self.call::<ServerRestart>(params).await
    }

    /// Lists the terminal templates the core knows.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a
    /// [`TemplatesResult`].
    pub async fn templates_list(&self, params: TemplatesList) -> Result<TemplatesResult, Error> {
        self.call::<TemplatesList>(params).await
    }

    /// Asks the core for its own description of its methods, an `OpenRPC` document, as JSON.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`].
    pub async fn rpc_discover(&self, params: RpcDiscover) -> Result<Value, Error> {
        self.call::<RpcDiscover>(params).await
    }

    /// Starts a terminal running a command, and optionally waits for it.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`RunResult`].
    pub async fn terminal_run(&self, params: TerminalRun) -> Result<RunResult, Error> {
        self.call::<TerminalRun>(params).await
    }

    /// Lists the terminals, and the machines they run on.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`ListResult`].
    pub async fn terminal_list(&self, params: TerminalList) -> Result<ListResult, Error> {
        self.call::<TerminalList>(params).await
    }

    /// Reads the record of one terminal.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`Terminal`].
    pub async fn terminal_get(&self, params: TerminalGet) -> Result<Terminal, Error> {
        self.call::<TerminalGet>(params).await
    }

    /// Kills a terminal and returns its record as it ends.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`Terminal`].
    pub async fn terminal_kill(&self, params: TerminalKill) -> Result<Terminal, Error> {
        self.call::<TerminalKill>(params).await
    }

    /// Removes a finished terminal.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`RemoveResult`].
    pub async fn terminal_rm(&self, params: TerminalRm) -> Result<RemoveResult, Error> {
        self.call::<TerminalRm>(params).await
    }

    /// Sets whether a terminal is kept after its program ends.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`Terminal`].
    pub async fn terminal_keep(&self, params: TerminalKeep) -> Result<Terminal, Error> {
        self.call::<TerminalKeep>(params).await
    }

    /// Sets and removes labels on a terminal.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`Terminal`].
    pub async fn terminal_label(&self, params: TerminalLabel) -> Result<Terminal, Error> {
        self.call::<TerminalLabel>(params).await
    }

    /// Reads the text on a terminal's screen.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`ReadResult`].
    pub async fn terminal_read(&self, params: TerminalRead) -> Result<ReadResult, Error> {
        self.call::<TerminalRead>(params).await
    }

    /// Types text into a terminal, and optionally waits for the program's answer.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`SendResult`].
    pub async fn terminal_send(&self, params: TerminalSend) -> Result<SendResult, Error> {
        self.call::<TerminalSend>(params).await
    }

    /// Waits for a terminal, or its children, to end or to report.
    ///
    /// Unless the request sets a timeout, the core waits for its configured `wait_timeout`, else
    /// 180 seconds, and this crate adds none of its own.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`WaitResult`].
    pub async fn terminal_wait(&self, params: TerminalWait) -> Result<WaitResult, Error> {
        self.call::<TerminalWait>(params).await
    }

    /// Presses named keys on a terminal.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`KeysResult`].
    pub async fn terminal_keys(&self, params: TerminalKeys) -> Result<KeysResult, Error> {
        self.call::<TerminalKeys>(params).await
    }

    /// Interrupts the program in a terminal.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not an [`InterruptResult`].
    pub async fn terminal_interrupt(
        &self,
        params: TerminalInterrupt,
    ) -> Result<InterruptResult, Error> {
        self.call::<TerminalInterrupt>(params).await
    }

    /// Stores bytes as a file beside a terminal, on its machine, for its program to read.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`PutResult`].
    pub async fn terminal_put(&self, params: TerminalPut) -> Result<PutResult, Error> {
        self.call::<TerminalPut>(params).await
    }

    /// Runs a command on a machine and returns what it printed.
    ///
    /// The output is returned as bytes, decoded from the base64 the core sends.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not an [`ExecResult`].
    pub async fn machine_exec(&self, params: MachineExec) -> Result<ExecResult, Error> {
        self.call::<MachineExec>(params).await
    }

    /// Reads lines of a text file on a machine.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`FileReadResult`].
    pub async fn file_read(&self, params: FileRead) -> Result<FileReadResult, Error> {
        self.call::<FileRead>(params).await
    }

    /// Writes bytes to a file on a machine.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`FileResult`].
    pub async fn file_write(&self, params: FileWrite) -> Result<FileResult, Error> {
        self.call::<FileWrite>(params).await
    }

    /// Replaces exact text in a file on a machine.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`FileResult`].
    pub async fn file_edit(&self, params: FileEdit) -> Result<FileResult, Error> {
        self.call::<FileEdit>(params).await
    }

    /// Reads a shared document and its version.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`DocGetResult`].
    pub async fn doc_get(&self, params: DocGet) -> Result<DocGetResult, Error> {
        self.call::<DocGet>(params).await
    }

    /// Replaces a shared document, if it is still at the version the request names.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`DocSetResult`].
    pub async fn doc_set(&self, params: DocSet) -> Result<DocSetResult, Error> {
        self.call::<DocSet>(params).await
    }

    /// Subscribes to herdr-core's events, optionally narrowed to some terminals.
    ///
    /// The returned [`Subscription`] delivers the notifications that follow. The connection stays
    /// open while it exists.
    ///
    /// # Errors
    ///
    /// - As [`Client::call_raw`].
    /// - [`Error::Decode`] when the reply is not a [`SubscribeResult`]. A `channel` that is not a
    ///   `u32`, is 0, or is already open for another stream opens nothing and sends nothing. For a
    ///   reply that names a channel the client could open, the channel is closed again.
    /// - [`Error::Protocol`] with [`ProtocolError::MissingChannel`] when the reply names no
    ///   channel to read events from.
    pub async fn events_subscribe(&self, params: EventsSubscribe) -> Result<Subscription, Error> {
        let method = MethodName::new(EventsSubscribe::NAME).map_err(unencodable)?;
        let params = serde_json::to_value(params).map_err(unencodable)?;
        let Answer { outcome, stream } =
            self.exchange(&method, params, Some(self.opening())).await?;
        let result = outcome.map_err(|error| Error::Rpc {
            method: method.clone(),
            error,
        })?;
        let result: SubscribeResult =
            serde_json::from_value(result).map_err(|source| Error::Decode {
                method: method.clone(),
                source,
            })?;
        match (result.channel, stream) {
            (None, _) => Err(Error::Protocol(ProtocolError::MissingChannel { method })),
            (Some(_), Some(handle)) => Ok(Subscription::new(handle, result)),
            // The router opens a route for every successful reply that names a `u32` channel it
            // can use, so a reply with a channel and no handle named channel 0 or one in use.
            (Some(channel), None) => Err(Error::Decode {
                method,
                source: unusable_channel(channel),
            }),
        }
    }

    /// Attaches to a terminal as a viewer and opens its byte stream.
    ///
    /// The returned [`AttachStream`] delivers the terminal's output and takes the viewer's input.
    /// The connection stays open while it exists. Attaching sets the terminal's size to the one the
    /// request names, unless the request observes only. A request that is resumed sets it once the
    /// viewer is active.
    ///
    /// # Errors
    ///
    /// - As [`Client::call_raw`].
    /// - [`Error::Decode`] when the reply is not an [`AttachResult`], which includes a reply that is
    ///   not a JSON object. A `channel` that is not a `u32`, is 0, or is already open for another
    ///   stream opens nothing and sends nothing. For a reply that names a channel the client could
    ///   open, the channel is closed again.
    /// - [`Error::Protocol`] with [`ProtocolError::MissingChannel`] when the reply is an object that
    ///   names no channel, or a null one, to read the terminal from.
    pub async fn terminal_attach(&self, params: TerminalAttach) -> Result<AttachStream, Error> {
        let method = MethodName::new(TerminalAttach::NAME).map_err(unencodable)?;
        let params = serde_json::to_value(params).map_err(unencodable)?;
        let Answer { outcome, stream } =
            self.exchange(&method, params, Some(self.opening())).await?;
        let result = outcome.map_err(|error| Error::Rpc {
            method: method.clone(),
            error,
        })?;
        // The reply's channel is required, so an object without one would not decode. It is reported
        // as the protocol error the other stream-opening method reports. A reply that is not an
        // object falls through to the decode below.
        if result.is_object() && result.get("channel").is_none_or(Value::is_null) {
            return Err(Error::Protocol(ProtocolError::MissingChannel { method }));
        }
        let result: AttachResult =
            serde_json::from_value(result).map_err(|source| Error::Decode {
                method: method.clone(),
                source,
            })?;
        match stream {
            Some(handle) => {
                let accepts = AttachStream::lists_automatic_input(&self.inner.hello.features);
                Ok(AttachStream::new(handle, result, accepts))
            }
            // The router opens a route for every successful reply that names a `u32` channel it
            // can use, so a reply with a channel and no handle named channel 0 or one in use.
            None => Err(Error::Decode {
                method,
                source: unusable_channel(result.channel),
            }),
        }
    }

    /// Adds a machine the core can run terminals on.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`Machine`].
    pub async fn machine_add(&self, params: MachineAdd) -> Result<Machine, Error> {
        self.call::<MachineAdd>(params).await
    }

    /// Lists the machines the core knows.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`MachineListResult`].
    pub async fn machine_list(&self, params: MachineList) -> Result<MachineListResult, Error> {
        self.call::<MachineList>(params).await
    }

    /// Connects to a machine the core knows.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`MachineView`].
    pub async fn machine_connect(&self, params: MachineConnect) -> Result<MachineView, Error> {
        self.call::<MachineConnect>(params).await
    }

    /// Renames a machine.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`Machine`].
    pub async fn machine_rename(&self, params: MachineRename) -> Result<Machine, Error> {
        self.call::<MachineRename>(params).await
    }

    /// Removes a machine.
    ///
    /// # Errors
    ///
    /// As [`Client::call_raw`], and [`Error::Decode`] when the reply is not a [`RemoveResult`].
    pub async fn machine_rm(&self, params: MachineRm) -> Result<RemoveResult, Error> {
        self.call::<MachineRm>(params).await
    }
}

/// Waits for a call's reply and for the writer's report on its frame, whichever speaks first.
///
/// A failed write is returned as it came. A reply that never arrives because the connection ended
/// is `Closed`, even while the frame is still being written: a write that stalls must not hold the
/// call after the connection is over.
async fn wait(
    mut written: oneshot::Receiver<Result<(), Error>>,
    mut reply: oneshot::Receiver<Answer>,
) -> Result<Answer, Error> {
    let mut acknowledged = false;
    poll_fn(|cx| {
        if !acknowledged {
            match Pin::new(&mut written).poll(cx) {
                Poll::Ready(Ok(Ok(()))) => acknowledged = true,
                Poll::Ready(Ok(Err(error))) => return Poll::Ready(Err(error)),
                Poll::Ready(Err(_)) => return Poll::Ready(Err(Error::Closed)),
                Poll::Pending => {}
            }
        }
        match Pin::new(&mut reply).poll(cx) {
            Poll::Ready(Ok(answer)) => Poll::Ready(Ok(answer)),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::Closed)),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

/// Removes a call's pending entry when the call ends or is dropped, unless it is kept.
///
/// A call that opens a stream keeps its entry once its request is with the writer, so the reply
/// still finds its route request.
struct Entry<'a> {
    pending: &'a Pending,
    id: RequestId,
    keep: bool,
}

impl Drop for Entry<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.pending.remove(self.id);
        }
    }
}

const fn unexpected(got: UnexpectedReply) -> Error {
    Error::Protocol(ProtocolError::UnexpectedHelloReply { got })
}

/// Why the router refused to open a stream on `channel`, as the source of an [`Error::Decode`].
///
/// The router refuses channel 0, which carries calls, and a channel another stream holds.
fn unusable_channel(channel: u32) -> serde_json::Error {
    let reason = if channel == Channel::RPC.get() {
        "channel 0 carries calls, not streams".to_owned()
    } else {
        format!("channel {channel} is already open for another stream")
    };
    serde::de::Error::custom(reason)
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::time::Duration;

    use serde_json::{Value, json};

    use super::Client;
    use crate::connection::memory_link::{MemoryPeer, MemoryTransport, link};
    use crate::connection::{ConnectionEndKind, Error};
    use crate::protocol::{Channel, Frame, FrameKind, MethodName, ServerStop};

    type TestResult = Result<(), Box<dyn StdError>>;

    const HELLO_REPLY: &str = r#"{"id":0,"jsonrpc":"2.0","result":{"core":{"id":"c","name":"n"},"features":[],"framed":true,"generation":1,"methods":[],"version":"0.1.21"}}"#;

    fn response(id: u64, result: &Value) -> Frame {
        Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: json!({"id": id, "jsonrpc": "2.0", "result": result})
                .to_string()
                .into_bytes(),
        }
    }

    #[tokio::test]
    async fn connect_over_memory_link_sends_hello_id_zero() -> TestResult {
        let (transport, mut peer) = link();
        peer.reply_line(HELLO_REPLY);
        let client = Client::connect(transport).await?;
        let line = peer.line().await?;
        assert_eq!(
            serde_json::from_str::<Value>(line.as_str())?,
            json!({"id": 0, "jsonrpc": "2.0", "method": "hello",
                   "params": {"framed": true, "generation": 1}})
        );
        assert_eq!(client.hello().version, "0.1.21");

        let call = client.call_raw(MethodName::new("x")?, json!({}));
        peer.push(response(1, &json!(true)));
        assert_eq!(call.await?, json!(true));
        Ok(())
    }

    #[tokio::test]
    async fn call_raw_writes_channel_zero_data() -> TestResult {
        let (transport, mut peer) = link();
        peer.reply_line(HELLO_REPLY);
        let client = Client::connect(transport).await?;
        let _hello = peer.line().await?;

        let call = client.call_raw(MethodName::new("terminal.get")?, json!({"terminal": "t1"}));
        peer.push(response(1, &json!(null)));
        call.await?;
        let frame = peer.frame().await?;
        assert_eq!(frame.channel, Channel::RPC);
        assert_eq!(frame.kind, FrameKind::Data);
        assert_eq!(
            serde_json::from_slice::<Value>(&frame.payload)?,
            json!({"id": 1, "jsonrpc": "2.0", "method": "terminal.get",
                   "params": {"terminal": "t1"}})
        );
        Ok(())
    }

    fn oversized() -> Value {
        json!({"blob": "x".repeat(16_777_216)})
    }

    #[tokio::test]
    async fn closed_is_checked_before_payload_size() -> TestResult {
        let (transport, mut peer) = link();
        peer.reply_line(HELLO_REPLY);
        let client = Client::connect(transport).await?;
        let _hello = peer.line().await?;
        assert_eq!(client.close_reason(), None);

        let refused = client.call_raw(MethodName::new("x")?, oversized()).await;
        assert!(matches!(refused, Err(Error::PayloadTooLarge { len }) if len > 16_777_216));
        assert!(!peer.has_unread());

        peer.end();
        tokio::time::timeout(Duration::from_secs(5), async {
            while client.close_reason().is_none() {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let refused = client.call_raw(MethodName::new("x")?, oversized()).await;
        assert!(matches!(refused, Err(Error::Closed)));
        assert!(!peer.has_unread());
        assert_eq!(
            client.close_reason().map(|end| end.kind),
            Some(ConnectionEndKind::PeerClosed)
        );
        Ok(())
    }

    async fn connected() -> Result<(Client<MemoryTransport>, MemoryPeer), Box<dyn StdError>> {
        let (transport, mut peer) = link();
        peer.reply_line(HELLO_REPLY);
        let client = Client::connect(transport).await?;
        let _hello = peer.line().await?;
        Ok((client, peer))
    }

    #[test]
    fn client_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<Client<MemoryTransport>>();
    }

    #[tokio::test]
    async fn refused_calls_leave_no_pending_entry() -> TestResult {
        let (client, mut peer) = connected().await?;
        let refused = client.call_raw(MethodName::new("x")?, oversized()).await;
        assert!(matches!(refused, Err(Error::PayloadTooLarge { .. })));
        assert!(client.inner.pending.is_empty());

        peer.end();
        tokio::time::timeout(Duration::from_secs(5), async {
            while client.close_reason().is_none() {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let refused = client.call_raw(MethodName::new("x")?, json!({})).await;
        assert!(matches!(refused, Err(Error::Closed)));
        assert!(client.inner.pending.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn dropped_call_leaves_no_pending_entry() -> TestResult {
        let (client, _peer) = connected().await?;
        let abandoned = tokio::time::timeout(
            Duration::from_millis(50),
            client.call_raw(MethodName::new("slow")?, json!({})),
        )
        .await;
        assert!(abandoned.is_err(), "the core never replies");
        assert!(client.inner.pending.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn call_decodes_a_good_reply_into_the_method_output() -> TestResult {
        let (client, mut peer) = connected().await?;
        let call = client.call::<ServerStop>(ServerStop::new());
        peer.push(response(1, &json!({"stopped": true})));
        assert!(call.await?.stopped);
        let frame = peer.frame().await?;
        assert_eq!(
            serde_json::from_slice::<Value>(&frame.payload)?,
            json!({"id": 1, "jsonrpc": "2.0", "method": "server.stop", "params": {}})
        );
        Ok(())
    }

    #[tokio::test]
    async fn call_names_the_method_when_the_result_does_not_decode() -> TestResult {
        let (client, peer) = connected().await?;
        let call = client.call::<ServerStop>(ServerStop::new());
        peer.push(response(1, &json!({"stopped": "yes"})));
        match call.await {
            Err(Error::Decode { method, .. }) => assert_eq!(method.as_str(), "server.stop"),
            other => return Err(format!("expected Error::Decode, got {other:?}").into()),
        }
        Ok(())
    }
}
