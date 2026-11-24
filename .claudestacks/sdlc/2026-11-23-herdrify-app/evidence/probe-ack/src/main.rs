//! Probe: serde 1.0.229 / serde_json 1.0.154. The exact error text of an `Ack` that decodes a
//! JSON object and refuses one whose `ok` is not `true`, and the key order of a compact JSON
//! object built from a struct through `serde_json::Map`.
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

#[derive(Debug)]
struct Ack;

impl<'de> Deserialize<'de> for Ack {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let m = Map::<String, Value>::deserialize(d)?;
        if m.get("ok") == Some(&Value::Bool(true)) {
            Ok(Self)
        } else {
            Err(D::Error::custom("expected \"ok\": true"))
        }
    }
}

#[derive(Serialize)]
struct TabNew {
    name: String,
    argv: Vec<String>,
    focus: bool,
}

fn main() {
    for t in [r#"{"ok":true}"#, r#"{"ok":true,"x":1}"#, r#"{"ok":false}"#, "{}", "7"] {
        match serde_json::from_slice::<Ack>(t.as_bytes()) {
            Ok(a) => println!("A1 {t} -> Ok({a:?})"),
            Err(e) => println!("A1 {t} -> Err({e})"),
        }
    }
    let cmd = TabNew { name: "api".into(), argv: vec!["claude".into()], focus: false };
    println!("A2 to_string(struct)       = {}", serde_json::to_string(&cmd).unwrap());
    let v = serde_json::to_value(&cmd).unwrap();
    println!("A2 to_string(to_value(..)) = {}", serde_json::to_string(&v).unwrap());
}
