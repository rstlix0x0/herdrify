//! What one herdr-app run produced, and what it means.
//!
//! A run ends with an exit status and two byte streams. [`RunOutput`] holds them as they came, and
//! [`classify`] turns them into the stdout bytes of a successful run or a typed [`Failure`]. The
//! rules read only the exit status, stdout and stderr, so they hold for any way of running the
//! program.
//!
//! Responsibilities:
//!
//! - the exit status, the output of a run, and the four ways a run can fail
//! - telling a refusal by the window, a missing window, a rejected command line and an
//!   unexpected ending apart
//!
//! Non-responsibilities:
//!
//! - starting the program or capturing its output, which a layer above does
//! - decoding the stdout of a successful run, which depends on the command that ran

use serde_json::Value;

/// How a herdr-app process ended.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Exit;
///
/// assert_ne!(Exit::Code(0), Exit::Code(1));
/// assert_ne!(Exit::Code(0), Exit::Signal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The process exited with this status code.
    Code(i32),
    /// The process was ended by a signal, so it has no status code.
    Signal,
}

/// Everything one herdr-app run produced.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Exit, RunOutput};
///
/// let output = RunOutput {
///     exit: Exit::Code(0),
///     stdout: b"{}".to_vec(),
///     stderr: Vec::new(),
/// };
/// assert_eq!(output.exit, Exit::Code(0));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    /// How the process ended.
    pub exit: Exit,
    /// The bytes the process wrote to standard output.
    pub stdout: Vec<u8>,
    /// The bytes the process wrote to standard error.
    pub stderr: Vec<u8>,
}

/// Why a herdr-app run did not succeed.
///
/// This is a classification of a finished run, not the error a caller sees: the client turns a
/// `Failure` into its own error, which carries the message and the label of the invocation.
/// [`classify`] builds these.
///
/// The text in each variant comes from a different stream:
///
/// - [`Refused`](Self::Refused) holds the `error` string from stdout, unchanged.
/// - [`NoWindow`](Self::NoWindow) and [`Cli`](Self::Cli) hold stderr with trailing whitespace
///   removed and one leading `herdr-app: ` stripped.
/// - [`Unexpected`](Self::Unexpected) holds stderr with whitespace removed from both ends.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{classify, Exit, Failure, RunOutput};
///
/// let output = RunOutput {
///     exit: Exit::Code(2),
///     stdout: Vec::new(),
///     stderr: b"herdr-app: args are not JSON\n".to_vec(),
/// };
/// assert_eq!(
///     classify(&output),
///     Err(Failure::Cli {
///         message: "args are not JSON".to_owned(),
///     }),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The window answered with an error object on stdout and exit code 1.
    Refused {
        /// The text of the window's `error` field.
        message: String,
    },
    /// There is no window to answer, so the command could not be delivered.
    NoWindow {
        /// herdr-app's description of the missing window.
        message: String,
    },
    /// herdr-app rejected the command line itself, with exit code 2.
    Cli {
        /// herdr-app's description of what it rejected.
        message: String,
    },
    /// The run ended in a way no other variant describes.
    Unexpected {
        /// How the process ended.
        exit: Exit,
        /// Standard error as lossy UTF-8 with surrounding whitespace removed, possibly empty.
        stderr: String,
    },
}

