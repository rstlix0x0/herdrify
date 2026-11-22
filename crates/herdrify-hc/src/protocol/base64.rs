//! Serde support for byte fields the core carries as base64 text.
//!
//! The core sends and takes process output and file data as base64 strings. A message type that
//! holds the bytes names this module's `serialize` in `#[serde(serialize_with = ...)]` for a request,
//! or the whole module in `#[serde(with = ...)]` for a reply, and a request type may call
//! `serialize` directly from its own `Serialize` impl. A caller sees `Vec<u8>` and never a base64
//! type. The module is private for that reason.
//!
//! Responsibilities:
//!
//! - writing bytes as a string in the standard padded base64 alphabet
//! - reading such a string back, refusing text that is not valid base64
//!
//! Non-responsibilities:
//!
//! - deciding which fields are base64, which the message types do

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Deserializer, Serializer};

/// Writes `bytes` as standard padded base64 text.
pub(super) fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}

/// Reads standard padded base64 text into bytes.
pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let text = String::deserialize(deserializer)?;
    STANDARD.decode(text).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde::{Deserialize, Serialize};

    type TestResult = Result<(), Box<dyn Error>>;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Carrier {
        #[serde(with = "super")]
        bytes: Vec<u8>,
    }

    #[test]
    fn base64_encodes_with_padding() -> TestResult {
        let carrier = Carrier {
            bytes: b"x\n".to_vec(),
        };
        assert_eq!(serde_json::to_string(&carrier)?, r#"{"bytes":"eAo="}"#);
        Ok(())
    }

    #[test]
    fn base64_decodes_what_it_encodes() -> TestResult {
        let carrier: Carrier = serde_json::from_str(r#"{"bytes":"eAo="}"#)?;
        assert_eq!(carrier.bytes, b"x\n");
        let empty: Carrier = serde_json::from_str(r#"{"bytes":""}"#)?;
        assert!(empty.bytes.is_empty());
        Ok(())
    }

    #[test]
    fn base64_refuses_invalid_input() {
        let refused = serde_json::from_str::<Carrier>(r#"{"bytes":"not base64!"}"#);
        assert!(refused.is_err());
    }
}
