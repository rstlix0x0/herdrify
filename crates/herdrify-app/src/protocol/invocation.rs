//! One herdr-app command line, as the argument list that starts it.
//!
//! herdr-app is driven by its command line, so a command is fully described by the arguments the
//! program is started with. [`Invocation`] builds that list for the four commands that take no
//! arguments and for `do`, which carries a command name and its arguments as one JSON object. The
//! list is data: each element is one process argument, passed without a shell, so spaces and
//! quotes in a value need no escaping beyond the JSON itself.
//!
//! Responsibilities:
//!
//! - the argument list of `--version`, `commands`, `ls`, `status` and `do <name> <json>`
//! - the short label of an invocation, for messages that say which command failed
//!
//! Non-responsibilities:
//!
//! - starting the program or reading what it prints, which a layer above does
//! - checking that a command name exists or that its arguments suit it, which herdr-app decides

use serde_json::{Map, Value};

use crate::files::CommandName;

/// Which command line an [`Invocation`] stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Version,
    Commands,
    Ls,
    Status,
    Run { name: CommandName, json: String },
}

/// The arguments that start one herdr-app command.
///
/// Build one with a constructor, then read its argument list with [`argv`](Self::argv) or a short
/// description with [`label`](Self::label).
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Invocation;
///
/// let invocation = Invocation::status();
/// assert_eq!(invocation.argv(), ["status"]);
/// assert_eq!(invocation.label(), "status");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    kind: Kind,
}

impl Invocation {
    /// The invocation that prints herdr-app's version: `--version`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Invocation;
    ///
    /// assert_eq!(Invocation::version().argv(), ["--version"]);
    /// ```
    #[must_use]
    pub const fn version() -> Self {
        Self {
            kind: Kind::Version,
        }
    }

    /// The invocation that lists the commands herdr-app offers: `commands`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Invocation;
    ///
    /// assert_eq!(Invocation::commands().argv(), ["commands"]);
    /// ```
    #[must_use]
    pub const fn commands() -> Self {
        Self {
            kind: Kind::Commands,
        }
    }

    /// The invocation that lists the spaces, their tabs and terminals, in the open window: `ls`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Invocation;
    ///
    /// assert_eq!(Invocation::ls().argv(), ["ls"]);
    /// ```
    #[must_use]
    pub const fn ls() -> Self {
        Self { kind: Kind::Ls }
    }

    /// The invocation that reports the state of herdr-app: `status`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Invocation;
    ///
    /// assert_eq!(Invocation::status().argv(), ["status"]);
    /// ```
    #[must_use]
    pub const fn status() -> Self {
        Self { kind: Kind::Status }
    }

    /// The invocation that runs the command `name` with `arguments`: `do <name> <json>`.
    ///
    /// The arguments are always written as one compact JSON object, `{}` when the map is empty.
    /// The order of the keys follows `serde_json`'s `Map`: sorted by key by default, insertion
    /// order when its `preserve_order` feature is on. Compare the JSON element as parsed JSON,
    /// never as text.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::CommandName;
    /// use herdrify_app::protocol::Invocation;
    /// use serde_json::{json, Map, Value};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let name = CommandName::new("tab.new")?;
    /// let mut arguments = Map::new();
    /// arguments.insert("name".to_owned(), Value::from("api"));
    ///
    /// let argv = Invocation::run(&name, &arguments).argv();
    /// assert_eq!(argv[..2], ["do", "tab.new"]);
    /// assert_eq!(serde_json::from_str::<Value>(&argv[2])?, json!({"name": "api"}));
    /// assert_eq!(Invocation::run(&name, &Map::new()).argv()[2], "{}");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn run(name: &CommandName, arguments: &Map<String, Value>) -> Self {
        Self {
            kind: Kind::Run {
                name: name.clone(),
                json: Value::Object(arguments.clone()).to_string(),
            },
        }
    }

    /// The argument list that starts this command, one element per process argument.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Invocation;
    ///
    /// assert_eq!(Invocation::ls().argv(), ["ls"]);
    /// ```
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        match &self.kind {
            Kind::Version => vec!["--version".to_owned()],
            Kind::Commands => vec!["commands".to_owned()],
            Kind::Ls => vec!["ls".to_owned()],
            Kind::Status => vec!["status".to_owned()],
            Kind::Run { name, json } => {
                vec!["do".to_owned(), name.as_str().to_owned(), json.clone()]
            }
        }
    }

    /// A short description of the command, without its arguments: `--version`, `commands`, `ls`,
    /// `status` or `do <name>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::CommandName;
    /// use herdrify_app::protocol::Invocation;
    /// use serde_json::Map;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let name = CommandName::new("tab.new")?;
    /// assert_eq!(Invocation::run(&name, &Map::new()).label(), "do tab.new");
    /// assert_eq!(Invocation::status().label(), "status");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn label(&self) -> String {
        match &self.kind {
            Kind::Version => "--version".to_owned(),
            Kind::Commands => "commands".to_owned(),
            Kind::Ls => "ls".to_owned(),
            Kind::Status => "status".to_owned(),
            Kind::Run { name, .. } => format!("do {name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn object(value: Value) -> Result<Map<String, Value>, &'static str> {
        match value {
            Value::Object(map) => Ok(map),
            _ => Err("not a JSON object"),
        }
    }

    #[test]
    fn invocation_argv_per_constructor() -> TestResult {
        assert_eq!(Invocation::version().argv(), ["--version"]);
        assert_eq!(Invocation::commands().argv(), ["commands"]);
        assert_eq!(Invocation::ls().argv(), ["ls"]);
        assert_eq!(Invocation::status().argv(), ["status"]);
        let run = Invocation::run(&CommandName::new("tab.new")?, &Map::new());
        assert_eq!(run.argv(), ["do", "tab.new", "{}"]);
        Ok(())
    }

    #[test]
    fn invocation_run_writes_compact_json() -> TestResult {
        let arguments = object(json!({"name": "api", "argv": ["claude"]}))?;
        let argv = Invocation::run(&CommandName::new("tab.new")?, &arguments).argv();
        assert_eq!(argv.len(), 3);
        assert_eq!(argv[0], "do");
        assert_eq!(argv[1], "tab.new");
        let parsed: Value = serde_json::from_str(&argv[2])?;
        assert_eq!(parsed, json!({"argv": ["claude"], "name": "api"}));
        assert!(!argv[2].contains(char::is_whitespace));

        let empty = Invocation::run(&CommandName::new("tab.new")?, &Map::new()).argv();
        assert_eq!(empty[2], "{}");
        Ok(())
    }

    #[test]
    fn invocation_keeps_spaces_and_quotes() -> TestResult {
        let arguments = object(json!({"argv": ["echo \"a b\""]}))?;
        let argv = Invocation::run(&CommandName::new("tab.new")?, &arguments).argv();
        let parsed: Value = serde_json::from_str(&argv[2])?;
        assert_eq!(parsed, json!({"argv": ["echo \"a b\""]}));
        Ok(())
    }

    #[test]
    fn invocation_label_per_constructor() -> TestResult {
        assert_eq!(Invocation::version().label(), "--version");
        assert_eq!(Invocation::commands().label(), "commands");
        assert_eq!(Invocation::ls().label(), "ls");
        assert_eq!(Invocation::status().label(), "status");
        let run = Invocation::run(&CommandName::new("tab.new")?, &Map::new());
        assert_eq!(run.label(), "do tab.new");
        Ok(())
    }
}