/// Reads one run: its stdout when it succeeded, or the [`Failure`] it ended in.
///
/// The [`Failure`] classifies the run; the client wraps it into the error callers see.
///
/// The rules apply in order, and the first that matches decides:
///
/// 1. exit code 0 succeeds with the stdout bytes
/// 2. exit code 1 with a stdout that is a JSON object whose `error` is a string is
///    [`Failure::Refused`] with that string
/// 3. exit code 2 reads stderr as lossy UTF-8, trims trailing whitespace and strips one leading
///    `herdr-app: `. When what remains starts with `no Herdr window at `, the run is
///    [`Failure::NoWindow`]; otherwise it is [`Failure::Cli`]
/// 4. anything else is [`Failure::Unexpected`]
///
/// # Errors
///
/// Returns the [`Failure`] the rules above select for any run that did not exit with code 0.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{classify, Exit, Failure, RunOutput};
///
/// let ok = RunOutput {
///     exit: Exit::Code(0),
///     stdout: b"[]".to_vec(),
///     stderr: Vec::new(),
/// };
/// assert_eq!(classify(&ok), Ok(&b"[]"[..]));
///
/// let refused = RunOutput {
///     exit: Exit::Code(1),
///     stdout: br#"{"error":"the window did not answer"}"#.to_vec(),
///     stderr: Vec::new(),
/// };
/// assert_eq!(
///     classify(&refused),
///     Err(Failure::Refused {
///         message: "the window did not answer".to_owned(),
///     }),
/// );
/// ```
pub fn classify(output: &RunOutput) -> Result<&[u8], Failure> {
    match output.exit {
        Exit::Code(0) => return Ok(&output.stdout),
        Exit::Code(1) => {
            if let Some(message) = error_message(&output.stdout) {
                return Err(Failure::Refused { message });
            }
        }
        Exit::Code(2) => {
            let text = String::from_utf8_lossy(&output.stderr);
            let trimmed = text.trim_end();
            let message = trimmed
                .strip_prefix("herdr-app: ")
                .unwrap_or(trimmed)
                .to_owned();
            return Err(if message.starts_with("no Herdr window at ") {
                Failure::NoWindow { message }
            } else {
                Failure::Cli { message }
            });
        }
        Exit::Code(_) | Exit::Signal => {}
    }
    Err(Failure::Unexpected {
        exit: output.exit,
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// The `error` string of a stdout that is a JSON object, if it has one.
fn error_message(stdout: &[u8]) -> Option<String> {
    match serde_json::from_slice::<Value>(stdout).ok()? {
        Value::Object(map) => map.get("error")?.as_str().map(str::to_owned),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn run(exit: Exit, stdout: &str, stderr: &str) -> RunOutput {
        RunOutput {
            exit,
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    /// The stdout of a run that classified as a success.
    fn succeeded(output: &RunOutput) -> Result<&[u8], String> {
        classify(output).map_err(|failure| format!("classified as a failure: {failure:?}"))
    }

    /// The failure of a run that classified as one.
    fn failed(output: &RunOutput) -> Result<Failure, String> {
        match classify(output) {
            Ok(stdout) => Err(format!("classified as a success: {stdout:?}")),
            Err(failure) => Ok(failure),
        }
    }

    #[test]
    fn classify_exit_zero_is_stdout() -> TestResult {
        let output = run(Exit::Code(0), "{\"ok\":true}", "ignored");
        assert_eq!(succeeded(&output)?, b"{\"ok\":true}");
        Ok(())
    }

    #[test]
    fn classify_refusal() -> TestResult {
        let output = run(
            Exit::Code(1),
            r#"{"error":"tab.new focus is true or false"}"#,
            "",
        );
        assert_eq!(
            failed(&output)?,
            Failure::Refused {
                message: "tab.new focus is true or false".to_owned(),
            },
        );
        Ok(())
    }

    #[test]
    fn classify_exit_one_without_error_object_is_unexpected() -> TestResult {
        for stdout in ["oops", r#"{"error":3}"#, r#"["error"]"#, "{}"] {
            let output = run(Exit::Code(1), stdout, "  bad \n");
            assert_eq!(
                failed(&output)?,
                Failure::Unexpected {
                    exit: Exit::Code(1),
                    stderr: "bad".to_owned(),
                },
                "stdout {stdout:?}",
            );
        }
        Ok(())
    }

    #[test]
    fn classify_no_window() -> TestResult {
        let output = run(
            Exit::Code(2),
            "",
            "herdr-app: no Herdr window at /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)\n",
        );
        assert_eq!(
            failed(&output)?,
            Failure::NoWindow {
                message: "no Herdr window at /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)"
                    .to_owned(),
            },
        );
        Ok(())
    }

    #[test]
    fn classify_no_window_without_prefix() -> TestResult {
        let output = run(
            Exit::Code(2),
            "",
            "no Herdr window at /tmp/x.sock: No such file or directory (os error 2)\n",
        );
        assert_eq!(
            failed(&output)?,
            Failure::NoWindow {
                message: "no Herdr window at /tmp/x.sock: No such file or directory (os error 2)"
                    .to_owned(),
            },
        );
        Ok(())
    }

    #[test]
    fn classify_cli_error() -> TestResult {
        let output = run(
            Exit::Code(2),
            "",
            "herdr-app: args are not JSON: key must be a string at line 1 column 2\n",
        );
        assert_eq!(
            failed(&output)?,
            Failure::Cli {
                message: "args are not JSON: key must be a string at line 1 column 2".to_owned(),
            },
        );
        Ok(())
    }

    #[test]
    fn classify_strips_one_prefix() -> TestResult {
        let output = run(
            Exit::Code(2),
            "",
            "herdr-app: herdr-app: args are not JSON\n",
        );
        assert_eq!(
            failed(&output)?,
            Failure::Cli {
                message: "herdr-app: args are not JSON".to_owned(),
            },
        );
        Ok(())
    }

    #[test]
    fn classify_signal_and_other_codes() -> TestResult {
        for exit in [Exit::Signal, Exit::Code(3)] {
            let output = run(exit, "", "  boom\n");
            assert_eq!(
                failed(&output)?,
                Failure::Unexpected {
                    exit,
                    stderr: "boom".to_owned(),
                },
            );
        }
        Ok(())
    }

    #[test]
    fn classify_recorded_pretty_refusal() -> TestResult {
        let stdout = concat!(
            "{\n",
            "  \"error\": \"tab.new: unknown argument(s): bogus; valid arguments: name, space, folder, argv, task, mod, props, focus\"\n",
            "}\n",
        );
        let output = run(Exit::Code(1), stdout, "");
        assert_eq!(
            failed(&output)?,
            Failure::Refused {
                message: "tab.new: unknown argument(s): bogus; valid arguments: name, space, folder, argv, task, mod, props, focus"
                    .to_owned(),
            },
        );
        Ok(())
    }
}
