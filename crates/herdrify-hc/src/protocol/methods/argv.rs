//! `Argv`: a program and its arguments, never empty.
//!
//! An empty list cannot be built: "no command" has its own explicit form, the terminal's shell, so
//! the first element is always the program and the rest are its arguments. It is shared by every
//! request that starts a process from a list.
//!
//! Responsibilities:
//!
//! - [`Argv`], a non-empty list of strings that serializes as a JSON array
//!
//! Non-responsibilities:
//!
//! - running the program or deciding which programs the core accepts, which the core decides

use serde::{Serialize, Serializer};

use crate::protocol::EmptyValueError;

/// A program and its arguments, run as given with no shell.
///
/// The list is never empty: the first element is the program. For `terminal.run` the list may
/// instead be a template name and its task.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Argv, EmptyValueError};
///
/// # fn main() -> Result<(), EmptyValueError> {
/// let argv = Argv::new(vec!["sleep".to_owned(), "5".to_owned()])?;
/// assert_eq!(argv.program(), "sleep");
/// assert_eq!(argv.args(), ["5"]);
/// assert!(Argv::new(Vec::new()).is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argv {
    program: String,
    args: Vec<String>,
}

impl Argv {
    /// Wraps `items`, whose first element is the program.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyValueError`] naming `argv` when `items` is empty.
    pub fn new(items: Vec<String>) -> Result<Self, EmptyValueError> {
        let mut items = items.into_iter();
        items
            .next()
            .map(|program| Self {
                program,
                args: items.collect(),
            })
            .ok_or(EmptyValueError { what: "argv" })
    }

    /// The program: the first element.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments: every element after the program.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }
}

impl Serialize for Argv {
    /// Writes the program and then the arguments as one JSON array.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(std::iter::once(&self.program).chain(&self.args))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::Argv;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn argv_refuses_an_empty_list() {
        let refused = Argv::new(vec![]);
        assert_eq!(
            refused.map_err(|error| error.to_string()),
            Err("argv must not be empty".to_owned())
        );
    }

    #[test]
    fn argv_splits_program_and_args() -> TestResult {
        let words = ["ssh", "box", "hc", "connect"].map(str::to_owned).to_vec();
        let argv = Argv::new(words)?;
        assert_eq!(argv.program(), "ssh");
        assert_eq!(argv.args(), ["box", "hc", "connect"]);
        assert_eq!(
            serde_json::to_string(&argv)?,
            r#"["ssh","box","hc","connect"]"#
        );
        Ok(())
    }

    #[test]
    fn argv_of_one_element_has_no_args() -> TestResult {
        let argv = Argv::new(vec!["true".to_owned()])?;
        assert_eq!(argv.program(), "true");
        assert!(argv.args().is_empty());
        assert_eq!(serde_json::to_string(&argv)?, r#"["true"]"#);
        Ok(())
    }
}
