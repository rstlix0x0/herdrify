//! The component schemas of herdr-core as Rust types, grouped by what they describe.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties. The
//! terminal types live in `terminal`, and the rest sit one file per domain, mirroring `methods`.
//! This file only lists them.

mod doc;
mod events;
mod file;
mod machine;
mod maps;
mod server;
mod templates;
mod terminal;

#[doc(inline)]
pub use doc::{DocGetResult, DocSetResult};
#[doc(inline)]
pub use events::SubscribeResult;
#[doc(inline)]
pub use file::{Edit, FileReadResult, FileResult};
#[doc(inline)]
pub use machine::{ExecResult, Machine, MachineListResult, MachineView};
#[doc(inline)]
pub use maps::{Environment, Labels};
#[doc(inline)]
pub use server::{CoreInfo, HolderInfo, LoginSession, StatusResult, StopResult};
#[doc(inline)]
pub use templates::{TemplateInfo, TemplatesResult};
#[doc(inline)]
pub use terminal::{
    AttachResult, Children, DisplayFacts, EndReason, FirstMessage, Foreground, IdleOrWorking,
    InitialSize, InterruptResult, KeysResult, ListResult, PutResult, ReadResult, RemoveResult,
    RunResult, SendResult, Status, StatusWord, Terminal, TerminalState, WaitReason, WaitResult,
};
