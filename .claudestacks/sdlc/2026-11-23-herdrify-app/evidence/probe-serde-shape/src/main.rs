//! Probe: serde 1.0.229 / serde_json 1.0.154 mechanics on the shipped herdr-app 0.1.16 files.
//! A flattened internally tagged enum, a flattened struct, a catch-all map and serde_json::Number,
//! together: do typed fields take their keys, does the map keep the rest, does the round trip
//! give an equal JSON value, and is each number's form kept?
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

#[derive(Debug, Serialize, Deserialize)]
struct Node {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(flatten)]
    kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    floating: Option<Vec<Value>>,
    #[serde(flatten)]
    look: Look,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Kind {
    Block { content: Value },
    Row { children: Vec<Node> },
    Column { children: Vec<Node> },
    Stack { #[serde(skip_serializing_if = "Option::is_none")] active: Option<String>, children: Vec<Node> },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Size {
    Px { px: Number },
    Weight { weight: Number },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Look {
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_opacity: Option<Number>,
}

fn walk(n: &Node, extra_keys: &mut std::collections::BTreeSet<String>) {
    for k in n.extra.keys() {
        extra_keys.insert(k.clone());
    }
    if let Kind::Row { children } | Kind::Column { children } | Kind::Stack { children, .. } = &n.kind {
        for c in children {
            walk(c, extra_keys);
        }
    }
}

fn main() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fresh-home-0.1.17/layouts");
    for f in ["main.json", "terminals.json", "win98.json"] {
        let text = std::fs::read_to_string(format!("{dir}/{f}")).unwrap();
        let src: Value = serde_json::from_str(&text).unwrap();
        match serde_json::from_str::<Node>(&text) {
            Err(e) => println!("S1 {f}: parse error {e}"),
            Ok(n) => {
                let back = serde_json::to_value(&n).unwrap();
                let mut keys = std::collections::BTreeSet::new();
                walk(&n, &mut keys);
                println!("S1 {f}: round trip JSON-equal: {}", back == src);
                println!("S1 {f}: keys left in extra (any depth): {keys:?}");
                println!("S1 {f}: root look: {:?}", n.look);
            }
        }
    }
    // S2: number forms through Number, and through the internal-tag buffer.
    let t = r#"{"type":"block","content":{},"size":{"px":120.0},"font_size":15,"terminal_opacity":0.0}"#;
    let n: Node = serde_json::from_str(t).unwrap();
    println!("S2 in : {t}");
    println!("S2 out: {}", serde_json::to_string(&n).unwrap());
    // S3: an unknown node type, and a missing type.
    for t in [r#"{"type":"grid","id":"g"}"#, r#"{"id":"g"}"#] {
        println!("S3 {t} -> {:?}", serde_json::from_str::<Node>(t).map(|_| ()).map_err(|e| e.to_string()));
    }
    // S4: a typed field holding the wrong type, inside the flattened buffer.
    let t = r#"{"type":"block","content":{},"font_size":"big"}"#;
    println!("S4 {t} -> {:?}", serde_json::from_str::<Node>(t).map(|_| ()).map_err(|e| e.to_string()));

    // S5: the alternative shape: a flat node with an explicit `type` field and kind fields as
    // options, a flattened struct and a catch-all map. Duplicate keys show as text longer than
    // the text of the same value after a pass through `Value` (which keeps one of each key).
    for f in ["main.json", "terminals.json", "win98.json"] {
        let text = std::fs::read_to_string(format!("{dir}/{f}")).unwrap();
        let src: Value = serde_json::from_str(&text).unwrap();
        let n: FlatNode = serde_json::from_str(&text).unwrap();
        let out = serde_json::to_string(&n).unwrap();
        let via_value = serde_json::to_string(&serde_json::to_value(&n).unwrap()).unwrap();
        let mut keys = std::collections::BTreeSet::new();
        walk_flat(&n, &mut keys);
        println!(
            "S5 {f}: JSON-equal={} duplicate_keys_in_text={} extra_keys={keys:?}",
            serde_json::from_str::<Value>(&out).unwrap() == src,
            out.len() != via_value.len()
        );
    }
    let t = r#"{"type":"block","content":{},"size":{"px":120.0},"font_size":15,"terminal_opacity":0.0,"zz":1}"#;
    let n: FlatNode = serde_json::from_str(t).unwrap();
    println!("S5 in : {t}");
    println!("S5 out: {}", serde_json::to_string(&n).unwrap());
    let t = r#"{"type":"grid","id":"g"}"#;
    println!("S5 unknown type {t} -> {:?}", serde_json::from_str::<FlatNode>(t).map(|n| n.kind).map_err(|e| e.to_string()));
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum NodeType {
    Block,
    Row,
    Column,
    Stack,
    #[serde(untagged)]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct FlatNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "type")]
    kind: NodeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FlatNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<Size>,
    #[serde(flatten)]
    look: Look,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

fn walk_flat(n: &FlatNode, extra_keys: &mut std::collections::BTreeSet<String>) {
    for k in n.extra.keys() {
        extra_keys.insert(k.clone());
    }
    for c in n.children.iter().flatten() {
        walk_flat(c, extra_keys);
    }
}
