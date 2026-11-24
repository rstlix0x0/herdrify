//! The names most programs need, in one import: `use herdrify_app::prelude::*;`.
//!
//! It brings in the [`HerdrApp`] entrypoint, the [`Error`] its calls end with, and the enums its
//! operations and options take as arguments, such as a [`Direction`] or a [`Focus`]. Everything
//! else stays under its module, such as the typed commands in [`protocol`](crate::protocol).
//!
//! This file only re-exports items, so it carries no unit tests (unit-test-mandate exemption #1,
//! export-only).

#[doc(inline)]
pub use crate::driver::Error;

#[doc(inline)]
pub use crate::herdr_app::HerdrApp;

#[doc(inline)]
pub use crate::protocol::{
    At, ClickTarget, Direction, FeedbackFrom, Focus, Place, ScrollEdge, SelectMode, SelectionMove,
    SettingsSection, SplitDirection,
};
