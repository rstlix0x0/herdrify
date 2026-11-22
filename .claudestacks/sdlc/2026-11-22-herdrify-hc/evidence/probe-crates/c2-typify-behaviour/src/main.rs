//! C2 probe: runtime behaviour of the typify-generated types.
use c2_typify_compile::generated::*;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

fn try_parse<T: DeserializeOwned + Debug>(label: &str, json: &str) {
    match serde_json::from_str::<T>(json) {
        Ok(v) => println!("{label}: Ok({v:?})"),
        Err(e) => println!("{label}: Err({e})"),
    }
}

const TERM_MIN: &str = r#""address":"a","argv":[],"cwd":"/","state":"live","created_ms":1,"screen":"s","cols":80,"rows":24"#;

fn main() {
    // unknown fields
    try_parse::<CoreInfo>("1 CoreInfo + unknown field", r#"{"id":"i","name":"n","zzz":1}"#);
    // anyOf [X, null] and default: null
    try_parse::<Terminal>("2a Terminal status absent", &format!("{{{TERM_MIN},\"labels\":{{}}}}"));
    try_parse::<Terminal>("2b Terminal status null", &format!("{{{TERM_MIN},\"labels\":{{}},\"status\":null}}"));
    try_parse::<Terminal>("2c Terminal labels absent (required map)", &format!("{{{TERM_MIN}}}"));
    try_parse::<Terminal>("2d Terminal title null (type string, default \"\")", &format!("{{{TERM_MIN},\"labels\":{{}},\"title\":null}}"));
    // type: [integer, null]
    try_parse::<ExecResult>("3a ExecResult code null", r#"{"ms":1,"stderr":"","stdout":"","timed_out":false,"truncated":false,"code":null}"#);
    try_parse::<ExecResult>("3b ExecResult code absent", r#"{"ms":1,"stderr":"","stdout":"","timed_out":false,"truncated":false}"#);
    let e: ExecResult = serde_json::from_str(r#"{"ms":1,"stderr":"","stdout":"","timed_out":false,"truncated":false,"code":null}"#).unwrap();
    println!("3c ExecResult code None serializes as: {}", serde_json::to_string(&e).unwrap());
    // formats
    try_parse::<AttachResult>("4a AttachResult channel -1 (uint32)", r#"{"channel":-1,"automatic_input":true,"resize_notifications":true,"terminal":{}}"#);
    try_parse::<InitialSize>("4b InitialSize cols 0 (uint16, minimum 1)", r#"{"cols":0,"rows":1}"#);
    try_parse::<InitialSize>("4c InitialSize cols 65536", r#"{"cols":65536,"rows":1}"#);
    try_parse::<Status>("4d Status progress 300 (uint8)", r#"{"state":"x","progress":300}"#);
    try_parse::<DocSetResult>("4e DocSetResult version 18446744073709551615 (uint64 max)", r#"{"version":18446744073709551615}"#);
    // string enum
    try_parse::<Children>("5a Children \"all\"", r#""all""#);
    try_parse::<Children>("5b Children \"some\"", r#""some""#);
    // Display.cell fixed array
    try_parse::<Display>("6a Display cell [1,2,3]", r#"{"cell":[1,2,3]}"#);
}
