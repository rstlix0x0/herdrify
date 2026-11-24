//! The name newtypes and the folder and argument-list values the herdr-app commands use.
//!
//! A bare `String` cannot say whether it names a space, a tab or a pane, so each name has its own
//! type and a command argument cannot take one where another belongs. They are built by the same
//! macro as the names in [`files`](crate::files), so they check, read and write the same way. A
//! folder and an argument list carry more than a name, so they have their own checks.
//!
//! Responsibilities:
//!
//! - the eleven names, each checked once at construction and each read from and written as a plain
//!   string, refusing an empty value with [`EmptyValueError`]
//! - [`Folder`], an absolute UTF-8 path, and [`FolderError`], the reasons one is refused
//! - [`Argv`], a non-empty list of arguments that keeps its order
//!
//! Non-responsibilities:
//!
//! - the names a layout file also uses, which [`files`](crate::files) owns and this layer reuses
//! - rejecting a non-empty value herdr-app would refuse, which herdr-app decides
//! - resolving or expanding a folder, or checking that it exists, which the caller does

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::files::{EmptyValueError, name};

name! {
    "protocol",
    /// The identifier of a space, as herdr-app prints it.
    ///
    /// Only the empty string is refused.
    SpaceId, "space id", "sp1"
}

name! {
    "protocol",
    /// The identifier of a tab, as herdr-app prints it.
    ///
    /// Only the empty string is refused.
    TabId, "tab id", "t1"
}

name! {
    "protocol",
    /// The identifier of a pane, as herdr-app prints it.
    ///
    /// Only the empty string is refused.
    PaneId, "pane id", "p1"
}

name! {
    "protocol",
    /// The identifier of an overlay, as herdr-app prints it.
    ///
    /// Only the empty string is refused.
    OverlayId, "overlay id", "frame-welcome"
}

name! {
    "protocol",
    /// The name of a space.
    ///
    /// Only the empty string is refused.
    SpaceName, "space name", "work"
}

name! {
    "protocol",
    /// The name of a tab.
    ///
    /// Only the empty string is refused.
    TabName, "tab name", "build"
}

name! {
    "protocol",
    /// The name of a pane.
    ///
    /// Only the empty string is refused.
    PaneName, "pane name", "logs"
}

name! {
    "protocol",
    /// The name of a terminal a command opens or targets.
    ///
    /// Only the empty string is refused. herdr-core decides which terminals exist.
    TerminalName, "terminal name", "shell"
}

name! {
    "protocol",
    /// The name of a machine a command targets.
    ///
    /// Only the empty string is refused.
    MachineName, "machine name", "devbox"
}

name! {
    "protocol",
    /// The task a command asks herdr-app to run.
    ///
    /// Only the empty string is refused.
    Task, "task", "build"
}

name! {
    "protocol",
    /// The name of an app herdr-app runs.
    ///
    /// Only the empty string is refused.
    AppName, "app name", "claude"
}

/// Why a path cannot be a [`Folder`].
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Folder, FolderError};
///
/// let refused = Folder::new("~/code").unwrap_err();
/// assert!(matches!(refused, FolderError::Relative { .. }));
/// assert_eq!(refused.to_string(), "\"~/code\" is not an absolute path");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FolderError {
    /// The path is empty.
    #[error(transparent)]
    Empty(EmptyValueError),
    /// The path is not absolute. A path starting with `~` is relative, because nothing expands it.
    #[error("\"{path}\" is not an absolute path")]
    Relative {
        /// The path as it was given.
        path: String,
    },
    /// The path is not valid UTF-8, so it cannot be written in a command or a file.
    #[error("\"{}\" is not UTF-8", .path.display())]
    NotUtf8 {
        /// The path as it was given. Its display replaces the bytes that are not UTF-8.
        path: PathBuf,
    },
}

/// A folder a command opens or runs in: an absolute UTF-8 path.
///
/// The path is kept exactly as given. Nothing resolves, normalises or expands it, and nothing
/// checks that it exists. It reads from and writes as a plain string.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Folder;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let absolute = if cfg!(windows) { r"C:\tmp\x" } else { "/tmp/x" };
/// let folder = Folder::new(absolute)?;
/// assert_eq!(folder.as_str(), absolute);
/// assert!(Folder::new("x/y").is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Folder(String);

