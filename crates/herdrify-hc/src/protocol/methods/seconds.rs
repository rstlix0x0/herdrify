//! `Seconds`: a timeout the core takes as a count of seconds.
//!
//! The core's timeouts are floating-point seconds, and a caller holds a `Duration`. A request type
//! stores the caller's `Duration` as `Seconds`, which writes it the way the core reads it. The type
//! is private because a caller only ever sees the `Duration`.
//!
//! Responsibilities:
//!
//! - [`Seconds`], a `Duration` that serializes as a floating-point count of seconds
//!
//! Non-responsibilities:
//!
//! - deciding which requests take a timeout, which the request types do
//! - reading a timeout back, because requests are only sent

use std::time::Duration;

use serde::{Serialize, Serializer};

/// A timeout that serializes as a floating-point count of seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Seconds(pub(super) Duration);

impl Serialize for Seconds {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::time::Duration;

    use super::Seconds;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn seconds_serializes_fractional_seconds() -> TestResult {
        assert_eq!(
            serde_json::to_string(&Seconds(Duration::from_millis(1500)))?,
            "1.5"
        );
        assert_eq!(
            serde_json::to_string(&Seconds(Duration::from_secs(180)))?,
            "180.0"
        );
        Ok(())
    }
}
