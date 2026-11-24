//! Probe: the herdrify-app file-model shapes (as the spec names them) with serde 1.0.229 /
//! serde_json 1.0.154, over the herdr-app 0.1.17 fixtures and the edge inputs.
//! Names are plain `String`s here; the shapes (flat node, flattened structs, `extra` in every
//! object, untagged enums with `extra` in each struct variant and a kept `Other`, custom
//! Section/Action) are the ones under test.
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

type Extra = Map<String, Value>;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum NodeKind { Block, Row, Column, Stack, #[serde(untagged)] Other(String) }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Shadow { Sm, Md, Lg, #[serde(untagged)] Other(String) }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Titlebar { None, #[serde(untagged)] Other(String) }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Edge { Top, Right, Bottom, Left, #[serde(untagged)] Other(String) }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Align { Start, Center, End, #[serde(untagged)] Other(String) }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Size {
    Px { px: Number, #[serde(flatten)] extra: Extra },
    Weight { weight: Number, #[serde(flatten)] extra: Extra },
    Other(Extra),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Padding { Even(Number), Pair([Number; 2]) }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Line {
    #[serde(skip_serializing_if = "Option::is_none")] width: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] color: Option<String>,
    #[serde(flatten)] extra: Extra,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Paint {
    Color(String),
    Gradient { gradient: Vec<String>, #[serde(skip_serializing_if = "Option::is_none")] angle: Option<Number>, #[serde(flatten)] extra: Extra },
    Image { image: String, #[serde(skip_serializing_if = "Option::is_none")] fit: Option<String>, #[serde(flatten)] extra: Extra },
    Other(Extra),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PaneHeader {
    content: Box<Content>,
    #[serde(skip_serializing_if = "Option::is_none")] height: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] reveal: Option<String>,
    #[serde(flatten)] extra: Extra,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PaneFrame {
    #[serde(skip_serializing_if = "Option::is_none")] gap: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] radius: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] background: Option<Paint>,
    #[serde(skip_serializing_if = "Option::is_none")] border: Option<Line>,
    #[serde(skip_serializing_if = "Option::is_none")] focus: Option<Line>,
    #[serde(skip_serializing_if = "Option::is_none")] header: Option<PaneHeader>,
    #[serde(skip_serializing_if = "Option::is_none")] lone: Option<bool>,
    #[serde(flatten)] extra: Extra,
}

/// A non-empty string: an empty one makes its variant fail, as `ModName` would.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "String", into = "String")]
struct Name(String);
impl TryFrom<String> for Name {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        if s.is_empty() { Err("a mod name cannot be empty".into()) } else { Ok(Self(s)) }
    }
}
impl From<Name> for String { fn from(n: Name) -> String { n.0 } }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Content {
    Mod { #[serde(rename = "mod")] module: Name, #[serde(skip_serializing_if = "Option::is_none")] props: Option<Extra>, #[serde(flatten)] extra: Extra },
    Terminal { terminal: String, #[serde(flatten)] extra: Extra },
    Tab { tab: String, #[serde(skip_serializing_if = "Option::is_none")] pane: Option<PaneFrame>, #[serde(flatten)] extra: Extra },
    Agent { agent: String, #[serde(flatten)] extra: Extra },
    Other(Extra),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct FloatingBlock {
    #[serde(skip_serializing_if = "Option::is_none")] id: Option<String>,
    content: Content,
    #[serde(skip_serializing_if = "Option::is_none")] edge: Option<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")] align: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")] peek: Option<Number>,
    #[serde(flatten)] extra: Extra,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum ThemeChoice {
    Name(String),
    ByAppearance { light: String, dark: String, #[serde(flatten)] extra: Extra },
    Other(Value),
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct Look {
    #[serde(skip_serializing_if = "Option::is_none")] theme: Option<ThemeChoice>,
    #[serde(skip_serializing_if = "Option::is_none")] colors: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")] font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] font_size: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] ui_font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] font_weight: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] terminal_padding: Option<Padding>,
    #[serde(skip_serializing_if = "Option::is_none")] terminal_opacity: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] terminal_line_height: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] terminal_cell_width: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] terminal_scroll_speed: Option<Number>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ClonedFrom { name: String, version: Value, #[serde(flatten)] extra: Extra }

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct RootOptions {
    #[serde(skip_serializing_if = "Option::is_none")] titlebar: Option<Titlebar>,
    #[serde(skip_serializing_if = "Option::is_none")] traffic_lights: Option<[Number; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")] window_blur: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] terminal_menu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] toast: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] link_handlers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")] cloned_from: Option<ClonedFrom>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct FocusStyle { #[serde(skip_serializing_if = "Option::is_none")] border: Option<Line>, #[serde(flatten)] extra: Extra }

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Node {
    #[serde(skip_serializing_if = "Option::is_none")] id: Option<String>,
    #[serde(rename = "type")] kind: NodeKind,
    #[serde(skip_serializing_if = "Option::is_none")] content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")] children: Option<Vec<Node>>,
    #[serde(skip_serializing_if = "Option::is_none")] active: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] header: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")] size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")] min: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] max: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] radius: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] gap: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")] padding: Option<Padding>,
    #[serde(skip_serializing_if = "Option::is_none")] background: Option<Paint>,
    #[serde(skip_serializing_if = "Option::is_none")] border: Option<Line>,
    #[serde(skip_serializing_if = "Option::is_none")] divider: Option<Line>,
    #[serde(skip_serializing_if = "Option::is_none")] shadow: Option<Shadow>,
    #[serde(skip_serializing_if = "Option::is_none")] focus: Option<FocusStyle>,
    #[serde(skip_serializing_if = "Option::is_none")] hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] resizable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] movable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] window_drag: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] floating: Option<Vec<FloatingBlock>>,
    #[serde(flatten)] look: Look,
    #[serde(flatten)] root: RootOptions,
    #[serde(flatten)] extra: Extra,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Theme {
    background: String, foreground: String, selection: String,
    #[serde(skip_serializing_if = "Option::is_none")] cursor: Option<String>,
    ansi: [String; 16],
    #[serde(skip_serializing_if = "Option::is_none")] colors: Option<BTreeMap<String, String>>,
    #[serde(flatten)] extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Section { Global, Terminal, Layout(String), Mod(String), Other(String) }
impl Section {
    fn parse(s: &str) -> Self {
        match s {
            "global" => Self::Global,
            "terminal" => Self::Terminal,
            _ => match (s.strip_prefix("layout:"), s.strip_prefix("mod:")) {
                (Some(n), _) if !n.is_empty() => Self::Layout(n.into()),
                (_, Some(n)) if !n.is_empty() => Self::Mod(n.into()),
                _ => Self::Other(s.into()),
            },
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Global => "global".into(),
            Self::Terminal => "terminal".into(),
            Self::Layout(n) => format!("layout:{n}"),
            Self::Mod(n) => format!("mod:{n}"),
            Self::Other(s) => s.clone(),
        }
    }
}
impl Serialize for Section {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&self.text()) }
}
impl<'de> Deserialize<'de> for Section {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> { Ok(Self::parse(&String::deserialize(d)?)) }
}

#[derive(Debug, PartialEq)]
enum Action {
    Terminal,
    Send(String),
    Run { command: String, value: Option<String> },
    Command { command: String, args: Option<Extra>, extra: Extra },
    Other(Value),
}
impl Serialize for Action {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Terminal => s.serialize_str("terminal"),
            Self::Send(t) => s.serialize_str(&format!("send:{t}")),
            Self::Run { command, value: None } => s.serialize_str(command),
            Self::Run { command, value: Some(v) } => s.serialize_str(&format!("{command}:{v}")),
            Self::Command { command, args, extra } => {
                let mut m = extra.clone();
                m.insert("command".into(), Value::String(command.clone()));
                if let Some(a) = args { m.insert("args".into(), Value::Object(a.clone())); }
                m.serialize(s)
            }
            Self::Other(v) => v.serialize(s),
        }
    }
}
impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Value::deserialize(d)? {
            Value::String(s) if s == "terminal" => Ok(Self::Terminal),
            Value::String(s) if s.starts_with("send:") => Ok(Self::Send(s["send:".len()..].into())),
            Value::String(s) => {
                let (command, value) = match s.split_once(':') { Some((c, v)) => (c.to_string(), Some(v.to_string())), None => (s.clone(), None) };
                if command.is_empty() {
                    return Err(D::Error::custom(format!("\"{s}\" is not a key action: its command name is empty")));
                }
                Ok(Self::Run { command, value })
            }
            Value::Object(mut m) => match (m.get("command"), m.get("args")) {
                (Some(Value::String(c)), None | Some(Value::Object(_))) => {
                    if c.is_empty() { return Err(D::Error::custom("a command name cannot be empty")); }
                    let command = c.clone();
                    m.remove("command");
                    let args = match m.remove("args") { Some(Value::Object(a)) => Some(a), _ => None };
                    Ok(Self::Command { command, args, extra: m })
                }
                _ => Ok(Self::Other(Value::Object(m))),
            },
            other => Ok(Self::Other(other)),
        }
    }
}

type Keys = BTreeMap<Section, BTreeMap<String, Action>>;

fn dup_free(text: &str) -> bool {
    let v: Value = serde_json::from_str(text).unwrap();
    serde_json::to_string_pretty(&v).unwrap().len() == text.len()
}

fn check<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(label: &str, path: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    let src: Value = serde_json::from_str(&text).unwrap();
    match serde_json::from_str::<T>(&text) {
        Err(e) => println!("R {label}: parse error: {e}"),
        Ok(m) => {
            let out = serde_json::to_string_pretty(&m).unwrap();
            let back: Value = serde_json::from_str(&out).unwrap();
            let again = serde_json::to_string_pretty(&serde_json::from_str::<T>(&out).unwrap()).unwrap();
            println!("R {label}: JSON-equal={} no_duplicate_keys={} reparse_stable={}", back == src, dup_free(&out), again == out);
        }
    }
}

fn show<T: std::fmt::Debug + Serialize>(label: &str, r: Result<T, serde_json::Error>) {
    match r {
        Ok(v) => println!("{label} -> Ok({v:?}) -> {}", serde_json::to_string(&v).unwrap()),
        Err(e) => println!("{label} -> Err({e})"),
    }
}

fn main() {
    let fx = concat!(env!("CARGO_MANIFEST_DIR"), "/../fresh-home-0.1.17");
    for f in ["main", "terminals", "win98"] {
        check::<Node>(&format!("layouts/{f}.json"), &format!("{fx}/layouts/{f}.json"));
    }
    for f in ["atom-one-dark", "win98"] {
        check::<Theme>(&format!("themes/{f}.json"), &format!("{fx}/themes/{f}.json"));
    }
    check::<Keys>("keys.json", &format!("{fx}/keys.json"));

    // E1: struct variants keep their unknown keys.
    show("E1 content terminal+x", serde_json::from_str::<Content>(r#"{"terminal":"t7","x":1}"#));
    show("E1 size px+weight", serde_json::from_str::<Size>(r#"{"px":1,"weight":2}"#));
    show("E1 theme by appearance+x", serde_json::from_str::<ThemeChoice>(r#"{"light":"paper","dark":"ink","x":1}"#));
    // E2: an invalid inner value makes the object fall through to Other.
    show("E2 content mod empty", serde_json::from_str::<Content>(r#"{"mod":""}"#));
    show("E2 size px string", serde_json::from_str::<Size>(r#"{"px":"x"}"#));
    show("E2 content agent", serde_json::from_str::<Content>(r#"{"agent":"focused"}"#));
    show("E2 content tab+pane", serde_json::from_str::<Content>(r#"{"tab":"active","pane":{"gap":6,"lone":false,"zz":1}}"#));
    show("E2 content unknown", serde_json::from_str::<Content>(r#"{"board":"x"}"#));
    // E3: a string where Paint wants a colour or object: refused, serde's untagged message.
    show("E3 paint number", serde_json::from_str::<Paint>("7"));
    // E4: keys actions of every kind.
    for t in [r#""pane.split:right""#, r#""space.new:{a:b}""#, r#""send:""#, r#""terminal""#, r#""terminal.copy""#,
              r#"{"command":"layout.resize","args":{"id":"sidebar","px":240}}"#, r#"{"command":"x","args":7}"#,
              r#"{"args":{}}"#, "7", "null", "true", r#"["a"]"#, r#"":x""#, r#""""#, r#"{"command":""}"#] {
        show(&format!("E4 action {t}"), serde_json::from_str::<Action>(t));
    }
    // E5: sections.
    for s in ["global", "terminal", "layout:herdrify-main", "mod:herdrify-host", "layout:", "mod:", "weird"] {
        println!("E5 section {s:?} -> {:?}", Section::parse(s));
    }
    // E6: the link_handlers root option and an unknown node type, kept.
    show("E6 node", serde_json::from_str::<Node>(r#"{"type":"grid","link_handlers":["my-browser"],"zz":[1,2.0]}"#));
}
