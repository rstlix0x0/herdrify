//! Probe: serde 1.0.229 / serde_json 1.0.154 messages and behaviours the herdrify-app spec quotes,
//! and every JSON example in the spec parsed with serde_json.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

#[derive(Debug, Deserialize, Serialize)]
struct Theme {
    background: String,
    foreground: String,
    selection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
    ansi: [String; 16],
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum Content {
    Mod {
        #[serde(rename = "mod")]
        module: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<Map<String, Value>>,
    },
    Terminal { terminal: String },
    Other(Map<String, Value>),
}

#[derive(Debug, Deserialize)]
struct Listing {
    #[allow(dead_code)]
    spaces: Vec<Value>,
}

fn show<T: std::fmt::Debug>(label: &str, r: Result<T, serde_json::Error>) {
    match r {
        Ok(v) => println!("{label} -> Ok({v:?})"),
        Err(e) => println!("{label} -> Err({e})"),
    }
}

fn main() {
    let theme16 = r##"["#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000"]"##;

    // M1: missing required field, wrong length, wrong type.
    show("M1 missing background", serde_json::from_str::<Theme>(r#"{"ansi":[]}"#));
    show("M1 ansi length 1", serde_json::from_str::<Theme>(r##"{"background":"#000000","foreground":"#000000","selection":"#000000","ansi":["#000000"]}"##));
    show("M1 background 7", serde_json::from_str::<Theme>(r#"{"background":7}"#));

    // M2: empty text, trailing text, a byte-order mark.
    show("M2 empty", serde_json::from_str::<Theme>(""));
    let ok = format!(r##"{{"background":"#000000","foreground":"#000000","selection":"#000000","ansi":{theme16}}}"##);
    show("M2 trailing", serde_json::from_str::<Theme>(&format!("{ok} x")));
    show("M2 BOM", serde_json::from_str::<Theme>(&format!("\u{feff}{ok}")));

    // M3: a duplicate named field; a duplicate unknown key (lands in the flattened map).
    let dup_named = format!(r##"{{"background":"#000000","background":"#111111","foreground":"#000000","selection":"#000000","ansi":{theme16}}}"##);
    show("M3 duplicate named field", serde_json::from_str::<Theme>(&dup_named));
    let dup_extra = format!(r##"{{"background":"#000000","foreground":"#000000","selection":"#000000","ansi":{theme16},"x":1,"x":2}}"##);
    show("M3 duplicate unknown key", serde_json::from_str::<Theme>(&dup_extra).map(|t| t.extra));
    show("M3 duplicate key in a Value", serde_json::from_str::<Value>(r#"{"x":1,"x":2}"#));

    // M4: an output missing a field, empty stdout, stdout that is not UTF-8.
    show("M4 ls missing spaces", serde_json::from_slice::<Listing>(br#"{"recent":[]}"#));
    show("M4 empty stdout", serde_json::from_slice::<Listing>(b""));
    show("M4 not UTF-8", serde_json::from_slice::<Value>(b"{\"spaces\":[\"\xff\"]}"));

    // M5: untagged content: known shapes and an unknown one kept.
    for t in [r#"{"mod":"files","props":{"role":"x"}}"#, r#"{"terminal":"t7"}"#, r#"{"agent":"focused"}"#] {
        let c: Content = serde_json::from_str(t).unwrap();
        println!("M5 {t} -> {c:?} -> {}", serde_json::to_string(&c).unwrap());
    }

    // M6: Number keeps the written form, including exponents and negative zero.
    for t in ["120.0", "268", "4.286", "0.0", "1e3", "-0.0", "-5"] {
        let n: Number = serde_json::from_str(t).unwrap();
        println!("M6 {t} -> {} (is_f64={})", serde_json::to_string(&n).unwrap(), n.is_f64());
    }
    println!("M6 Number::from_f64(NaN) = {:?}", Number::from_f64(f64::NAN));

    // M7: to_string_pretty layout.
    let v: Value = serde_json::from_str(r#"{"id":"root","size":{"px":240}}"#).unwrap();
    println!("M7 pretty:\n{}", serde_json::to_string_pretty(&v).unwrap());

    // M8: JSON examples the spec quotes, parsed.
    let examples = [
        ("AGENTS.md:514-520 keys", r#"{"global":{"ctrl-shift-t":"tab.new","ctrl-shift-o":"pane.split:right","alt-1":"space.select:1"},"terminal":{"ctrl-shift-c":"terminal.copy","ctrl-left":"send:\u001bb","ctrl-1":"terminal"},"layout:terminals":{"ctrl-shift-pagedown":"space.next","ctrl-shift-pageup":"space.prev"},"mod:editor":{"ctrl-s":"editor.save"}}"#),
        ("AGENTS.md:531 object action", r#"{"ctrl-shift-r":{"command":"layout.resize","args":{"id":"sidebar","px":240}}}"#),
        ("AGENTS.md:376-380 pane frame", r#"{"tab":"active","pane":{"gap":6,"radius":6,"background":"surface","border":{"width":1,"color":"border"},"focus":{"width":1,"color":"accent"},"header":{"content":{"mod":"pane"},"height":30,"position":"top","reveal":"always"},"lone":false}}"#),
        ("AGENTS.md:406 floating", r#"{"floating":[{"id":"board","content":{"mod":"board"},"edge":"left","align":"center","peek":52}]}"#),
        ("AGENTS.md:236 gradient", r##"{"gradient":["#2a6cbd","#0b2a57"],"angle":165}"##),
        ("spec invocation tab.new", r#"{"name":"api","argv":["claude"]}"#),
    ];
    for (label, t) in examples {
        show(&format!("M8 {label}"), serde_json::from_str::<Value>(t).map(|_| "parses"));
    }

    // M9: a try_from newtype's refusal, as serde_json reports it inside a file.
    show("M9 colour #12345", serde_json::from_str::<Line>(r##"{"color":"#12345"}"##));
    show("M9 colour ok", serde_json::from_str::<Line>(r##"{"color":"#ffffff0d"}"##));

    // M10: the same refusal inside an untagged enum.
    show("M10 paint #12345", serde_json::from_str::<Holder>(r##"{"background":"#12345"}"##));
    show("M10 paint word", serde_json::from_str::<Holder>(r#"{"background":"canvas"}"#));
    show("M10 paint object", serde_json::from_str::<Holder>(r#"{"background":{"image":"a.jpg","fit":"cover"}}"#));
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Paint {
    #[allow(dead_code)]
    Color(Color),
    #[allow(dead_code)]
    Other(Map<String, Value>),
}

#[derive(Debug, Deserialize)]
struct Holder {
    #[allow(dead_code)]
    background: Paint,
}

#[derive(Debug)]
struct ColorError(String);
impl std::fmt::Display for ColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\" is not a hex colour: use #rrggbb or #rrggbbaa", self.0)
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
struct Color(#[allow(dead_code)] String);
impl TryFrom<String> for Color {
    type Error = ColorError;
    fn try_from(s: String) -> Result<Self, ColorError> {
        let hex = s.strip_prefix('#').map(|h| (h.len() == 6 || h.len() == 8) && h.chars().all(|c| c.is_ascii_hexdigit()));
        match hex {
            Some(false) => Err(ColorError(s)),
            _ => Ok(Self(s)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Line {
    #[allow(dead_code)]
    color: Color,
}
