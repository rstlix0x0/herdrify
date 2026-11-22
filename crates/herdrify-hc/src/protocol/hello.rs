//! The `hello` handshake and the core version it reports.
//!
//! A client opens a connection with `hello`, asking for protocol generation 1 and, for a framed
//! connection, framing. The reply names the core, the methods and features it has, and its version.
//! The version decides whether this client can talk to the core at all, so it has its own type that
//! parses and orders versions.
//!
//! Responsibilities:
//!
//! - [`HelloParams`], the optional parameters of `hello`, and its `Method` implementation
//! - [`HelloResult`], the reply with every property the schema lists
//! - [`Generation`], the protocol generation number
//! - [`InvalidCoreVersion`], the error a version that does not parse returns
//! - [`CoreVersion`], a `SemVer` 2.0.0 version with precedence ordering and the minimum this client
//!   supports
//!
//! Non-responsibilities:
//!
//! - sending `hello` or deciding what to do with a version that is too old, which the client does

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{CoreInfo, LoginSession, Method};

/// The protocol generation a connection speaks.
///
/// The field is public so a caller writes `Generation(1)`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::Generation;
///
/// assert_eq!(Generation::ONE.get(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Generation(pub u32);

impl Generation {
    /// Generation 1, the only one the core speaks.
    pub const ONE: Self = Self(1);

    /// The generation number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The parameters of `hello`.
///
/// Nothing is sent that was not set. A client opening a framed connection sends
/// `{"framed":true,"generation":1}`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Generation, HelloParams};
///
/// let params = HelloParams::new().generation(Generation::ONE).framed();
/// assert_eq!(serde_json::to_string(&params)?, r#"{"framed":true,"generation":1}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct HelloParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    calls_you: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    core: Option<CoreInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    framed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation: Option<Generation>,
}

impl HelloParams {
    /// A `hello` that sets nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            calls_you: None,
            core: None,
            features: None,
            framed: None,
            generation: None,
        }
    }

    /// What the dialed core calls this peer, so news about `t7` reaches it as `rohan/t7`.
    #[must_use]
    pub fn calls_you(mut self, name: impl Into<String>) -> Self {
        self.calls_you = Some(name.into());
        self
    }

    /// Identifies the dialing peer as another core.
    #[must_use]
    pub fn core(mut self, core: CoreInfo) -> Self {
        self.core = Some(core);
        self
    }

    /// The features the dialing peer supports.
    #[must_use]
    pub fn features<I, S>(mut self, features: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.features = Some(features.into_iter().map(Into::into).collect());
        self
    }

    /// Switches the connection to framed mode after the reply.
    #[must_use]
    pub const fn framed(mut self) -> Self {
        self.framed = Some(true);
        self
    }

    /// The protocol generation the caller speaks.
    #[must_use]
    pub const fn generation(mut self, generation: Generation) -> Self {
        self.generation = Some(generation);
        self
    }
}

impl Method for HelloParams {
    const NAME: &'static str = "hello";
    type Params = Self;
    type Output = HelloResult;
}

/// The reply to `hello`: what the core is and what it can do.
///
/// `version` stays text so a core that reports a version this client cannot parse is still read;
/// [`CoreVersion::parse`] turns it into a comparable value.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloResult {
    /// Which core answered.
    pub core: CoreInfo,
    /// The features the core supports.
    pub features: Vec<String>,
    /// Whether the connection is framed after this reply.
    pub framed: bool,
    /// The protocol generation the core speaks.
    pub generation: Generation,
    /// The methods the core serves.
    pub methods: Vec<String>,
    /// The core's version.
    pub version: String,
    /// Whether systemd lingering is enabled; absent when unsupported, not applicable or unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keeps_running_after_logout: Option<bool>,
    /// The OS login the core inherited; absent when unavailable or unsupported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_session: Option<LoginSession>,
    /// The account for the administrator's `loginctl enable-linger` command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logout_linger_user: Option<String>,
}