impl Folder {
    /// Wraps `path` as a folder.
    ///
    /// # Errors
    ///
    /// Returns [`FolderError::Empty`] labelled `folder` when `path` is empty,
    /// [`FolderError::NotUtf8`] when it is not valid UTF-8, and [`FolderError::Relative`] when it
    /// is not absolute. They are checked in that order.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::{Folder, FolderError};
    ///
    /// assert!(matches!(Folder::new(""), Err(FolderError::Empty(_))));
    /// assert!(matches!(Folder::new("./x"), Err(FolderError::Relative { .. })));
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, FolderError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(FolderError::Empty(EmptyValueError { label: "folder" }));
        }
        let text = path.to_str().ok_or_else(|| FolderError::NotUtf8 {
            path: path.to_path_buf(),
        })?;
        if path.is_absolute() {
            Ok(Self(text.to_owned()))
        } else {
            Err(FolderError::Relative {
                path: text.to_owned(),
            })
        }
    }

    /// The wrapped path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Folder {
    type Error = FolderError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Folder> for String {
    fn from(value: Folder) -> Self {
        value.0
    }
}

impl fmt::Display for Folder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The arguments of a program a command runs: a non-empty list that keeps its order.
///
/// It reads from and writes as a JSON array of strings. Only the length of the list is checked: an
/// item, `argv[0]` included, may be empty.
///
/// # Examples
///
/// ```
/// use herdrify_app::{files::EmptyValueError, protocol::Argv};
///
/// # fn main() -> Result<(), EmptyValueError> {
/// let argv = Argv::new(["sh", "-c", "echo a b"])?;
/// assert_eq!(argv.as_slice(), ["sh", "-c", "echo a b"]);
/// assert!(Argv::new(Vec::<String>::new()).is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Argv(Vec<String>);

