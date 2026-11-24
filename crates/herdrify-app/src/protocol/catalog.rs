//! The commands herdr-app lists, as the decoded listing and as a built-in table.
//!
//! `herdr-app commands` prints every command with its arguments and a description.
//! [`CommandInfo`] decodes one entry of that listing, and [`KNOWN_COMMANDS`] is the table of the
//! commands herdr-app 0.1.17 lists ([`AppVersion::PROVEN`]), so a program can compare what it knows
//! with what an installed herdr-app reports. The oldest supported release,
//! 0.1.16 ([`AppVersion::MINIMUM`]), lists 75 commands: it lacks `pane.rename` and `pane.resize`.
//!
//! Responsibilities:
//!
//! - decoding one entry of the `commands` listing into its name, arguments and description
//! - holding the names, argument names and `required` flags of the commands herdr-app 0.1.17 lists,
//!   in the order it lists them
//!
//! Non-responsibilities:
//!
//! - running `herdr-app commands` to obtain the listing, which a layer above does
//! - deciding what a difference between the table and a listing means, which the caller decides

use serde::Deserialize;

use crate::files::CommandName;
#[cfg(doc)]
use crate::protocol::AppVersion;

/// One argument of a command, as the `commands` listing describes it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::ArgInfo;
///
/// let arg: ArgInfo = serde_json::from_str(r#"{"name":"id","required":true}"#)?;
/// assert_eq!(arg.name, "id");
/// assert!(arg.required);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ArgInfo {
    /// The argument's name, as herdr-app printed it. It is a plain `String`, not a newtype: an output
    /// value is kept unchecked, so a listing never fails to decode.
    pub name: String,
    /// Whether herdr-app refuses the command without this argument.
    pub required: bool,
}

/// One command of the `commands` listing: its name, its arguments in the listed order, and what it
/// does.
///
/// Fields herdr-app adds later are ignored.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::CommandInfo;
///
/// let info: CommandInfo = serde_json::from_str(
///     r#"{"name":"space.close","args":[{"name":"id","required":false}],"about":"Close a space."}"#,
/// )?;
/// assert_eq!(info.name.as_str(), "space.close");
/// assert_eq!(info.args.len(), 1);
/// assert_eq!(info.about, "Close a space.");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandInfo {
    /// The command's dotted name, such as `space.new`.
    pub name: CommandName,
    /// The command's arguments, in the order herdr-app lists them.
    pub args: Vec<ArgInfo>,
    /// The command's description.
    pub about: String,
}

/// A command of herdr-app 0.1.17 ([`AppVersion::PROVEN`]), with the argument names and `required`
/// flags it lists, in the listed order.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::KNOWN_COMMANDS;
///
/// let space_new = KNOWN_COMMANDS.iter().find(|command| command.name == "space.new");
/// assert_eq!(space_new.map(|command| command.args.len()), Some(7));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownCommand {
    /// The command's dotted name.
    pub name: &'static str,
    /// Each argument's name and whether it is required, in the listed order.
    pub args: &'static [(&'static str, bool)],
}

