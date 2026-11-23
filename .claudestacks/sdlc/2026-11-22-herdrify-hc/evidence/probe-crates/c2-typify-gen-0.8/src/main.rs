//! C2 probe: feed herdr-core OpenRPC `components.schemas` to typify.
//!
//! usage: c2-typify-gen <openrpc.json> <mode> [out.rs]
//!   mode = components   : the 36 components.schemas only, refs rewritten
//!   mode = whole-doc    : the entire OpenRPC document as a root schema (no rewriting)
//!   mode = with-params  : components + one object schema per method assembled from `params`
use std::{env, fs, process::ExitCode};

use schemars::schema::RootSchema;
use serde_json::{Map, Value, json};
use typify::{TypeSpace, TypeSpaceSettings};

const FROM: &str = "\"#/components/schemas/";
const TO: &str = "\"#/definitions/";

fn rewrite(v: &Value) -> Value {
    let s = serde_json::to_string(v).unwrap();
    let n = s.matches(FROM).count();
    eprintln!("ref rewrite: replaced {n} occurrences of {FROM} with {TO}");
    serde_json::from_str(&s.replace(FROM, TO)).unwrap()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let doc: Value = serde_json::from_str(&fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mode = args[2].as_str();
    let root_json = match mode {
        "components" => {
            let schemas = &doc["components"]["schemas"];
            eprintln!("components.schemas count: {}", schemas.as_object().unwrap().len());
            json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "definitions": rewrite(schemas),
            })
        }
        "whole-doc" => doc.clone(),
        "with-params" => {
            let mut defs = rewrite(&doc["components"]["schemas"]).as_object().unwrap().clone();
            for m in doc["methods"].as_array().unwrap() {
                let name = m["name"].as_str().unwrap();
                let mut props = Map::new();
                let mut required = vec![];
                for p in m["params"].as_array().unwrap() {
                    let pn = p["name"].as_str().unwrap().to_owned();
                    props.insert(pn.clone(), rewrite(&p["schema"]));
                    if p["required"].as_bool() == Some(true) {
                        required.push(Value::String(pn));
                    }
                }
                let ty = format!("{}Params", name.replace(['.', '_'], " ")
                    .split(' ').map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }).collect::<String>());
                defs.insert(ty, json!({"type":"object","properties":props,"required":required}));
            }
            eprintln!("definitions after assembling params: {}", defs.len());
            json!({"$schema": "http://json-schema.org/draft-07/schema#", "definitions": defs})
        }
        other => panic!("unknown mode {other}"),
    };
    let root: RootSchema = match serde_json::from_value(root_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("schemars RootSchema parse error: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("RootSchema.definitions: {}", root.definitions.len());
    let mut ts = TypeSpace::new(TypeSpaceSettings::default().with_struct_builder(false));
    if let Err(e) = ts.add_root_schema(root) {
        eprintln!("typify add_root_schema error: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("typify generated type count: {}", ts.iter_types().count());
    let code = ts.to_stream().to_string();
    if let Some(out) = args.get(3) {
        fs::write(out, code).unwrap();
        eprintln!("wrote {out} (unformatted token stream; run rustfmt on it)");
    }
    ExitCode::SUCCESS
}
