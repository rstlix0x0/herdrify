//! Counts facts from the saved `hc schema` and `herdr-app commands` outputs.
//!
//! Usage: `cargo run --bin facts -- <hc-schema.json> <herdr-app-commands.json>`

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;

use serde_json::Value;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let schema_path = args.next().ok_or("missing hc schema path")?;
    let commands_path = args.next().ok_or("missing herdr-app commands path")?;

    let schema: Value = serde_json::from_str(&fs::read_to_string(&schema_path)?)?;
    println!("== hc schema ({schema_path})");
    println!("openrpc: {}", schema["openrpc"]);
    println!("info.title: {}", schema["info"]["title"]);
    println!("info.version: {}", schema["info"]["version"]);
    println!("info.description: {}", schema["info"]["description"]);
    let methods = schema["methods"].as_array().ok_or("methods not an array")?;
    println!("methods: {}", methods.len());
    for m in methods {
        let params: Vec<&str> = m["params"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["name"].as_str())
            .collect();
        println!(
            "  method {} ({}) | {}",
            m["name"].as_str().unwrap_or("?"),
            params.join(", "),
            m["summary"]
                .as_str()
                .or_else(|| m["description"].as_str())
                .unwrap_or("")
        );
    }
    let schemas = schema["components"]["schemas"]
        .as_object()
        .ok_or("components.schemas not an object")?;
    println!("components.schemas: {}", schemas.len());

    let commands: Value = serde_json::from_str(&fs::read_to_string(&commands_path)?)?;
    let commands = commands.as_array().ok_or("commands not an array")?;
    println!("== herdr-app commands ({commands_path})");
    println!("commands: {}", commands.len());
    let mut command_keys = BTreeSet::new();
    let mut arg_keys = BTreeSet::new();
    for c in commands {
        if let Some(obj) = c.as_object() {
            command_keys.extend(obj.keys().cloned());
        }
        for a in c["args"].as_array().into_iter().flatten() {
            if let Some(obj) = a.as_object() {
                arg_keys.extend(obj.keys().cloned());
            }
        }
    }
    println!("keys per command: {command_keys:?}");
    println!("keys per arg: {arg_keys:?}");
    let with_argv: Vec<&str> = commands
        .iter()
        .filter(|c| {
            c["args"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|a| a["name"].as_str() == Some("argv"))
        })
        .filter_map(|c| c["name"].as_str())
        .collect();
    println!("commands with an `argv` arg: {} {with_argv:?}", with_argv.len());
    Ok(())
}