/// The 77 commands herdr-app 0.1.17 ([`AppVersion::PROVEN`]) lists, in the order it lists them.
///
/// Release 0.1.16 ([`AppVersion::MINIMUM`]) lists 75 of them: it lacks `pane.rename` and
/// `pane.resize`.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::KNOWN_COMMANDS;
///
/// let space_new = KNOWN_COMMANDS.iter().find(|command| command.name == "space.new");
/// assert!(space_new.is_some_and(|command| command.args.iter().all(|(_, required)| !required)));
/// ```
pub const KNOWN_COMMANDS: &[KnownCommand] = &[
    KnownCommand {
        name: "space.new",
        args: &[
            ("name", false),
            ("folder", false),
            ("tab", false),
            ("argv", false),
            ("task", false),
            ("machine", false),
            ("focus", false),
        ],
    },
    KnownCommand {
        name: "space.close",
        args: &[("id", false), ("confirmed", false)],
    },
    KnownCommand {
        name: "space.rename",
        args: &[("id", false), ("name", false)],
    },
    KnownCommand {
        name: "space.move",
        args: &[("id", true), ("index", true)],
    },
    KnownCommand {
        name: "tab.new",
        args: &[
            ("name", false),
            ("space", false),
            ("folder", false),
            ("argv", false),
            ("task", false),
            ("mod", false),
            ("props", false),
            ("focus", false),
        ],
    },
    KnownCommand {
        name: "tab.close",
        args: &[("id", false), ("confirmed", false)],
    },
    KnownCommand {
        name: "tab.rename",
        args: &[("id", false), ("name", false)],
    },
    KnownCommand {
        name: "tab.move",
        args: &[("id", false), ("index", false), ("by", false)],
    },
    KnownCommand {
        name: "pane.split",
        args: &[
            ("direction", true),
            ("pane", false),
            ("argv", false),
            ("mod", false),
            ("props", false),
        ],
    },
    KnownCommand {
        name: "pane.close",
        args: &[("pane", false)],
    },
    KnownCommand {
        name: "terminal.open",
        args: &[
            ("argv", false),
            ("folder", false),
            ("place", false),
            ("name", false),
        ],
    },
    KnownCommand {
        name: "terminal.keep",
        args: &[("terminal", true), ("place", false)],
    },
    KnownCommand {
        name: "terminal.park",
        args: &[("terminal", true)],
    },
    KnownCommand {
        name: "terminal.right_click",
        args: &[("to", true), ("terminal", false)],
    },
    KnownCommand {
        name: "terminal.pin",
        args: &[("terminal", true), ("pinned", true)],
    },
    KnownCommand {
        name: "terminal.kill",
        args: &[("terminal", true)],
    },
    KnownCommand {
        name: "space.select",
        args: &[("index", false), ("id", false)],
    },
    KnownCommand {
        name: "space.next",
        args: &[],
    },
    KnownCommand {
        name: "space.prev",
        args: &[],
    },
    KnownCommand {
        name: "tab.select",
        args: &[("index", false), ("id", false)],
    },
    KnownCommand {
        name: "tab.settle",
        args: &[("id", false)],
    },
    KnownCommand {
        name: "tab.unsettle",
        args: &[("id", false)],
    },
    KnownCommand {
        name: "tab.next",
        args: &[],
    },
    KnownCommand {
        name: "tab.prev",
        args: &[],
    },
    KnownCommand {
        name: "pane.zoom",
        args: &[("pane", false)],
    },
    KnownCommand {
        name: "pane.rename",
        args: &[("pane", false), ("name", false)],
    },
    KnownCommand {
        name: "pane.focus",
        args: &[("pane", false), ("terminal", false)],
    },
    KnownCommand {
        name: "pane.go",
        args: &[("direction", true)],
    },
    KnownCommand {
        name: "pane.resize",
        args: &[("direction", true), ("pane", false), ("px", false)],
    },
    KnownCommand {
        name: "pane.swap",
        args: &[("direction", false), ("pane", false), ("target", false)],
    },
    KnownCommand {
        name: "terminal.peek",
        args: &[("terminal", true)],
    },
    KnownCommand {
        name: "terminal.show",
        args: &[("terminal", false), ("pane", false)],
    },
    KnownCommand {
        name: "terminal.select_all",
        args: &[("terminal", false)],
    },
    KnownCommand {
        name: "terminal.select",
        args: &[("terminal", false), ("at", false), ("mode", false)],
    },
    KnownCommand {
        name: "terminal.adjust_selection",
        args: &[("terminal", false), ("direction", true)],
    },
    KnownCommand {
        name: "terminal.clear_selection",
        args: &[("terminal", false)],
    },
    KnownCommand {
        name: "terminal.copy",
        args: &[],
    },
    KnownCommand {
        name: "terminal.scroll",
        args: &[
            ("lines", false),
            ("pages", false),
            ("to", false),
            ("terminal", false),
        ],
    },
    KnownCommand {
        name: "terminal.paste",
        args: &[],
    },
    KnownCommand {
        name: "terminal.paste_image",
        args: &[],
    },
    KnownCommand {
        name: "terminal.search",
        args: &[],
    },
    KnownCommand {
        name: "agent.focus",
        args: &[("terminal", true)],
    },
    KnownCommand {
        name: "agent.next",
        args: &[],
    },
    KnownCommand {
        name: "agent.prev",
        args: &[],
    },
    KnownCommand {
        name: "layout.switch",
        args: &[("name", false)],
    },
    KnownCommand {
        name: "layout.clone",
        args: &[("name", true), ("as", false)],
    },
    KnownCommand {
        name: "mod.clone",
        args: &[("name", true), ("as", false)],
    },
    KnownCommand {
        name: "layout.set",
        args: &[("layout", true)],
    },
    KnownCommand {
        name: "layout.toggle",
        args: &[("id", true)],
    },
    KnownCommand {
        name: "layout.show",
        args: &[("id", true)],
    },
    KnownCommand {
        name: "layout.hide",
        args: &[("id", true)],
    },
    KnownCommand {
        name: "layout.resize",
        args: &[("id", true), ("px", false), ("weight", false)],
    },
    KnownCommand {
        name: "layout.select",
        args: &[("stack", true), ("child", true)],
    },
    KnownCommand {
        name: "layout.reset",
        args: &[("name", false)],
    },
    KnownCommand {
        name: "mod.open",
        args: &[("name", true)],
    },
    KnownCommand {
        name: "overlay.close",
        args: &[("id", false)],
    },
    KnownCommand {
        name: "app.help",
        args: &[],
    },
    KnownCommand {
        name: "app.welcome",
        args: &[],
    },
    KnownCommand {
        name: "app.whats_new",
        args: &[],
    },
    KnownCommand {
        name: "app.feedback",
        args: &[("message", false), ("from", false)],
    },
    KnownCommand {
        name: "app.palette",
        args: &[],
    },
    KnownCommand {
        name: "app.settings",
        args: &[("section", false)],
    },
    KnownCommand {
        name: "app.check_updates",
        args: &[],
    },
    KnownCommand {
        name: "app.update_core",
        args: &[],
    },
    KnownCommand {
        name: "app.reset",
        args: &[],
    },
    KnownCommand {
        name: "app.theme",
        args: &[("name", true)],
    },
    KnownCommand {
        name: "app.zoom_in",
        args: &[],
    },
    KnownCommand {
        name: "app.zoom_out",
        args: &[],
    },
    KnownCommand {
        name: "app.zoom_reset",
        args: &[],
    },
    KnownCommand {
        name: "app.update",
        args: &[],
    },
    KnownCommand {
        name: "app.quit",
        args: &[],
    },
    KnownCommand {
        name: "app.hide",
        args: &[],
    },
    KnownCommand {
        name: "app.hide_others",
        args: &[],
    },
    KnownCommand {
        name: "app.show_all",
        args: &[],
    },
    KnownCommand {
        name: "window.close",
        args: &[],
    },
    KnownCommand {
        name: "window.minimize",
        args: &[],
    },
    KnownCommand {
        name: "window.maximize",
        args: &[],
    },
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn find(name: &str) -> Result<&'static KnownCommand, String> {
        KNOWN_COMMANDS
            .iter()
            .find(|command| command.name == name)
            .ok_or_else(|| format!("{name} is not listed"))
    }

    #[test]
    fn known_commands_has_77_unique_names() {
        let names: HashSet<&str> = KNOWN_COMMANDS.iter().map(|command| command.name).collect();
        assert_eq!(KNOWN_COMMANDS.len(), 77);
        assert_eq!(names.len(), 77);
    }

    #[test]
    fn known_commands_include_the_0_1_17_additions() -> Result<(), String> {
        assert_eq!(
            find("pane.rename")?.args,
            [("pane", false), ("name", false)]
        );
        assert_eq!(
            find("pane.resize")?.args,
            [("direction", true), ("pane", false), ("px", false)]
        );
        Ok(())
    }

    // recorded from herdr-app 0.1.17 (`space.new`, the first entry of `herdr-app commands`)
    const RECORDED_SPACE_NEW: &str = r#"{"name":"space.new","args":[{"name":"name","required":false},{"name":"folder","required":false},{"name":"tab","required":false},{"name":"argv","required":false},{"name":"task","required":false},{"name":"machine","required":false},{"name":"focus","required":false}],"about":"Make a space: its name (default: none, so it's called after where its first terminal is), the folder its terminals start in (default: the focused terminal's), its first tab's name (default: none, so it's called by its number), optionally an agent (argv) and its first message (task), and the machine it lives on (`hc machine ls`; default the focused space's machine for keys and clicks, this computer for scripts; empty string: this computer; its terminals start in that machine's home unless folder). It is shown at once; focus false keeps the shown space and tab (with no space shown, the new space is shown)."}"#;

    #[test]
    fn command_info_decodes_a_recorded_listing_entry() -> Result<(), serde_json::Error> {
        let info: CommandInfo = serde_json::from_str(RECORDED_SPACE_NEW)?;
        assert_eq!(info.name.as_str(), "space.new");
        assert_eq!(info.args.len(), 7);
        assert!(info.args.iter().all(|arg| !arg.required));
        assert!(info.about.starts_with("Make a space: its name"));
        assert!(
            info.about
                .ends_with("(with no space shown, the new space is shown).")
        );
        Ok(())
    }

    #[test]
    fn command_info_ignores_unknown_fields() -> Result<(), serde_json::Error> {
        let info: CommandInfo = serde_json::from_str(
            r#"{"name":"space.new","args":[{"name":"name","required":false,"extra":1}],"about":"Make a space.","extra":1}"#,
        )?;
        assert_eq!(info.args.len(), 1);
        assert_eq!(info.about, "Make a space.");
        Ok(())
    }
}
