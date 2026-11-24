//! Builds a fully populated JSON instance of any schema in the `OpenRPC` fixture.
//!
//! Every optional property is set, so a type that drops or renames a property cannot round-trip
//! the instance. The values are the simplest ones the schema allows: `"x"` for text, the first
//! word of an enum, and the smallest integer the schema accepts that is at least 1.
//!
//! Two tables override those values. `BYTE_PROPERTIES` lists the text the SDK reads as base64
//! bytes. `STRING_OVERRIDES` lists the values the SDK validates, such as a transfer id, where `"x"`
//! would be refused: each entry names the schema, the property and the text to use instead.

use serde_json::{Map, Value, json};

/// Text that is valid base64, for the byte fields the SDK reads as bytes.
const BASE64_BYTES: &str = "eAo=";

/// The properties whose schema says `string` but whose value the SDK reads as base64 bytes.
const BYTE_PROPERTIES: [(&str, &str); 2] = [("ExecResult", "stdout"), ("ExecResult", "stderr")];

/// The properties whose value the SDK validates, as `(schema, property, value)`. An entry replaces
/// the built value of that property whatever type its schema gives.
pub const STRING_OVERRIDES: [(&str, &str, &str); 1] =
    [("PutBeginResult", "transfer", "0123456789abcdef")];

/// Builds the instance of the component schema called `schema_name`.
///
/// # Errors
///
/// Returns the reason when the name is not in the fixture, when a schema holds a `$ref` that does
/// not resolve, or when an `anyOf` has no branch other than `null`.
pub fn instance(fixture: &Value, schema_name: &str) -> Result<Value, String> {
    let mut building = Vec::new();
    build_named(fixture, schema_name, &mut building)
}

fn build_named(fixture: &Value, name: &str, building: &mut Vec<String>) -> Result<Value, String> {
    let schema = fixture
        .pointer(&format!("/components/schemas/{name}"))
        .ok_or_else(|| format!("no schema named {name}"))?;
    building.push(name.to_owned());
    let built = build(fixture, schema, building);
    building.pop();
    built
}

fn build(fixture: &Value, schema: &Value, building: &mut Vec<String>) -> Result<Value, String> {
    let Some(schema) = schema.as_object() else {
        return Ok(json!("x"));
    };
    if let Some(target) = schema.get("$ref").and_then(Value::as_str) {
        let name = ref_name(target)?;
        return build_named(fixture, name, building);
    }
    if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
        let branch = branches
            .iter()
            .find(|branch| branch.get("type") != Some(&json!("null")))
            .ok_or("anyOf has no branch other than null")?;
        return build(fixture, branch, building);
    }
    if let Some(first) = schema
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|words| words.first())
    {
        return Ok(first.clone());
    }
    match non_null_type(schema) {
        Some("object") => build_object(fixture, schema, building),
        Some("array") => build_array(fixture, schema, building),
        Some("integer") => Ok(json!(integer(schema))),
        Some("number") => Ok(json!(1.0)),
        Some("boolean") => Ok(json!(true)),
        _ => Ok(json!("x")),
    }
}

fn ref_name(target: &str) -> Result<&str, String> {
    target
        .strip_prefix("#/components/schemas/")
        .ok_or_else(|| format!("cannot resolve the reference {target}"))
}

fn non_null_type(schema: &Map<String, Value>) -> Option<&str> {
    match schema.get("type")? {
        Value::String(word) => Some(word),
        Value::Array(words) => words
            .iter()
            .filter_map(Value::as_str)
            .find(|word| *word != "null"),
        _ => None,
    }
}

fn integer(schema: &Map<String, Value>) -> i64 {
    schema
        .get("minimum")
        .and_then(Value::as_i64)
        .filter(|minimum| *minimum >= 1)
        .unwrap_or(1)
}

fn build_object(
    fixture: &Value,
    schema: &Map<String, Value>,
    building: &mut Vec<String>,
) -> Result<Value, String> {
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let owner = building.last().map_or("", String::as_str).to_owned();
        let mut object = Map::new();
        for (property, property_schema) in properties {
            let value = if BYTE_PROPERTIES.contains(&(owner.as_str(), property.as_str())) {
                json!(BASE64_BYTES)
            } else if let Some((_, _, text)) = STRING_OVERRIDES
                .iter()
                .find(|(schema, name, _)| (*schema, *name) == (owner.as_str(), property.as_str()))
            {
                json!(text)
            } else {
                build(fixture, property_schema, building)?
            };
            object.insert(property.clone(), value);
        }
        return Ok(Value::Object(object));
    }
    let value = match schema.get("additionalProperties") {
        Some(entry @ Value::Object(_)) => build(fixture, entry, building)?,
        _ => return Ok(Value::Object(Map::new())),
    };
    Ok(json!({ "k": value }))
}

fn build_array(
    fixture: &Value,
    schema: &Map<String, Value>,
    building: &mut Vec<String>,
) -> Result<Value, String> {
    let items = schema.get("items").unwrap_or(&Value::Null);
    if let Some(target) = items.get("$ref").and_then(Value::as_str)
        && building
            .iter()
            .any(|name| name == ref_name(target).unwrap_or(""))
    {
        return Ok(json!([]));
    }
    let count = schema
        .get("minItems")
        .and_then(Value::as_u64)
        .map_or(1, |count| usize::try_from(count).unwrap_or(1));
    let mut array = Vec::with_capacity(count);
    for _ in 0..count {
        array.push(build(fixture, items, building)?);
    }
    Ok(Value::Array(array))
}
