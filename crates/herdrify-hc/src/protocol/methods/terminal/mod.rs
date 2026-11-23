//! The requests that act on terminals, in two roles.
//!
//! `lifecycle` holds the requests that start, list, inspect, end and describe terminals, and
//! `interaction` holds the ones that read from, attach to, send to and wait on a terminal. This
//! file only lists them.

mod interaction;
mod lifecycle;

#[doc(inline)]
pub use interaction::{
    TerminalAttach, TerminalInterrupt, TerminalKeys, TerminalPut, TerminalRead, TerminalSend,
    TerminalWait, WaitTarget,
};
#[doc(inline)]
pub use lifecycle::{
    Command, Keep, LabelChange, LabelChanges, TerminalGet, TerminalKeep, TerminalKill,
    TerminalLabel, TerminalList, TerminalRm, TerminalRun,
};
