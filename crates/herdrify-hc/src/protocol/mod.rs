//! The herdr-core protocol as pure data and rules, with no I/O.
//!
//! Nothing here touches an async runtime, a socket or a process, so every type can be built,
//! encoded and tested without one. The transport sits above this layer and depends on it.
//!
//! Responsibilities:
//!
//! - the identifier newtypes that name terminals, machines, documents and keys, and the version
//!   number a document carries
//! - the error a refusing newtype returns for an empty value
//! - the wire codec: frames and their decoder, lines and their decoder, and the NDJSON decoder
//!   for stream channels, with the errors they return
//! - the component schemas of the core as types a reply is read into: records, results, the
//!   closed and open word enums, and the label and environment maps
//! - the `Method` trait and the request types of the non-handshake methods, each holding exactly
//!   the parameters the schema lists for its method
//! - the JSON-RPC envelope: the request a client sends and the response or notification a peer
//!   sends back, with the refusals (`RpcError`) a response can carry
//! - the `hello` handshake, and the core version it reports
//! - notifications and the typed events among them
//! - `ProtocolError`, the error for a peer that breaks the protocol
//! - the serde support that carries bytes as base64 text, and the equality and hashing of the open
//!   word enums by the word each is sent as; both are private to the crate
//!
//! Non-responsibilities:
//!
//! - moving bytes or messages between a connection and these types, which the transport does
//! - deciding which values the core accepts beyond the empty string, which the core decides
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests.

mod base64;
mod error;
mod frame;
mod hello;
mod identifiers;
mod jsonrpc;
mod line;
mod methods;
mod ndjson;
mod notification;
mod types;
mod word;

#[doc(inline)]
pub use error::{ProtocolError, UnexpectedReply};
#[doc(inline)]
pub use frame::{Channel, Focus, Frame, FrameDecoder, FrameError, FrameKind, MAX_PAYLOAD_LEN};
#[doc(inline)]
pub use hello::{CoreVersion, Generation, HelloParams, HelloResult, InvalidCoreVersion};
#[doc(inline)]
pub use identifiers::{
    CallerId, DocName, DocVersion, EmptyValueError, FileName, KeyName, LabelKey, LabelValue,
    MachineName, MethodName, RemotePath, SessionName, TerminalAddress, TerminalName, TerminalRef,
};
#[doc(inline)]
pub use jsonrpc::{ErrorKind, Incoming, Request, RequestId, RpcError};
#[doc(inline)]
pub use line::{Line, LineDecoder, LineError};
#[doc(inline)]
pub use methods::{
    Argv, Command, DocGet, DocSet, EventsSubscribe, ExecCommand, FileEdit, FileRead, FileWrite,
    Keep, LabelChange, LabelChanges, MachineAdd, MachineConnect, MachineExec, MachineList,
    MachineRename, MachineRm, MachineTarget, Method, RpcDiscover, ServerRestart, ServerStatus,
    ServerStop, TemplatesList, TerminalAttach, TerminalGet, TerminalInterrupt, TerminalKeep,
    TerminalKeys, TerminalKill, TerminalLabel, TerminalList, TerminalPut, TerminalRead, TerminalRm,
    TerminalRun, TerminalSend, TerminalWait, WaitTarget,
};
#[doc(inline)]
pub use ndjson::NdjsonDecoder;
#[doc(inline)]
pub use notification::{Event, EventKind, Notification, Seq};
#[doc(inline)]
pub use types::{
    AttachResult, Children, CoreInfo, DisplayFacts, DocGetResult, DocSetResult, Edit, EndReason,
    Environment, ExecResult, FileReadResult, FileResult, FirstMessage, Foreground, HolderInfo,
    IdleOrWorking, InitialSize, InterruptResult, KeysResult, Labels, ListResult, LoginSession,
    Machine, MachineListResult, MachineView, PutResult, ReadResult, RemoveResult, RunResult,
    SendResult, Status, StatusResult, StatusWord, StopResult, SubscribeResult, TemplateInfo,
    TemplatesResult, Terminal, TerminalState, WaitReason, WaitResult,
};
