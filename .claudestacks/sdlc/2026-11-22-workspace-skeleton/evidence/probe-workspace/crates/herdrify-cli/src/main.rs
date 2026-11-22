//! The `herdrify` command.
//!
//! Prints the command's name and version on standard output and exits.

fn main() {
    println!("{}", version_line());
}

/// The line the command prints: its name and its package version.
fn version_line() -> String {
    format!("herdrify {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::version_line;

    #[test]
    fn version_line_names_the_command_and_its_package_version() {
        assert_eq!(
            version_line(),
            format!("herdrify {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
