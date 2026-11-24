//! The version of a herdr-app program, read from its output and ordered by semver precedence.
//!
//! [`AppVersion::parse`] reads the text `herdr-app --version` prints, and the ordering of the result
//! is semver 2.0 precedence, so a version can be compared with [`AppVersion::MINIMUM`], the oldest
//! release the layer supports. Nothing here runs the program.
//!
//! Responsibilities:
//!
//! - parsing `herdr-app MAJOR.MINOR.PATCH[-PRE][+BUILD]`, dropping the build part after `+` without
//!   checking it, and refusing any other text
//! - ordering versions by semver 2.0 precedence, which ignores build metadata
//! - naming the oldest supported version and the newest one proven against this layer
//!
//! Non-responsibilities:
//!
//! - running `herdr-app` to obtain the text, which a layer above does
//! - deciding what to do with a version that is too old, which the caller decides

use std::cmp::Ordering;
use std::fmt;

use thiserror::Error;

/// One dot-separated identifier of a pre-release suffix.
///
/// Numeric identifiers rank below alphanumeric ones, numeric ones by value and alphanumeric ones by
/// ASCII order, as semver 2.0 requires.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{AppVersion, PreIdentifier};
///
/// let version = AppVersion::parse("herdr-app 0.1.16-rc.1")?;
/// assert_eq!(
///     version.pre(),
///     [PreIdentifier::Alpha("rc".to_owned()), PreIdentifier::Numeric(1)],
/// );
/// # Ok::<(), herdrify_app::protocol::InvalidAppVersion>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PreIdentifier {
    // The variant order is the ranking: the derived `Ord` puts `Numeric` below `Alpha`.
    /// An identifier made only of digits, compared by value.
    Numeric(u64),
    /// An identifier holding a letter or a hyphen, compared in ASCII order.
    Alpha(String),
}

/// A herdr-app version, without build metadata.
///
/// Ordering is semver 2.0 precedence: the three numbers first, then a version with a pre-release
/// ranks below the same numbers without one.
///
/// A value only comes from [`AppVersion::parse`], [`AppVersion::MINIMUM`] or
/// [`AppVersion::PROVEN`], so printing one and parsing the text back gives an equal value.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppVersion;
///
/// let release = AppVersion::parse("herdr-app 0.1.16")?;
/// let candidate = AppVersion::parse("herdr-app 0.1.16-rc.1")?;
/// assert!(candidate < release);
/// assert!(release >= AppVersion::MINIMUM);
/// assert_eq!(release.to_string(), "0.1.16");
/// # Ok::<(), herdrify_app::protocol::InvalidAppVersion>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppVersion {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<PreIdentifier>,
}

/// The text is not `herdr-app MAJOR.MINOR.PATCH[-PRE]` followed by an optional `+` and build part.
///
/// The build part after the first `+` is dropped without being checked.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppVersion;
///
/// let error = AppVersion::parse("herdr-app 0.1").unwrap_err();
/// assert_eq!(
///     error.to_string(),
///     r#"cannot read a herdr-app version from "herdr-app 0.1""#,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("cannot read a herdr-app version from {text:?}")]
pub struct InvalidAppVersion {
    /// The text that was refused, as it was given.
    pub text: String,
}

impl AppVersion {
    /// The oldest herdr-app version the layer supports.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// assert_eq!(AppVersion::MINIMUM.to_string(), "0.1.16");
    /// ```
    pub const MINIMUM: Self = Self {
        major: 0,
        minor: 1,
        patch: 16,
        pre: Vec::new(),
    };

    /// The newest herdr-app version the layer has been proven against.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// assert!(AppVersion::PROVEN >= AppVersion::MINIMUM);
    /// assert_eq!(AppVersion::PROVEN.to_string(), "0.1.17");
    /// ```
    pub const PROVEN: Self = Self {
        major: 0,
        minor: 1,
        patch: 17,
        pre: Vec::new(),
    };

    /// The major number.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// assert_eq!(AppVersion::PROVEN.major(), 0);
    /// ```
    #[must_use]
    pub const fn major(&self) -> u64 {
        self.major
    }

