//! The names most programs need, in one import: `use herdrify_hc::prelude::*;`.
//!
//! It brings in the [`HerdrCore`] entrypoint and what its calls return or take as options, and
//! the [`Client`] for programs that use the typed requests directly. Everything else stays under
//! its module, such as the request types in [`protocol`](crate::protocol).
//!
//! This file only re-exports items, so it carries no unit tests (the unit-test mandate's first
//! structural exemption).

#[doc(no_inline)]
pub use crate::connection::{AttachEvent, AttachStream, Client, CloseReason, Error, Subscription};
#[doc(no_inline)]
pub use crate::herdr_core::{HerdrCore, HerdrCoreError, Started, TerminalHandle};
#[doc(no_inline)]
pub use crate::protocol::{
    Children, DisplayFacts, ErrorKind, Event, EventKind, IdleOrWorking, Keep, Notification,
};