/// The text is not a `SemVer` 2.0.0 version.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::CoreVersion;
///
/// let error = CoreVersion::parse("0.1").err().map(|e| e.to_string());
/// assert_eq!(error.as_deref(), Some(r#""0.1" is not a MAJOR.MINOR.PATCH version"#));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{found:?} is not a MAJOR.MINOR.PATCH version")]
pub struct InvalidCoreVersion {
    /// The text that did not parse.
    pub found: String,
}

/// A core's version, as `SemVer` 2.0.0.
///
/// Versions order by semver precedence: build metadata is ignored, and a pre-release sorts below its
/// release. Equality follows the same rule, so `0.1.21+abc` equals `0.1.21`. `Display` prints the
/// text the version was parsed from.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::CoreVersion;
///
/// let version = CoreVersion::parse("0.1.22")?;
/// assert!(version >= CoreVersion::MINIMUM);
/// assert!(CoreVersion::parse("0.1.21-rc.1")? < CoreVersion::MINIMUM);
/// assert_eq!(version.to_string(), "0.1.22");
/// # Ok::<(), herdrify_hc::protocol::InvalidCoreVersion>(())
/// ```
#[derive(Debug, Clone)]
pub struct CoreVersion {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<String>,
    text: Cow<'static, str>,
}

impl CoreVersion {
    /// The oldest core version this client supports: 0.1.21.
    pub const MINIMUM: Self = Self {
        major: 0,
        minor: 1,
        patch: 21,
        pre: Vec::new(),
        text: Cow::Borrowed("0.1.21"),
    };

    /// Parses `text` as `MAJOR.MINOR.PATCH` with an optional `-pre-release` and `+build`.
    ///
    /// The three numbers are decimals that fit a `u64` with no leading zero, and each pre-release
    /// and build identifier is a non-empty run of `[0-9A-Za-z-]`. A numeric pre-release identifier
    /// has no leading zero, so `1.0.0-01` is refused while `1.0.0-0a` and `1.0.0+01` are accepted.
    /// Surrounding whitespace is refused.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCoreVersion`] when `text` does not follow that grammar.
    pub fn parse(text: &str) -> Result<Self, InvalidCoreVersion> {
        let invalid = || InvalidCoreVersion {
            found: text.to_owned(),
        };
        let (rest, build) = match text.split_once('+') {
            Some((rest, build)) => (rest, Some(build)),
            None => (text, None),
        };
        let (numbers, pre) = match rest.split_once('-') {
            Some((numbers, pre)) => (numbers, Some(pre)),
            None => (rest, None),
        };

        let mut parts = numbers.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        let number = |part: &str| parse_number(part).ok_or_else(invalid);

        let pre = match pre {
            Some(pre) => identifiers(pre)
                .filter(|list| list.iter().all(|identifier| !has_leading_zero(identifier)))
                .ok_or_else(invalid)?,
            None => Vec::new(),
        };
        if let Some(build) = build {
            identifiers(build).ok_or_else(invalid)?;
        }
        Ok(Self {
            major: number(major)?,
            minor: number(minor)?,
            patch: number(patch)?,
            pre,
            text: Cow::Owned(text.to_owned()),
        })
    }
}

/// A decimal that fits a `u64` with no leading zero, except `0` itself.
fn parse_number(part: &str) -> Option<u64> {
    let plain = !part.is_empty()
        && part.bytes().all(|byte| byte.is_ascii_digit())
        && (part == "0" || !part.starts_with('0'));
    if plain { part.parse().ok() } else { None }
}

/// Whether `identifier` is a number written with a leading zero, such as `01`; a lone `0` and an
/// alphanumeric identifier such as `0a` are fine.
fn has_leading_zero(identifier: &str) -> bool {
    is_numeric(identifier) && identifier.len() > 1 && identifier.starts_with('0')
}