    /// The minor number.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// assert_eq!(AppVersion::PROVEN.minor(), 1);
    /// ```
    #[must_use]
    pub const fn minor(&self) -> u64 {
        self.minor
    }

    /// The patch number.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// assert_eq!(AppVersion::PROVEN.patch(), 17);
    /// ```
    #[must_use]
    pub const fn patch(&self) -> u64 {
        self.patch
    }

    /// The pre-release identifiers after the first hyphen, empty for a release.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// assert!(AppVersion::PROVEN.pre().is_empty());
    /// let candidate = AppVersion::parse("herdr-app 0.1.16-rc.1")?;
    /// assert_eq!(candidate.pre().len(), 2);
    /// # Ok::<(), herdrify_app::protocol::InvalidAppVersion>(())
    /// ```
    #[must_use]
    pub fn pre(&self) -> &[PreIdentifier] {
        &self.pre
    }

    /// Reads the version from the text `herdr-app --version` prints.
    ///
    /// Surrounding whitespace is trimmed and the text must start with `herdr-app `. The rest is
    /// `MAJOR.MINOR.PATCH`, then an optional `-PRE` of dot-separated identifiers, then an optional
    /// build part: everything after the first `+` is dropped without being checked. Numbers have no
    /// leading zero and fit in a `u64`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidAppVersion`] holding `text` when it has any other shape.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::AppVersion;
    ///
    /// let version = AppVersion::parse("herdr-app 0.1.18+abc\n")?;
    /// assert_eq!(version.to_string(), "0.1.18");
    /// assert!(AppVersion::parse("0.1.18").is_err());
    /// # Ok::<(), herdrify_app::protocol::InvalidAppVersion>(())
    /// ```
    pub fn parse(text: &str) -> Result<Self, InvalidAppVersion> {
        read(text).ok_or_else(|| InvalidAppVersion {
            text: text.to_owned(),
        })
    }
}

/// Reads `text` as `parse` describes, or `None` when it has another shape.
fn read(text: &str) -> Option<AppVersion> {
    let rest = text.trim().strip_prefix("herdr-app ")?;
    let (rest, _build) = rest.split_once('+').unwrap_or((rest, ""));
    let (core, pre) = match rest.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (rest, None),
    };
    let mut numbers = core.split('.');
    let major = number(numbers.next()?)?;
    let minor = number(numbers.next()?)?;
    let patch = number(numbers.next()?)?;
    if numbers.next().is_some() {
        return None;
    }
    let pre = match pre {
        Some(pre) => pre.split('.').map(identifier).collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };
    Some(AppVersion {
        major,
        minor,
        patch,
        pre,
    })
}

/// Reads one of the three version numbers: ASCII digits, no leading zero unless it is `0`.
fn number(text: &str) -> Option<u64> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if !digits || (text.len() > 1 && text.starts_with('0')) {
        return None;
    }
    text.parse().ok()
}

/// Reads one pre-release identifier: non-empty `[0-9A-Za-z-]`, with all-digit ones read as numbers.
fn identifier(text: &str) -> Option<PreIdentifier> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return None;
    }
    if text.bytes().all(|b| b.is_ascii_digit()) {
        number(text).map(PreIdentifier::Numeric)
    } else {
        Some(PreIdentifier::Alpha(text.to_owned()))
    }
}

impl Ord for AppVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let numbers =
            (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        numbers.then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.pre.cmp(&other.pre),
        })
    }
}

