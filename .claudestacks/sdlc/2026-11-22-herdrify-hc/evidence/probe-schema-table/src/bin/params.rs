//! Every method param with its JSON Schema type and the start of its description.
use serde_json::Value;
fn ty(v: &Value) -> String {
    if let Some(r) = v.get("$ref").and_then(Value::as_str) { return r.trim_start_matches("#/components/schemas/").to_string(); }
    if let Some(a) = v.get("anyOf").and_then(Value::as_array) { return a.iter().map(ty).collect::<Vec<_>>().join("|"); }
    let mut t = match v.get("type") { Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("|"), Some(Value::String(s)) => s.clone(), _ => "any".into() };
    if let Some(f) = v.get("format").and_then(Value::as_str) { t = format!("{t}<{f}>"); }
    if let Some(i) = v.get("items") { t = format!("{t}[{}]", ty(i)); }
    if let Some(ap) = v.get("additionalProperties") { if ap.is_object() { t = format!("{t}{{str:{}}}", ty(ap)); } }
    if let Some(m) = v.get("minimum") { t = format!("{t} min={m}"); }
    if let Some(e) = v.get("enum") { t = format!("{t} enum={e}"); }
    if let Some(d) = v.get("default") { t = format!("{t} default={d}"); }
    t
}
fn main() {
    let path = std::env::args().nth(1).expect("schema path");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    for m in doc["methods"].as_array().expect("methods") {
        for p in m["params"].as_array().expect("params") {
            let d: String = p.get("description").and_then(Value::as_str).unwrap_or("").replace('\n', " ").chars().take(110).collect();
            println!("{} . {}{} : {} -- {}", m["name"].as_str().expect("n"), p["name"].as_str().expect("n"), if p["required"].as_bool() == Some(true) { "*" } else { "" }, ty(&p["schema"]), d);
        }
    }
}