/// The dot-separated identifiers of `list`, or `None` when the list or any identifier is empty or
/// holds a character outside `[0-9A-Za-z-]`.
fn identifiers(list: &str) -> Option<Vec<String>> {
    list.split('.')
        .map(|identifier| {
            let valid = !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
            valid.then(|| identifier.to_owned())
        })
        .collect()
}

fn is_numeric(identifier: &str) -> bool {
    identifier.bytes().all(|byte| byte.is_ascii_digit())
}

/// Orders two pre-release identifiers: numbers by value, below words, words by ASCII order.
fn cmp_identifier(a: &str, b: &str) -> Ordering {
    match (is_numeric(a), is_numeric(b)) {
        (true, true) => {
            let (a, b) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
            a.len().cmp(&b.len()).then_with(|| a.cmp(b))
        }
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b),
    }
}

/// Orders two pre-release lists: no list is above any list, then identifier by identifier, with
/// the shorter list below when it is a prefix of the longer.
fn cmp_pre(a: &[String], b: &[String]) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a
            .iter()
            .zip(b)
            .map(|(x, y)| cmp_identifier(x, y))
            .find(|ordering| ordering.is_ne())
            .unwrap_or_else(|| a.len().cmp(&b.len())),
    }
}

impl fmt::Display for CoreVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl Ord for CoreVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| cmp_pre(&self.pre, &other.pre))
    }
}