impl PartialOrd for AppVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (index, identifier) in self.pre.iter().enumerate() {
            f.write_str(if index == 0 { "-" } else { "." })?;
            match identifier {
                PreIdentifier::Numeric(n) => write!(f, "{n}")?,
                PreIdentifier::Alpha(text) => f.write_str(text)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Result<AppVersion, InvalidAppVersion> {
        AppVersion::parse(&format!("herdr-app {text}"))
    }

    #[test]
    fn parses_the_installed_output() -> Result<(), InvalidAppVersion> {
        let version = AppVersion::parse("herdr-app 0.1.17\n")?;
        assert_eq!(
            (version.major(), version.minor(), version.patch()),
            (0, 1, 17)
        );
        assert!(version.pre().is_empty());
        assert_eq!(version, AppVersion::PROVEN);
        assert_eq!(version.to_string(), "0.1.17");
        let padded = AppVersion::parse("  herdr-app 0.1.17\r\n")?;
        assert_eq!(padded, AppVersion::PROVEN);
        Ok(())
    }

    #[test]
    fn drops_build_metadata() -> Result<(), InvalidAppVersion> {
        let version = AppVersion::parse("herdr-app 0.1.18+abc")?;
        assert_eq!(version.to_string(), "0.1.18");
        assert_eq!(version, v("0.1.18")?);
        assert_eq!(v("0.1.18-rc.1+b.7")?.to_string(), "0.1.18-rc.1");
        let hyphenated = AppVersion::parse("herdr-app 0.1.16+build-1")?;
        assert_eq!(hyphenated, v("0.1.16")?);
        assert!(hyphenated.pre().is_empty());
        Ok(())
    }

    #[test]
    fn printed_version_parses_back_equal() -> Result<(), InvalidAppVersion> {
        for text in ["0.1.16", "0.1.16-rc.1", "1.2.3-alpha.10.beta"] {
            let version = v(text)?;
            assert_eq!(AppVersion::parse(&format!("herdr-app {version}"))?, version);
        }
        Ok(())
    }

    #[test]
    fn exposes_the_parsed_parts() -> Result<(), InvalidAppVersion> {
        let version = v("1.2.3-alpha.10")?;
        assert_eq!(
            (version.major(), version.minor(), version.patch()),
            (1, 2, 3)
        );
        assert_eq!(
            version.pre(),
            [
                PreIdentifier::Alpha("alpha".to_owned()),
                PreIdentifier::Numeric(10)
            ]
        );
        Ok(())
    }

    #[test]
    fn pre_release_ranks_below_release() -> Result<(), InvalidAppVersion> {
        assert!(v("0.1.16-rc.1")? < AppVersion::MINIMUM);
        let ladder = [
            v("0.1.16-alpha")?,
            v("0.1.16-alpha.1")?,
            v("0.1.16-beta")?,
            v("0.1.16-rc.1")?,
            v("0.1.16")?,
        ];
        for pair in ladder.windows(2) {
            assert!(pair[0] < pair[1], "{} < {}", pair[0], pair[1]);
        }
        assert!(v("0.1.16-1")? < v("0.1.16-a")?);
        assert!(v("0.1.16-2")? < v("0.1.16-10")?);
        assert!(v("0.1.16-a-b")? > v("0.1.16-a")?);
        assert_eq!(v("0.1.16-rc.1")?.to_string(), "0.1.16-rc.1");
        Ok(())
    }

    #[test]
    fn refuses_malformed_text() -> Result<(), InvalidAppVersion> {
        for text in [
            "",
            "herdr-app 01.1.16",
            "herdr-app 0.1",
            "x 0.1.16",
            "herdr-app 0.1.16-",
            "herdr-app 0.1.16.4",
            "herdr-app 0.1.16-a..b",
            "herdr-app 0.1.16-01",
            "herdr-app 0.1.16-a_b",
            "herdr-app 0.1.x",
            "herdr-app 99999999999999999999.1.16",
            "herdr-app0.1.16",
        ] {
            let error = AppVersion::parse(text).err();
            assert_eq!(
                error,
                Some(InvalidAppVersion {
                    text: text.to_owned()
                }),
                "{text:?}"
            );
        }
        let error = AppVersion::parse("herdr-app 0.1").err();
        assert_eq!(
            error.map(|e| e.to_string()),
            Some(r#"cannot read a herdr-app version from "herdr-app 0.1""#.to_owned())
        );
        assert_eq!(v("0.0.0")?.to_string(), "0.0.0");
        Ok(())
    }

    #[test]
    fn orders_against_minimum() -> Result<(), InvalidAppVersion> {
        assert!(v("0.1.15")? < AppVersion::MINIMUM);
        assert!(AppVersion::MINIMUM <= v("0.1.16")?);
        assert!(v("0.1.16")? < v("0.2.0")?);
        assert!(v("0.1.9")? < v("0.1.10")?);
        assert!(v("1.0.0")? > v("0.99.99")?);
        Ok(())
    }
}
