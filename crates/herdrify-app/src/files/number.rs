//! The number a herdr-app file holds, kept in the form it was written.
//!
//! A plain `f64` would write `268` back as `268.0`, and a bare integer type would refuse `4.286`.
//! [`Number`] keeps whether a value was written as an integer or a fraction, so a file read and
//! written again says what it said before. The wrapped `serde_json::Number` stays private so the
//! public API does not depend on it.
//!
//! Responsibilities:
//!
//! - [`Number`], built from an integer or a finite `f64` and read and written as a plain JSON
//!   number
//! - [`NumberError`], the refusal of a value JSON cannot hold
//!
//! Non-responsibilities:
//!
//! - deciding which numbers a field accepts, such as a range, which the file shapes do
//! - preserving the spelling of an exponent: `1e3` is written back as `1000.0`, which is the same
//!   number

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The refusal of a value that is not a finite number.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Number, NumberError};
///
/// let refused: NumberError = Number::try_from(f64::NAN).unwrap_err();
/// assert_eq!(refused.to_string(), "NaN is not a finite number");
/// ```
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{value} is not a finite number")]
pub struct NumberError {
    /// The value that was refused: NaN or an infinity.
    pub value: f64,
}

/// A JSON number that is written back the way it was read.
///
/// An integer stays an integer and a fraction keeps its decimal point, so `268` is written as
/// `268` and `120.0` as `120.0`. An exponent is written out, so `1e3` becomes `1000.0`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Number;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let width: Number = serde_json::from_str("268")?;
/// let scale: Number = serde_json::from_str("120.0")?;
/// assert_eq!(serde_json::to_string(&width)?, "268");
/// assert_eq!(serde_json::to_string(&scale)?, "120.0");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Number(serde_json::Number);

impl From<u32> for Number {
    fn from(value: u32) -> Self {
        Self(value.into())
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self(value.into())
    }
}

impl From<u64> for Number {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl TryFrom<f64> for Number {
    type Error = NumberError;

    /// Wraps `value` as a number written with a decimal point.
    ///
    /// # Errors
    ///
    /// Returns [`NumberError`] when `value` is NaN or an infinity, which JSON cannot hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Number, NumberError};
    ///
    /// # fn main() -> Result<(), NumberError> {
    /// let ratio = Number::try_from(4.286)?;
    /// assert!(!ratio.is_integer());
    /// assert!(Number::try_from(f64::INFINITY).is_err());
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        serde_json::Number::from_f64(value)
            .map(Self)
            .ok_or(NumberError { value })
    }
}

impl Number {
    /// The value as an `f64`.
    ///
    /// An integer beyond 2^53 in magnitude is rounded to the nearest `f64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Number, NumberError};
    ///
    /// # fn main() -> Result<(), NumberError> {
    /// assert_eq!(Number::from(268_u32).as_f64(), 268.0);
    /// assert_eq!(Number::try_from(4.5)?.as_f64(), 4.5);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        // Without serde_json's `arbitrary_precision` feature, which this crate does not enable,
        // every number converts, so the fallback is never taken.
        self.0.as_f64().unwrap_or(f64::NAN)
    }

    /// Whether the value was written as an integer, without a decimal point.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Number, NumberError};
    ///
    /// # fn main() -> Result<(), NumberError> {
    /// assert!(Number::from(268_u32).is_integer());
    /// assert!(!Number::try_from(120.0)?.is_integer());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.0.is_i64() || self.0.is_u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads `text` as a [`Number`] and writes it back.
    fn round_trip(text: &str) -> Result<String, serde_json::Error> {
        let number: Number = serde_json::from_str(text)?;
        serde_json::to_string(&number)
    }

    #[test]
    fn number_keeps_the_written_form() -> Result<(), serde_json::Error> {
        for text in ["120.0", "268", "4.286", "0.0", "-5", "-0.0"] {
            assert_eq!(round_trip(text)?, text);
        }
        Ok(())
    }

    #[test]
    fn number_writes_exponents_as_floats() -> Result<(), serde_json::Error> {
        assert_eq!(round_trip("1e3")?, "1000.0");
        Ok(())
    }

    #[test]
    fn number_refuses_non_finite() -> Result<(), String> {
        let nan = Number::try_from(f64::NAN).err().ok_or("NaN was accepted")?;
        assert_eq!(nan.to_string(), "NaN is not a finite number");
        assert!(Number::try_from(f64::INFINITY).is_err());
        assert!(Number::try_from(f64::NEG_INFINITY).is_err());
        Ok(())
    }

    #[test]
    fn number_reports_integer_form() -> Result<(), NumberError> {
        assert!(Number::from(268u32).is_integer());
        assert!(Number::from(-5i64).is_integer());
        assert!(Number::from(u64::MAX).is_integer());
        assert!(!Number::try_from(4.5)?.is_integer());
        assert!(!Number::try_from(120.0)?.is_integer());
        Ok(())
    }

    #[test]
    fn number_reads_as_f64() -> Result<(), NumberError> {
        assert!((Number::from(268u32).as_f64() - 268.0).abs() < f64::EPSILON);
        assert!((Number::try_from(4.286)?.as_f64() - 4.286).abs() < f64::EPSILON);
        Ok(())
    }
}
