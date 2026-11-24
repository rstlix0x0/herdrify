//! The refusal of a herdr-app file that cannot be read.
//!
//! A layout, theme or keys file can fail in many ways: bad JSON, a missing field, a wrong type, a
//! malformed colour. [`FileError`] reports all of them the same way, as the kind of file and the
//! reason, so a caller can show one line without matching on the cause.
//!
//! Responsibilities:
//!
//! - [`FileKind`], which of the three files was being read
//! - [`FileError`], the file kind and the reason it was refused
//!
//! Non-responsibilities:
//!
//! - classifying the reason: the detail is text for a person, not a value to match on
//! - reading the file or locating it, which a layer above does

use std::fmt;

use thiserror::Error;

/// Which herdr-app file was being read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::FileKind;
///
/// assert_eq!(FileKind::Keys.to_string(), "keys");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A layout file.
    Layout,
    /// A theme file.
    Theme,
    /// A keys file.
    Keys,
}

impl fmt::Display for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Layout => "layout",
            Self::Theme => "theme",
            Self::Keys => "keys",
        })
    }
}

/// The refusal of a layout, theme or keys file.
///
/// It displays as `invalid <kind> file: <detail>`, where the kind is `layout`, `theme` or `keys`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{FileError, FileKind};
///
/// let refused = FileError::new(FileKind::Theme, "missing field `colors` at line 1 column 2");
/// assert_eq!(
///     refused.to_string(),
///     "invalid theme file: missing field `colors` at line 1 column 2",
/// );
/// assert_eq!(refused.file(), FileKind::Theme);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid {file} file: {detail}")]
#[non_exhaustive]
pub struct FileError {
    file: FileKind,
    detail: String,
}

impl FileError {
    /// Builds the refusal of a `file` for the reason `detail`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{FileError, FileKind};
    ///
    /// let refused = FileError::new(FileKind::Layout, "expected value at line 1 column 1");
    /// assert_eq!(
    ///     refused.to_string(),
    ///     "invalid layout file: expected value at line 1 column 1",
    /// );
    /// ```
    pub fn new(file: FileKind, detail: impl Into<String>) -> Self {
        Self {
            file,
            detail: detail.into(),
        }
    }

    /// Builds the refusal of a `file` that `source` could not read, keeping the text of `source`
    /// as the detail.
    pub(crate) fn from_serde(file: FileKind, source: &serde_json::Error) -> Self {
        Self::new(file, source.to_string())
    }

    /// The file that was refused.
    #[must_use]
    pub const fn file(&self) -> FileKind {
        self.file
    }

    /// The reason the file was refused.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_error_display_names_the_file() {
        let error = FileError::new(FileKind::Theme, "x");
        assert_eq!(error.to_string(), "invalid theme file: x");
        assert_eq!(error.file(), FileKind::Theme);
        assert_eq!(error.detail(), "x");
    }

    #[test]
    fn file_kind_words() {
        assert_eq!(FileKind::Layout.to_string(), "layout");
        assert_eq!(FileKind::Theme.to_string(), "theme");
        assert_eq!(FileKind::Keys.to_string(), "keys");
    }

    #[test]
    fn from_serde_keeps_the_serde_text_verbatim() -> Result<(), String> {
        let Err(source) = serde_json::from_str::<serde_json::Value>("") else {
            return Err("an empty text was read".to_owned());
        };
        let error = FileError::from_serde(FileKind::Layout, &source);
        assert_eq!(error.file(), FileKind::Layout);
        assert_eq!(error.detail(), source.to_string());
        assert_eq!(
            error.to_string(),
            "invalid layout file: EOF while parsing a value at line 1 column 0"
        );
        Ok(())
    }
}
