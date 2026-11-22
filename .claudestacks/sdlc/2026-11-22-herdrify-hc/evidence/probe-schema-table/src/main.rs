//! Tabulate the herdr-core OpenRPC document: per method its required/optional params and result;
//! per component schema its required/optional properties and enum values.
use serde_json::Value;
fn rf(v: &Value) -> String {
    if let Some(r) = v.get("$ref").and_then(Value::as_str) { return r.trim_start_matches("#/components/schemas/").to_string(); }
    if let Some(t) = v.get("type") { return t.to_string(); }
    if v.get("anyOf").is_some() { return format!("anyOf{}", v["anyOf"].as_array().map(|a| a.iter().map(rf).collect::<Vec<_>>().join("|")).unwrap_or_default()); }
    v.to_string()
}
fn main() {
    let path = std::env::args().nth(1).expect("schema path");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    let methods = doc["methods"].as_array().expect("methods");
    println!("methods: {}", methods.len());
    for m in methods {
        let (mut req, mut opt) = (vec![], vec![]);
        for p in m["params"].as_array().expect("params") {
            let n = p["name"].as_str().expect("name").to_string();
            if p["required"].as_bool() == Some(true) { req.push(n) } else { opt.push(n) }
        }
        let errs: Vec<String> = m.get("errors").and_then(Value::as_array).map(|a| a.iter().map(|e| e["data"]["kind"].as_str().unwrap_or("?").to_string()).collect()).unwrap_or_default();
        println!("M {} | required: [{}] | optional: [{}] | result: {} | errors: [{}]", m["name"].as_str().expect("n"), req.join(", "), opt.join(", "), rf(&m["result"]["schema"]), errs.join(", "));
    }
    let schemas = doc["components"]["schemas"].as_object().expect("schemas");
    println!("schemas: {}", schemas.len());
    for (name, s) in schemas {
        let required: Vec<&str> = s.get("required").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let shape = if let Some(props) = s.get("properties").and_then(Value::as_object) {
            let (r, o): (Vec<&String>, Vec<&String>) = props.keys().partition(|k| required.contains(&k.as_str()));
            format!("object | required: [{}] | optional: [{}]", r.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "), o.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
        } else if let Some(e) = s.get("enum") { format!("enum {e}") }
        else if let Some(o) = s.get("oneOf").or(s.get("anyOf")) { format!("union of {} variants: {}", o.as_array().map_or(0, Vec::len), serde_json::to_string(o).expect("s").chars().take(300).collect::<String>()) }
        else { format!("other: {}", s.to_string().chars().take(200).collect::<String>()) };
        println!("S {name} | {shape}");
    }
}
