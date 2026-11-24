//! The herdr-app command-line language as pure data and rules, with no I/O.
//!
//! Nothing here touches an async runtime, a file or a process, so every type can be built and
//! tested without one. Whatever runs the `herdr-app` program sits above this layer and depends on
//! it. The layer builds on [`files`](crate::files) for the names both share.
//!
//! Responsibilities:
//!
//! - the name newtypes the commands use for spaces, tabs, panes, overlays, terminals, machines,
//!   tasks and apps, each refusing an empty value with the error from [`files`](crate::files)
//! - the folder a command runs in, which must be an absolute UTF-8 path, and the argument list of
//!   a program it runs, which must not be empty
//! - the command line of each herdr-app command, built as the list of arguments that starts it
//! - what one run produced: its exit status and output streams, and the sorting of a run into
//!   the stdout of a success or a typed failure
//! - the herdr-app version, read from the text the program prints and ordered against the oldest
//!   supported release
//! - the `commands` listing, decoded entry by entry, and the table of the commands herdr-app 0.1.17
//!   ([`AppVersion::PROVEN`]) lists, where 0.1.16 ([`AppVersion::MINIMUM`]) lists two fewer
//! - the `ls` listing, decoded into the spaces, tabs and terminals of a window and the spaces
//!   closed recently, with an agent status that keeps words it does not know
//! - the window status, decoded into the focus, the layout in use, the floating nodes and the
//!   problems herdr-app reports
//!
//! Non-responsibilities:
//!
//! - running `herdr-app` and capturing its output, which a layer above does
//! - deciding which command's output a successful run's stdout holds, which the caller knows from
//!   the command it ran
//! - deciding which names herdr-app accepts beyond the empty string, which herdr-app decides
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod catalog;
mod identifiers;
mod invocation;
mod listing;
mod outcome;
mod status;
mod version;

#[doc(inline)]
pub use catalog::{ArgInfo, CommandInfo, KNOWN_COMMANDS, KnownCommand};

#[doc(inline)]
pub use identifiers::{
    AppName, Argv, Folder, FolderError, MachineName, OverlayId, PaneId, PaneName, SpaceId,
    SpaceName, TabId, TabName, Task, TerminalName,
};

#[doc(inline)]
pub use invocation::Invocation;

#[doc(inline)]
pub use listing::{AgentStatus, Listing, RecentSpace, SpaceEntry, TabEntry, TerminalEntry};

#[doc(inline)]
pub use outcome::{Exit, Failure, RunOutput, classify};

#[doc(inline)]
pub use status::{Float, WindowStatus};

#[doc(inline)]
pub use version::{AppVersion, InvalidAppVersion, PreIdentifier};
