//! The component schemas that describe terminals, in four roles.
//!
//! `record` holds the terminal and the facts attached to it, `state` the words that say what a
//! terminal is doing, `reasons` the open sets that say why a terminal ended or a wait returned,
//! and `results` the replies to the `terminal.*` methods. This file only lists them.

mod reasons;
mod record;
mod results;
mod state;

#[doc(inline)]
pub use reasons::{EndReason, WaitReason};
#[doc(inline)]
pub use record::{DisplayFacts, FirstMessage, Foreground, InitialSize, Status, Terminal};
#[doc(inline)]
pub use results::{
    AttachResult, InterruptResult, KeysResult, ListResult, PutResult, ReadResult, RemoveResult,
    RunResult, SendResult, WaitResult,
};
#[doc(inline)]
pub use state::{Children, IdleOrWorking, StatusWord, TerminalState};