impl Argv {
    /// Wraps `items` as an argument list, keeping their order.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyValueError`] labelled `argv` when `items` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Argv;
    ///
    /// assert!(Argv::new(["ls"]).is_ok());
    /// assert_eq!(Argv::new(Vec::<String>::new()).unwrap_err().label, "argv");
    /// ```
    pub fn new(
        items: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, EmptyValueError> {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        if items.is_empty() {
            Err(EmptyValueError { label: "argv" })
        } else {
            Ok(Self(items))
        }
    }

    /// The arguments, in the order they were given.
    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl TryFrom<Vec<String>> for Argv {
    type Error = EmptyValueError;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Argv> for Vec<String> {
    fn from(value: Argv) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts that `new("")` refuses with `label`, in both the field and the displayed text.
    fn refuses_with<T: fmt::Debug>(
        refused: Result<T, EmptyValueError>,
        label: &str,
    ) -> Result<(), String> {
        let error = refused.err().ok_or("an empty value was accepted")?;
        if error.label != label {
            return Err(format!("label was {:?}, not {label:?}", error.label));
        }
        let shown = error.to_string();
        if shown != format!("a {label} cannot be empty") {
            return Err(format!("displayed {shown:?}"));
        }
        Ok(())
    }

    #[test]
    fn each_protocol_name_refuses_empty_with_its_label() -> Result<(), String> {
        refuses_with(SpaceId::new(""), "space id")?;
        refuses_with(TabId::new(""), "tab id")?;
        refuses_with(PaneId::new(""), "pane id")?;
        refuses_with(OverlayId::new(""), "overlay id")?;
        refuses_with(SpaceName::new(""), "space name")?;
        refuses_with(TabName::new(""), "tab name")?;
        refuses_with(PaneName::new(""), "pane name")?;
        refuses_with(TerminalName::new(""), "terminal name")?;
        refuses_with(MachineName::new(""), "machine name")?;
        refuses_with(Task::new(""), "task")?;
        refuses_with(AppName::new(""), "app name")
    }

    /// Reads `"sp1"` as `T`, writes it back, and checks `""` is refused.
    fn round_trips<T>() -> Result<(), serde_json::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let value: T = serde_json::from_str("\"sp1\"")?;
        assert_eq!(serde_json::to_string(&value)?, "\"sp1\"");
        assert!(serde_json::from_str::<T>("\"\"").is_err());
        Ok(())
    }

    #[test]
    fn protocol_names_round_trip_through_serde() -> Result<(), serde_json::Error> {
        round_trips::<SpaceId>()?;
        round_trips::<TabId>()?;
        round_trips::<PaneId>()?;
        round_trips::<OverlayId>()?;
        round_trips::<SpaceName>()?;
        round_trips::<TabName>()?;
        round_trips::<PaneName>()?;
        round_trips::<TerminalName>()?;
        round_trips::<MachineName>()?;
        round_trips::<Task>()?;
        round_trips::<AppName>()
    }

    /// An absolute path on the platform the tests run on.
    const ABSOLUTE: &str = if cfg!(windows) { r"C:\tmp\x" } else { "/tmp/x" };

    #[test]
    fn folder_accepts_absolute_paths() -> Result<(), Box<dyn std::error::Error>> {
        let folder = Folder::new(ABSOLUTE)?;
        assert_eq!(folder.as_str(), ABSOLUTE);
        assert_eq!(folder.to_string(), ABSOLUTE);
        assert_eq!(
            serde_json::to_string(&folder)?,
            serde_json::to_string(ABSOLUTE)?
        );
        let read: Folder = serde_json::from_str(&serde_json::to_string(ABSOLUTE)?)?;
        assert_eq!(read, folder);
        Ok(())
    }

    #[test]
    fn folder_refuses_relative_paths() -> Result<(), String> {
        for relative in ["x/y", "./x", "~/code"] {
            let error = Folder::new(relative)
                .err()
                .ok_or_else(|| format!("{relative:?} was accepted"))?;
            let expected = FolderError::Relative {
                path: relative.to_owned(),
            };
            if error != expected {
                return Err(format!("{relative:?} gave {error:?}"));
            }
            let shown = error.to_string();
            if shown != format!("\"{relative}\" is not an absolute path") {
                return Err(format!("displayed {shown:?}"));
            }
        }
        Ok(())
    }

    #[test]
    fn folder_refuses_empty() -> Result<(), String> {
        let error = Folder::new("")
            .err()
            .ok_or("an empty folder was accepted")?;
        if error != FolderError::Empty(EmptyValueError { label: "folder" }) {
            return Err(format!("gave {error:?}"));
        }
        if error.to_string() != "a folder cannot be empty" {
            return Err(format!("displayed {:?}", error.to_string()));
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn folder_refuses_non_utf8() -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;

        let path = std::path::Path::new(std::ffi::OsStr::from_bytes(b"/tmp/\xff"));
        let error = Folder::new(path)
            .err()
            .ok_or("a non-UTF-8 folder was accepted")?;
        if error
            != (FolderError::NotUtf8 {
                path: path.to_path_buf(),
            })
        {
            return Err(format!("gave {error:?}"));
        }
        if error.to_string() != "\"/tmp/\u{fffd}\" is not UTF-8" {
            return Err(format!("displayed {:?}", error.to_string()));
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn folder_checks_utf8_before_absoluteness() -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;

        let path = std::path::Path::new(std::ffi::OsStr::from_bytes(b"x/\xff"));
        let error = Folder::new(path)
            .err()
            .ok_or("a relative non-UTF-8 folder was accepted")?;
        if error
            != (FolderError::NotUtf8 {
                path: path.to_path_buf(),
            })
        {
            return Err(format!("gave {error:?}"));
        }
        Ok(())
    }

    #[test]
    fn folder_refuses_bad_values_on_read() {
        assert!(serde_json::from_str::<Folder>("\"x/y\"").is_err());
        assert!(serde_json::from_str::<Folder>("\"\"").is_err());
    }

    #[test]
    fn argv_refuses_empty_and_keeps_order() -> Result<(), Box<dyn std::error::Error>> {
        let error = Argv::new(Vec::<String>::new())
            .err()
            .ok_or("an empty argv was accepted")?;
        assert_eq!(error, EmptyValueError { label: "argv" });
        assert_eq!(error.to_string(), "a argv cannot be empty");

        let argv = Argv::new(["sh", "-c", "echo a b"])?;
        assert_eq!(argv.as_slice(), ["sh", "-c", "echo a b"]);
        let json = serde_json::to_string(&argv)?;
        assert_eq!(json, r#"["sh","-c","echo a b"]"#);
        assert_eq!(serde_json::from_str::<Argv>(&json)?, argv);
        assert!(serde_json::from_str::<Argv>("[]").is_err());
        Ok(())
    }
}