impl PartialOrd for CoreVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CoreVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for CoreVersion {}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::{CoreVersion, Generation, HelloParams, HelloResult};
    use crate::protocol::{CoreInfo, LoginSession, Method};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn version_table_accepted() -> TestResult {
        for text in [
            "0.1.21",
            "0.1.22",
            "0.2.0",
            "1.0.0",
            "0.1.21+abc",
            "1.0.0-0",
            "1.0.0-0a",
            "1.0.0-01a",
            "1.0.0+01",
        ] {
            let version = CoreVersion::parse(text)?;
            assert!(version >= CoreVersion::MINIMUM, "{text}");
            assert_eq!(version.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn version_table_below_floor() -> TestResult {
        for text in ["0.1.20", "0.0.9", "0.1.21-rc.1"] {
            assert!(CoreVersion::parse(text)? < CoreVersion::MINIMUM, "{text}");
        }
        Ok(())
    }

    #[test]
    fn version_table_invalid() {
        for text in [
            "v0.1.21",
            "0.1",
            "",
            "0.1.x",
            "0.1.21.4",
            "0.01.21",
            "0.1.21-",
            "0.1.21+",
            " 0.1.21",
            "18446744073709551616.0.0",
            "1.0.0-a..b",
            "1.0.0+a+b",
            "1.0.0-\u{e9}",
            "1.0.0+a b",
            "-1.0.0",
            "1..0",
            "1.0.0-01",
            "1.0.0-00",
            "1.0.0-alpha.01",
        ] {
            assert!(CoreVersion::parse(text).is_err(), "{text:?} parsed");
        }
    }

    #[test]
    fn invalid_version_names_the_text() {
        let error = CoreVersion::parse("0.1").err();
        assert_eq!(
            error.map(|e| e.to_string()).as_deref(),
            Some(r#""0.1" is not a MAJOR.MINOR.PATCH version"#)
        );
    }

    #[test]
    fn precedence_ignores_build_and_ranks_pre_releases() -> TestResult {
        let order = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in order.windows(2) {
            let (low, high) = (CoreVersion::parse(pair[0])?, CoreVersion::parse(pair[1])?);
            assert!(low < high, "{} < {}", pair[0], pair[1]);
        }
        assert_eq!(
            CoreVersion::parse("0.1.21+a")?,
            CoreVersion::parse("0.1.21+b")?
        );
        assert_eq!(CoreVersion::parse("0.1.21+a")?, CoreVersion::MINIMUM);
        assert!(CoreVersion::parse("1.0.0-9")? < CoreVersion::parse("1.0.0-a")?);
        assert!(CoreVersion::parse("1.0.0-9")? < CoreVersion::parse("1.0.0-10")?);
        assert!(
            CoreVersion::parse("1.0.0-a-b")?
                .cmp(&CoreVersion::parse("1.0.0-a-b")?)
                .is_eq()
        );
        Ok(())
    }

    #[test]
    fn hello_result_decodes_the_live_reply() -> TestResult {
        let wire = r#"{"core":{"id":"fdf328619a6eb255651d94a149cc7181","name":"hiraqs-macbook-air"},"features":["framed","attach","events","status","shell","resize_notifications","automatic_input","restart_environment"],"framed":false,"generation":1,"keeps_running_after_logout":null,"login_session":{"gui":true,"id":100002,"user":501},"logout_linger_user":null,"methods":["hello","server.status","server.stop","server.restart","terminal.run","templates.list","terminal.list","terminal.get","terminal.kill","terminal.rm","terminal.keep","terminal.label","terminal.read","terminal.attach","terminal.send","terminal.wait","terminal.keys","terminal.interrupt","terminal.put","machine.exec","file.read","file.write","file.edit","doc.get","doc.set","events.subscribe","machine.add","machine.list","machine.connect","machine.rename","machine.rm","rpc.discover"],"version":"0.1.21"}"#;
        let result: HelloResult = serde_json::from_str(wire)?;
        assert_eq!(result.version, "0.1.21");
        assert_eq!(result.generation, Generation::ONE);
        assert!(!result.framed);
        assert_eq!(
            result.core,
            CoreInfo {
                id: "fdf328619a6eb255651d94a149cc7181".to_owned(),
                name: "hiraqs-macbook-air".to_owned(),
            }
        );
        assert_eq!(result.features.len(), 8);
        assert_eq!(result.methods.len(), 32);
        assert_eq!(result.methods.first().map(String::as_str), Some("hello"));
        assert_eq!(result.keeps_running_after_logout, None);
        assert_eq!(result.logout_linger_user, None);
        assert_eq!(
            result.login_session,
            Some(LoginSession {
                id: 100_002,
                user: 501,
                gui: Some(true),
            })
        );
        assert_eq!(CoreVersion::parse(&result.version)?, CoreVersion::MINIMUM);
        Ok(())
    }

    #[test]
    fn hello_result_reads_absent_optionals_as_none() -> TestResult {
        let wire = r#"{"core":{"id":"a","name":"b"},"features":[],"framed":true,"generation":1,"methods":[],"version":"0.1.21"}"#;
        let result: HelloResult = serde_json::from_str(wire)?;
        assert_eq!(result.login_session, None);
        assert_eq!(
            serde_json::to_value(&result)?,
            serde_json::from_str::<serde_json::Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn hello_params_serialize_the_client_request() -> TestResult {
        let params = HelloParams::new().generation(Generation::ONE).framed();
        assert_eq!(
            serde_json::to_value(&params)?,
            json!({"generation": 1, "framed": true})
        );
        assert_eq!(serde_json::to_value(HelloParams::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn hello_params_serialize_every_setter() -> TestResult {
        let params = HelloParams::new()
            .calls_you("rohan")
            .core(CoreInfo {
                id: "abc".to_owned(),
                name: "host".to_owned(),
            })
            .features(["framed", "events"])
            .framed()
            .generation(Generation(1));
        assert_eq!(
            serde_json::to_value(&params)?,
            json!({
                "calls_you": "rohan",
                "core": {"id": "abc", "name": "host"},
                "features": ["framed", "events"],
                "framed": true,
                "generation": 1,
            })
        );
        Ok(())
    }

    #[test]
    fn hello_is_the_hello_method() {
        assert_eq!(<HelloParams as Method>::NAME, "hello");
    }
}
