//! The component schemas that describe terminals, in five roles.
//!
//! `record` holds the terminal and the facts attached to it, `state` the words that say what a
//! terminal is doing, `reasons` the open sets that say why a terminal ended or a wait returned,
//! `put` the replies to the put methods, and `results` the replies to the other `terminal.*`
//! methods. This file only lists them.

mod put;
mod reasons;
mod record;
mod results;
mod state;

#[doc(inline)]
pub use put::{EmptyResult, PutBeginResult, PutPieceResult, PutResult};
#[doc(inline)]
pub use reasons::{EndReason, WaitReason};
#[doc(inline)]
pub use record::{DisplayFacts, FirstMessage, Foreground, InitialSize, Status, Terminal};
#[doc(inline)]
pub use results::{
    AttachResult, InterruptResult, KeysResult, ListResult, ReadResult, RemoveResult, RunResult,
    SendResult, WaitResult,
};
#[doc(inline)]
pub use state::{Children, IdleOrWorking, StatusWord, TerminalState};
