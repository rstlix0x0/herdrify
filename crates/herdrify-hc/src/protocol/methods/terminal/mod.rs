//! The requests that act on terminals, in three roles.
//!
//! `lifecycle` holds the requests that start, list, inspect, end and describe terminals,
//! `interaction` holds the ones that read from, attach to, send to and wait on a terminal, and
//! `put` holds the ones that send a file to a terminal, whole or in pieces. This file only lists
//! them.

mod interaction;
mod lifecycle;
mod put;

#[doc(inline)]
pub use interaction::{
    TerminalAttach, TerminalInterrupt, TerminalKeys, TerminalRead, TerminalSend, TerminalWait,
    WaitTarget,
};
#[doc(inline)]
pub use lifecycle::{
    Command, Keep, LabelChange, LabelChanges, TerminalGet, TerminalKeep, TerminalKill,
    TerminalLabel, TerminalList, TerminalRm, TerminalRun,
};
#[doc(inline)]
pub use put::{
    TerminalPut, TerminalPutAbort, TerminalPutBegin, TerminalPutChunk, TerminalPutFinish,
};
