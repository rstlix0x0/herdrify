//! The layout, theme and keys files herdr-app 0.1.17 ships, read and written back by the models.
//!
//! Each fixture is a byte-for-byte copy of a file from a fresh herdr-app home. The tests run
//! offline through the public API and check, for every fixture, that it parses, that writing it
//! back gives the same JSON value, that the written text repeats no key, and that reading the
//! written text and writing it again changes nothing.
//!
//! Responsibilities:
//!
//! - the four round-trip checks over every shipped layout, theme and keys file
//!
//! Non-responsibilities:
//!
//! - the models' own behaviour on invalid or unusual input, which their unit tests cover
//! - running herdr-app, which the tests never need

use herdrify_app::files::{FileError, Keys, Layout, Theme};
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A fixture under `fixtures/` as its name and its text, so the name is the path and cannot drift.
macro_rules! fixture {
    ($path:literal) => {
        ($path, include_str!(concat!("fixtures/", $path)))
    };
}

/// Runs the four round-trip checks on one fixture, given as its name and its text.
///
/// `parse` and `to_json` are the model's own reading and writing functions.
fn assert_round_trip<M>(
    (name, source): (&str, &str),
    parse: fn(&str) -> Result<M, FileError>,
    to_json: fn(&M) -> String,
) -> TestResult {
    let model = parse(source).map_err(|error| format!("{name}: does not parse: {error}"))?;
    let written = to_json(&model);

    let source_value: Value = serde_json::from_str(source)?;
    let written_value: Value = serde_json::from_str(&written)?;
    assert_eq!(
        written_value, source_value,
        "{name}: written value differs from the source"
    );

    let expected_len = serde_json::to_string_pretty(&source_value)?.len() + "\n".len();
    assert_eq!(
        written.len(),
        expected_len,
        "{name}: written text has a different length from the pretty-printed value, so a key repeats"
    );

    let reparsed =
        parse(&written).map_err(|error| format!("{name}: written text does not parse: {error}"))?;
    assert_eq!(
        to_json(&reparsed),
        written,
        "{name}: a second round trip changed the text"
    );
    Ok(())
}

#[test]
fn shipped_layouts_round_trip() -> TestResult {
    let fixtures = [
        fixture!("layouts/main.json"),
        fixture!("layouts/terminals.json"),
        fixture!("layouts/win98.json"),
    ];
    for fixture in fixtures {
        assert_round_trip(fixture, Layout::parse, Layout::to_json)?;
    }
    Ok(())
}

#[test]
fn shipped_themes_round_trip() -> TestResult {
    let fixtures = [
        fixture!("themes/atom-one-dark.json"),
        fixture!("themes/win98.json"),
    ];
    for fixture in fixtures {
        assert_round_trip(fixture, Theme::parse, Theme::to_json)?;
    }
    Ok(())
}

#[test]
fn shipped_keys_round_trip() -> TestResult {
    assert_round_trip(fixture!("keys.json"), Keys::parse, Keys::to_json)
}
