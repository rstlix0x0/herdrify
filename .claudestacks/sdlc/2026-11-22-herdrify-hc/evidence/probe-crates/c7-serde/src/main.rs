//! C7 probe: serde / serde_json behaviour for JSON-RPC message types.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
struct Plain {
    id: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Strict {
    id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct WithExtra {
    id: u32,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Opt {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OptNoDefault {
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OptSkip {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    jsonrpc: String,
    id: Value,
    method: String,
}

#[derive(Debug, Deserialize)]
struct Response<'a> {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    #[serde(borrow, default)]
    result: Option<&'a RawValue>,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ResponseOwned {
    id: Value,
    result: Option<Box<RawValue>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Incoming {
    Response { id: Value, result: Box<RawValue> },
    Error { id: Value, error: Value },
    Notification { method: String, params: Option<Box<RawValue>> },
}

#[derive(Debug, Deserialize)]
struct ResponseRequiredRaw {
    result: Box<RawValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IncomingValue {
    Response { id: Value, result: Value },
    Error { id: Value, error: Value },
    Notification { method: String, params: Option<Value> },
}

#[derive(Debug, Deserialize)]
struct Hello {
    version: String,
}

fn show<T: std::fmt::Debug>(label: &str, r: Result<T, serde_json::Error>) {
    match r {
        Ok(v) => println!("{label}: Ok({v:?})"),
        Err(e) => println!("{label}: Err(\"{e}\") classify={:?} line={} column={}", e.classify(), e.line(), e.column()),
    }
}

fn main() {
    println!("--- (a) unknown field");
    show("a1 Plain {id:1, zzz:2}", serde_json::from_str::<Plain>(r#"{"id":1,"zzz":2}"#));
    show("a2 Strict(deny_unknown_fields) {id:1, zzz:2}", serde_json::from_str::<Strict>(r#"{"id":1,"zzz":2}"#));

    println!("--- (b) flatten extra Map captures unknown fields");
    let w: WithExtra = serde_json::from_str(r#"{"id":1,"zzz":2,"nested":{"k":[1,null]},"s":"x"}"#).unwrap();
    println!("b1 parsed: {w:?}");
    let back = serde_json::to_string(&w).unwrap();
    println!("b2 serialized back: {back}");
    show("b3 WithExtra id \"1\" (string)", serde_json::from_str::<WithExtra>(r#"{"id":"1"}"#));
    show("b4 WithExtra id -1 (u32 under flatten)", serde_json::from_str::<WithExtra>(r#"{"id":-1}"#));

    println!("--- (c) u32 out of range");
    show("c1 u32 -1", serde_json::from_str::<Plain>(r#"{"id":-1}"#));
    show("c2 u32 4294967296", serde_json::from_str::<Plain>(r#"{"id":4294967296}"#));
    show("c3 u32 1.0", serde_json::from_str::<Plain>(r#"{"id":1.0}"#));
    show("c4 u32 4294967295", serde_json::from_str::<Plain>(r#"{"id":4294967295}"#));

    println!("--- (d) Option<String>: null vs absent; skip_serializing_if");
    show("d1 Opt(default) null", serde_json::from_str::<Opt>(r#"{"name":null}"#));
    show("d2 Opt(default) absent", serde_json::from_str::<Opt>(r#"{}"#));
    show("d3 OptNoDefault null", serde_json::from_str::<OptNoDefault>(r#"{"name":null}"#));
    show("d4 OptNoDefault absent", serde_json::from_str::<OptNoDefault>(r#"{}"#));
    println!("d5 Opt(default) None serializes: {}", serde_json::to_string(&Opt { name: None }).unwrap());
    println!("d6 OptSkip None serializes: {}", serde_json::to_string(&OptSkip { name: None }).unwrap());
    println!("d7 OptSkip Some serializes: {}", serde_json::to_string(&OptSkip { name: Some("x".into()) }).unwrap());

    println!("--- (e) serde_json::Value as JSON-RPC id");
    for raw in [r#"1"#, r#""abc""#, r#"null"#, r#"18446744073709551615"#, r#"-7"#, r#"1.5"#, r#""1""#] {
        let s = format!(r#"{{"jsonrpc":"2.0","id":{raw},"method":"hello"}}"#);
        let r: Request = serde_json::from_str(&s).unwrap();
        let back = serde_json::to_string(&r).unwrap();
        println!(
            "e id {raw:>22} -> {:?} is_u64={} is_string={} roundtrip={} equal_to_Value(1)={}",
            r.id,
            r.id.is_u64(),
            r.id.is_string(),
            back,
            r.id == Value::from(1u64)
        );
    }
    println!("e id 1 vs \"1\" equal? {}", Value::from(1) == Value::from("1"));

    println!("--- (f) RawValue keeps result unparsed");
    let text = r#"{"jsonrpc":"2.0","id":7,"result":{"version":"0.1.21",  "extra":[1,2]}}"#;
    let resp: Response = serde_json::from_str(text).unwrap();
    println!("f1 id={:?} raw result text={:?} error={:?}", resp.id, resp.result.map(RawValue::get), resp.error);
    let hello: Hello = serde_json::from_str(resp.result.unwrap().get()).unwrap();
    println!("f2 later typed parse of raw result: {hello:?}");
    let owned: ResponseOwned = serde_json::from_str(text).unwrap();
    println!("f3 Box<RawValue> owned: id={:?} result={:?}", owned.id, owned.result.as_deref().map(RawValue::get));
    let null_result: ResponseOwned = serde_json::from_str(r#"{"jsonrpc":"2.0","id":8,"result":null}"#).unwrap();
    println!("f4 result:null with Option<Box<RawValue>>: {:?}", null_result.result.as_deref().map(RawValue::get));
    show("f5 untagged Incoming response", serde_json::from_str::<Incoming>(text));
    show("f6 untagged Incoming error", serde_json::from_str::<Incoming>(r#"{"jsonrpc":"2.0","id":9,"error":{"code":-32601,"message":"Method not found"}}"#));
    show("f7 untagged Incoming notification", serde_json::from_str::<Incoming>(r#"{"jsonrpc":"2.0","method":"event","params":{"seq":1}}"#));
    show("f8 untagged Incoming garbage", serde_json::from_str::<Incoming>(r#"{"jsonrpc":"2.0"}"#));
    let v: Value = serde_json::from_str(text).unwrap();
    let raw_from_value = serde_json::value::to_raw_value(&v["result"]).unwrap();
    println!("f9 to_raw_value from Value (re-serialized, whitespace not kept): {}", raw_from_value.get());
    let req_null: ResponseRequiredRaw = serde_json::from_str(r#"{"jsonrpc":"2.0","id":8,"result":null}"#).unwrap();
    println!("f10 result:null with non-Option Box<RawValue>: {:?}", req_null.result.get());
    show("f11 untagged IncomingValue (Value fields) response", serde_json::from_str::<IncomingValue>(text));
    show("f12 untagged IncomingValue notification", serde_json::from_str::<IncomingValue>(r#"{"jsonrpc":"2.0","method":"event","params":{"seq":1}}"#));
}
