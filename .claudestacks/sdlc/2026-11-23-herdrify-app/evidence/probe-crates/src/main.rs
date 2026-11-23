//! Probe: crate behaviour the herdrify-app design relies on.
//! tokio 1.54.1 process handling; serde 1.0.229 / serde_json 1.0.154 shapes and round trips.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Node {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_opacity: Option<f64>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Size {
    Px { px: u32 },
    Weight { weight: f64 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Paint {
    Color(String),
    Gradient { gradient: Vec<String>, angle: f64 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Tagged {
    Block { id: String },
    Row { id: String, children: Vec<Tagged> },
}

#[tokio::main]
async fn main() {
    // P1: a program that does not exist.
    let e = Command::new("herdr-app-does-not-exist").output().await.unwrap_err();
    println!("P1 missing program: kind={:?} display={e}", e.kind());

    // P2: kill_on_drop(true) kills the child when the output() future is dropped.
    let pidfile = std::env::temp_dir().join("probe-crates-pid");
    let _ = std::fs::remove_file(&pidfile);
    let mut c = Command::new("sh");
    c.args(["-c", &format!("echo $$ > {}; exec sleep 30", pidfile.display())]).kill_on_drop(true);
    let r = tokio::time::timeout(Duration::from_millis(700), c.output()).await;
    println!("P2 output() timed out: {}", r.is_err());
    tokio::time::sleep(Duration::from_millis(300)).await;
    let pid = std::fs::read_to_string(&pidfile).unwrap_or_default().trim().to_string();
    let alive = std::process::Command::new("kill").args(["-0", &pid]).status().unwrap().success();
    println!("P2 child pid {pid} alive after drop with kill_on_drop(true): {alive}");

    // P3: exit status of a signalled child has no code.
    let mut s = Command::new("sh");
    s.args(["-c", "kill -TERM $$"]);
    let st = s.output().await.unwrap().status;
    println!("P3 signalled child: code={:?} success={}", st.code(), st.success());

    // P4: Value equality across integer and float forms.
    println!("P4 json!(1) == json!(1.0): {}", json!(1) == json!(1.0));
    println!("P4 f64 15.0 serializes as: {}", serde_json::to_string(&15.0_f64).unwrap());
    println!("P4 u32 parses 15.0: {:?}", serde_json::from_str::<u32>("15.0").map_err(|e| e.to_string()));
    println!("P4 f64 parses 15: {:?}", serde_json::from_str::<f64>("15"));

    // P5: flatten keeps unknown keys; round trip; key order of the output.
    let src = json!({"zeta":1,"id":"root","size":{"px":240},"alpha":{"k":[1,2]},"terminal_opacity":1});
    let n: Node = serde_json::from_value(src.clone()).unwrap();
    println!("P5 parsed: {n:?}");
    let back = serde_json::to_value(&n).unwrap();
    println!("P5 back: {back}");
    println!("P5 back == src: {}", back == src);
    println!("P5 serialized text order: {}", serde_json::to_string(&n).unwrap());

    // P6: untagged enums: which variant, and the error when none match.
    for t in [r#"{"px":240}"#, r#"{"weight":2}"#, r#"{"px":2.5}"#, r#"{"other":1}"#] {
        println!("P6 Size {t} -> {:?}", serde_json::from_str::<Size>(t).map_err(|e| e.to_string()));
    }
    for t in [r##""border""##, r##"{"gradient":["#2a6cbd","#0b2a57"],"angle":165}"##, r#"{"image":"x.jpg"}"#] {
        println!("P6 Paint {t} -> {:?}", serde_json::from_str::<Paint>(t).map_err(|e| e.to_string()));
    }

    // P7: internally tagged recursive enum and its error for an unknown type.
    for t in [r#"{"type":"row","id":"r","children":[{"type":"block","id":"b"}]}"#, r#"{"type":"stack","id":"s"}"#] {
        println!("P7 Tagged {t} -> {:?}", serde_json::from_str::<Tagged>(t).map_err(|e| e.to_string()));
    }

    // P9: which paths std calls absolute (the Folder rule).
    for p in ["/tmp/x", "x/y", "~/code", "./x", ""] {
        println!("P9 Path::new({p:?}).is_absolute() = {}", std::path::Path::new(p).is_absolute());
    }

    // P8: deny of a missing required field, and a wrong type, as serde_json words it.
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Theme {
        background: String,
        ansi: [String; 16],
    }
    for t in [r#"{"ansi":[]}"#, r##"{"background":"#000000","ansi":["#000000"]}"##, r#"{"background":7}"#] {
        println!("P8 Theme {t} -> {:?}", serde_json::from_str::<Theme>(t).map_err(|e| e.to_string()));
    }
}
