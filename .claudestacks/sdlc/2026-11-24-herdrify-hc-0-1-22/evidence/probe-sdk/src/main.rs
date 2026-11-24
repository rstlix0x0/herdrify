use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

use base64::Engine as _;
use herdrify_hc::protocol::CoreVersion;
use herdrify_hc::{Error, HerdrCoreError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct EmptyResult {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. the docs guard reuses UnsupportedVersion: its exact text.
    let e = Error::UnsupportedVersion {
        found: CoreVersion::parse("0.1.21")?,
        minimum: CoreVersion::parse("0.1.22")?,
    };
    println!("[1] Error: {e}");
    let wrapped: HerdrCoreError = HerdrCoreError::from(e);
    println!("[1] HerdrCoreError: {wrapped}");
    println!("[1] 0.1.21 < 0.1.22: {}", CoreVersion::parse("0.1.21")? < CoreVersion::parse("0.1.22")?);
    println!("[1] 0.1.22+abc >= 0.1.22: {}", CoreVersion::parse("0.1.22+abc")? >= CoreVersion::parse("0.1.22")?);
    println!("[1] 0.1.22-rc.1 < 0.1.22: {}", CoreVersion::parse("0.1.22-rc.1")? < CoreVersion::parse("0.1.22")?);
    // 2. std RandomState as an id source: two fresh states, 16 hex digits each.
    let a = RandomState::new().hash_one(0u8);
    let b = RandomState::new().hash_one(0u8);
    let c = RandomState::new().hash_one(0u8);
    println!("[2] {a:016x}{b:016x} then {c:016x}… distinct: {}", a != b && b != c && a != c);
    // 3. serde: an empty struct reads `{}` and writes `{}`; extra fields are ignored.
    let v: EmptyResult = serde_json::from_str("{}")?;
    println!("[3] from {{}} -> {:?}, back -> {}", v, serde_json::to_string(&v)?);
    let extra: EmptyResult = serde_json::from_str(r#"{"later":1}"#)?;
    println!("[3] from {{\"later\":1}} -> {extra:?}");
    println!("[3] from null -> {:?}", serde_json::from_str::<EmptyResult>("null").map_err(|e| e.to_string()));
    // 5. an empty doc name, as the HerdrCore layer reports it.
    let empty = herdrify_hc::protocol::DocName::new("").map_err(HerdrCoreError::from);
    println!("[5] DocName::new(\"\") -> {:?} / {}", empty.as_ref().err(), empty.as_ref().err().map(ToString::to_string).unwrap_or_default());
    // 4. size of one 1 MiB piece as a request, against the 16 MiB frame limit.
    let data = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 1024 * 1024]);
    let req = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"terminal.put_chunk",
        "params":{"transfer":"f".repeat(64),"offset":u64::MAX,"data":data}});
    println!("[4] 1 MiB piece request: {} bytes; limit 16777216", serde_json::to_vec(&req)?.len());
    Ok(())
}
