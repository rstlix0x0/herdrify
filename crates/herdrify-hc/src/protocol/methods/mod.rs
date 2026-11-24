//! The `Method` trait and the request types of herdr-core's methods other than `hello`, grouped by
//! what they act on, with the small types their parameters share. The `hello` parameters live with
//! the handshake.
//!
//! Each request type holds exactly the parameters the schema lists for its method, sends only the
//! ones a caller set, and implements `Method` with the wire name and the reply type. Required
//! parameters are constructor arguments and optional ones are setters. This file only lists them.

mod argv;
mod doc;
mod events;
mod file;
mod machine;
mod method;
mod rpc;
mod seconds;
mod server;
mod templates;
mod terminal;

#[doc(inline)]
pub use argv::Argv;
#[doc(inline)]
pub use doc::{DocGet, DocSet};
#[doc(inline)]
pub use events::EventsSubscribe;
#[doc(inline)]
pub use file::{FileEdit, FileRead, FileWrite};
#[doc(inline)]
pub use machine::{
    ExecCommand, MachineAdd, MachineConnect, MachineExec, MachineList, MachineRename, MachineRm,
    MachineTarget,
};
#[doc(inline)]
pub use method::Method;
#[doc(inline)]
pub use rpc::RpcDiscover;
#[doc(inline)]
pub use server::{ServerRestart, ServerStatus, ServerStop};
#[doc(inline)]
pub use templates::TemplatesList;
#[doc(inline)]
pub use terminal::{
    Command, Keep, LabelChange, LabelChanges, TerminalAttach, TerminalGet, TerminalInterrupt,
    TerminalKeep, TerminalKeys, TerminalKill, TerminalLabel, TerminalList, TerminalPut,
    TerminalPutAbort, TerminalPutBegin, TerminalPutChunk, TerminalPutFinish, TerminalRead,
    TerminalRm, TerminalRun, TerminalSend, TerminalWait, WaitTarget,
};
