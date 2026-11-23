//! Offline checks that the SDK matches the hc 0.1.21 `OpenRPC` document.
//!
//! The document is a copy kept beside this file, so these checks need no `hc` and no network.
//! Four things are checked: the method names, the parameter names each request sends, the
//! component schemas, which must read into an SDK type and write back unchanged, and, at compile
//! time, that every `Method::Output` is the reply type the schema gives that method.

#[path = "support/schema_instance.rs"]
pub mod schema_instance;

use std::collections::BTreeSet;
use std::error::Error;
use std::time::Duration;

use herdrify_hc::protocol::{
    Argv, AttachResult, CallerId, Children, Command, CoreInfo, DisplayFacts, DocGet, DocGetResult,
    DocName, DocSet, DocSetResult, DocVersion, Edit, Environment, EventsSubscribe, ExecCommand,
    ExecResult, FileEdit, FileName, FileRead, FileReadResult, FileResult, FileWrite, FirstMessage,
    Foreground, Generation, HelloParams, HelloResult, HolderInfo, IdleOrWorking, InitialSize,
    InterruptResult, Keep, KeyName, KeysResult, LabelChange, LabelChanges, LabelKey, LabelValue,
    Labels, ListResult, LoginSession, Machine, MachineAdd, MachineConnect, MachineExec,
    MachineList, MachineListResult, MachineName, MachineRename, MachineRm, MachineTarget,
    MachineView, Method, PutResult, ReadResult, RemotePath, RemoveResult, RpcDiscover, RunResult,
    SendResult, ServerRestart, ServerStatus, ServerStop, SessionName, Status, StatusResult,
    StopResult, SubscribeResult, TemplateInfo, TemplatesList, TemplatesResult, Terminal,
    TerminalAttach, TerminalGet, TerminalInterrupt, TerminalKeep, TerminalKeys, TerminalKill,
    TerminalLabel, TerminalList, TerminalName, TerminalPut, TerminalRead, TerminalRef, TerminalRm,
    TerminalRun, TerminalSend, TerminalWait, WaitResult, WaitTarget,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use schema_instance::instance;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture() -> Result<Value, serde_json::Error> {
    serde_json::from_str(include_str!("fixtures/hc-0.1.21.openrpc.json"))
}

#[test]
fn instance_fills_every_property() -> TestResult {
    let fixture = fixture()?;
    assert_eq!(instance(&fixture, "Edit")?, json!({"new": "x", "old": "x"}));
    Ok(())
}

#[test]
fn instance_applies_the_value_rules() -> TestResult {
    let fixture = fixture()?;
    assert_eq!(
        instance(&fixture, "InitialSize")?,
        json!({"cols": 1, "rows": 1})
    );
    assert_eq!(
        instance(&fixture, "Display")?,
        json!({"bg": "x", "cell": [1, 1], "cursor": "x", "fg": "x"})
    );
    assert_eq!(instance(&fixture, "Children")?, json!("any"));
    let exec = instance(&fixture, "ExecResult")?;
    assert_eq!(exec["stdout"], json!("eAo="));
    assert_eq!(exec["stderr"], json!("eAo="));
    assert_eq!(exec["ms"], json!(1));
    assert_eq!(exec["timed_out"], json!(true));
    assert_eq!(exec["code"], json!(1));
    let doc = instance(&fixture, "DocGetResult")?;
    assert_eq!(doc["value"], json!("x"));
    let labelled = instance(&fixture, "RunResult")?;
    assert_eq!(labelled["labels"], json!({"k": "x"}));
    Ok(())
}

#[test]
fn instance_stops_at_a_cycle() -> TestResult {
    let fixture = fixture()?;
    assert_eq!(instance(&fixture, "WaitResult")?["children"], json!([]));
    assert_eq!(
        instance(&fixture, "RunResult")?["wait"]["children"],
        json!([])
    );
    Ok(())
}

/// What one value of a union request type must send, and which of its variants it stands for.
struct Shape {
    /// The position of the variant among all the variants of its type.
    slot: usize,
    /// The fixture names this variant never sends because another variant does.
    lacks: &'static [&'static str],
    /// The names beyond the required ones that the variant's constructor argument sends.
    extra: &'static [&'static str],
}

/// The shape of a request with a single variant.
const PLAIN: Shape = Shape {
    slot: 0,
    lacks: &[],
    extra: &[],
};

/// The parameter names one request sends in one situation.
struct Variant {
    /// The variant of the request's union type that this situation builds.
    slot: usize,
    /// The names sent when every setter is used.
    full: BTreeSet<String>,
    /// The names sent when only the constructor arguments are given.
    minimal: BTreeSet<String>,
    /// The fixture names this variant never sends because another variant does.
    lacks: &'static [&'static str],
    /// The names beyond the required ones that the variant's constructor argument sends.
    extra: &'static [&'static str],
}

/// One method and the situations its request is built in.
struct Case {
    method: &'static str,
    /// How many variants the request's union type has: every slot below it needs a situation.
    slots: usize,
    variants: Vec<Variant>,
}

fn names<P: Serialize>(params: &P) -> Result<BTreeSet<String>, Box<dyn Error>> {
    match serde_json::to_value(params)? {
        Value::Object(object) => Ok(object.keys().cloned().collect()),
        other => Err(format!("params serialized as {other}, not an object").into()),
    }
}

fn variant<P: Serialize>(full: &P, minimal: &P, shape: &Shape) -> Result<Variant, Box<dyn Error>> {
    Ok(Variant {
        slot: shape.slot,
        full: names(full)?,
        minimal: names(minimal)?,
        lacks: shape.lacks,
        extra: shape.extra,
    })
}

/// A method whose request sends the same names however it is built.
fn plain<M: Method>(full: &M::Params, minimal: &M::Params) -> Result<Case, Box<dyn Error>> {
    Ok(Case {
        method: M::NAME,
        slots: 1,
        variants: vec![variant(full, minimal, &PLAIN)?],
    })
}

const fn exclusive<M: Method>(slots: usize, variants: Vec<Variant>) -> Case {
    Case {
        method: M::NAME,
        slots,
        variants,
    }
}

fn terminal() -> Result<TerminalRef, Box<dyn Error>> {
    Ok(TerminalRef::new("t1")?)
}

fn machine() -> Result<MachineName, Box<dyn Error>> {
    Ok(MachineName::new("m1")?)
}

fn caller() -> CallerId {
    CallerId::new("c")
}

fn path() -> RemotePath {
    RemotePath::new("/p")
}

fn display() -> DisplayFacts {
    DisplayFacts {
        bg: Some("#000000".to_owned()),
        cell: Some([1, 1]),
        cursor: Some("#000000".to_owned()),
        fg: Some("#ffffff".to_owned()),
    }
}

fn environment() -> Environment {
    std::iter::once(("K", "v")).collect()
}

fn labels() -> Result<Labels, Box<dyn Error>> {
    Ok(std::iter::once((LabelKey::new("k")?, LabelValue::new("v")?)).collect())
}

const fn seconds() -> Duration {
    Duration::from_secs(1)
}

fn argv() -> Result<Argv, Box<dyn Error>> {
    Ok(Argv::new(vec!["hc".to_owned()])?)
}

fn server_cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let hello = HelloParams::new()
        .calls_you("rohan")
        .core(CoreInfo {
            id: "i".to_owned(),
            name: "n".to_owned(),
        })
        .features(["f"])
        .framed()
        .generation(Generation::ONE);
    Ok(vec![
        plain::<HelloParams>(&hello, &HelloParams::new())?,
        plain::<ServerStatus>(&ServerStatus::new(), &ServerStatus::new())?,
        plain::<ServerStop>(&ServerStop::new(), &ServerStop::new())?,
        plain::<ServerRestart>(
            &ServerRestart::new().environment(environment()),
            &ServerRestart::new(),
        )?,
        plain::<TemplatesList>(&TemplatesList::new(), &TemplatesList::new())?,
        plain::<RpcDiscover>(&RpcDiscover::new(), &RpcDiscover::new())?,
        plain::<EventsSubscribe>(
            &EventsSubscribe::new()
                .children_of(terminal()?)
                .labels(labels()?)
                .terminal(terminal()?),
            &EventsSubscribe::new(),
        )?,
    ])
}

fn run_with_every_setter(command: Command) -> Result<TerminalRun, Box<dyn Error>> {
    Ok(TerminalRun::new(command)
        .caller(caller())
        .cwd(path())
        .die_with_parent()
        .display(display())
        .env(environment())
        .keep()
        .labels(labels()?)
        .name(TerminalName::new("n")?)
        .no_notify()
        .no_parent()
        .on(machine()?)
        .outlive_parent()
        .parent(terminal()?)
        .rm()
        .size(InitialSize {
            cols: 80.try_into()?,
            rows: 24.try_into()?,
        })
        .timeout(seconds())
        .wait())
}

fn wait_with_every_setter(target: WaitTarget) -> TerminalWait {
    TerminalWait::new(target)
        .caller(caller())
        .output("x".to_owned())
        .timeout(seconds())
}

/// How many variants `Command` has.
const COMMAND_SLOTS: usize = 3;

/// The shape of each `Command`. A new variant stops this match compiling.
const fn command_shape(command: &Command) -> Shape {
    match command {
        Command::Argv(_) => Shape {
            slot: 0,
            lacks: &["line"],
            extra: &["argv"],
        },
        Command::Line(_) => Shape {
            slot: 1,
            lacks: &["argv"],
            extra: &["line"],
        },
        Command::Shell => Shape {
            slot: 2,
            lacks: &["argv", "line"],
            extra: &[],
        },
    }
}

/// How many variants `WaitTarget` has.
const WAIT_SLOTS: usize = 3;

/// The shape of each `WaitTarget`. A new variant stops this match compiling.
const fn wait_shape(target: &WaitTarget) -> Shape {
    match target {
        WaitTarget::Terminal(_) => Shape {
            slot: 0,
            lacks: &["children"],
            extra: &["terminal"],
        },
        WaitTarget::Children(_) => Shape {
            slot: 1,
            lacks: &["terminal"],
            extra: &["children"],
        },
        WaitTarget::ChildrenOf(..) => Shape {
            slot: 2,
            lacks: &[],
            extra: &["terminal", "children"],
        },
    }
}

fn run_and_wait_cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let commands = [
        Command::Argv(argv()?),
        Command::Line("ls".to_owned()),
        Command::Shell,
    ];
    let run = commands
        .into_iter()
        .map(|command| {
            variant(
                &run_with_every_setter(command.clone())?,
                &TerminalRun::new(command.clone()),
                &command_shape(&command),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let targets = [
        WaitTarget::Terminal(terminal()?),
        WaitTarget::Children(Children::Any),
        WaitTarget::ChildrenOf(terminal()?, Children::All),
    ];
    let wait = targets
        .into_iter()
        .map(|target| {
            variant(
                &wait_with_every_setter(target.clone()),
                &TerminalWait::new(target.clone()),
                &wait_shape(&target),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(vec![
        exclusive::<TerminalRun>(COMMAND_SLOTS, run),
        exclusive::<TerminalWait>(WAIT_SLOTS, wait),
    ])
}

fn terminal_cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let changes: LabelChanges = [
        (LabelKey::new("a")?, LabelChange::Set(LabelValue::new("v")?)),
        (LabelKey::new("b")?, LabelChange::Remove),
    ]
    .into_iter()
    .collect();
    let mut cases = run_and_wait_cases()?;
    cases.extend([
        plain::<TerminalList>(
            &TerminalList::new().all().labels(labels()?),
            &TerminalList::new(),
        )?,
        plain::<TerminalGet>(
            &TerminalGet::new(terminal()?).caller(caller()),
            &TerminalGet::new(terminal()?),
        )?,
        plain::<TerminalKill>(
            &TerminalKill::new(terminal()?).caller(caller()),
            &TerminalKill::new(terminal()?),
        )?,
        plain::<TerminalRm>(
            &TerminalRm::new(terminal()?).caller(caller()),
            &TerminalRm::new(terminal()?),
        )?,
        plain::<TerminalKeep>(
            &TerminalKeep::new(terminal()?, Keep::On),
            &TerminalKeep::new(terminal()?, Keep::On),
        )?,
        plain::<TerminalLabel>(
            &TerminalLabel::new(terminal()?, changes.clone()),
            &TerminalLabel::new(terminal()?, changes),
        )?,
        plain::<TerminalRead>(
            &TerminalRead::new(terminal()?).caller(caller()).lines(5),
            &TerminalRead::new(terminal()?),
        )?,
        plain::<TerminalAttach>(
            &TerminalAttach::new(terminal()?, 80, 24)
                .caller(caller())
                .display(display())
                .history(5)
                .observe()
                .resize_notifications()
                .resumed(),
            &TerminalAttach::new(terminal()?, 80, 24),
        )?,
        plain::<TerminalSend>(
            &TerminalSend::new(terminal()?, "x".to_owned())
                .caller(caller())
                .force()
                .only_if(IdleOrWorking::Idle)
                .timeout(seconds())
                .wait(),
            &TerminalSend::new(terminal()?, "x".to_owned()),
        )?,
        plain::<TerminalKeys>(
            &TerminalKeys::new(terminal()?, vec![KeyName::new("Enter")?]).caller(caller()),
            &TerminalKeys::new(terminal()?, vec![KeyName::new("Enter")?]),
        )?,
        plain::<TerminalInterrupt>(
            &TerminalInterrupt::new(terminal()?).timeout(seconds()),
            &TerminalInterrupt::new(terminal()?),
        )?,
        plain::<TerminalPut>(
            &TerminalPut::new(terminal()?, FileName::new("f"), vec![1]),
            &TerminalPut::new(terminal()?, FileName::new("f"), vec![1]),
        )?,
    ]);
    Ok(cases)
}

fn exec_with_every_setter(command: ExecCommand) -> Result<MachineExec, Box<dyn Error>> {
    Ok(MachineExec::new(command)
        .cwd(path())
        .env(environment())
        .input(vec![1])
        .on(machine()?)
        .timeout(seconds()))
}

/// How many variants `ExecCommand` has.
const EXEC_SLOTS: usize = 2;

/// The shape of each `ExecCommand`. A new variant stops this match compiling.
const fn exec_shape(command: &ExecCommand) -> Shape {
    match command {
        ExecCommand::Argv(_) => Shape {
            slot: 0,
            lacks: &["line"],
            extra: &["argv"],
        },
        ExecCommand::Line(_) => Shape {
            slot: 1,
            lacks: &["argv"],
            extra: &["line"],
        },
    }
}

/// How many variants `MachineTarget` has.
const TARGET_SLOTS: usize = 2;

/// The shape of each `MachineTarget`. A new variant stops this match compiling.
const fn target_shape(target: &MachineTarget) -> Shape {
    match target {
        MachineTarget::Ssh { .. } => Shape {
            slot: 0,
            lacks: &["via"],
            extra: &["target"],
        },
        MachineTarget::Via { .. } => Shape {
            slot: 1,
            lacks: &["target"],
            extra: &["name", "via"],
        },
    }
}

fn machine_cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let commands = [
        ExecCommand::Argv(argv()?),
        ExecCommand::Line("ls".to_owned()),
    ];
    let exec = commands
        .into_iter()
        .map(|command| {
            variant(
                &exec_with_every_setter(command.clone())?,
                &MachineExec::new(command.clone()),
                &exec_shape(&command),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ssh = |name: Option<MachineName>| MachineTarget::Ssh {
        target: "host".to_owned(),
        name,
    };
    let via = || -> Result<MachineTarget, Box<dyn Error>> {
        Ok(MachineTarget::Via {
            name: machine()?,
            command: "sh".to_owned(),
        })
    };
    let add_with = |full: MachineTarget, minimal: MachineTarget| {
        variant(
            &MachineAdd::new(full.clone())
                .session(SessionName::new("s")?)
                .temporary(),
            &MachineAdd::new(minimal),
            &target_shape(&full),
        )
    };
    let add = vec![
        add_with(ssh(Some(machine()?)), ssh(None))?,
        add_with(via()?, via()?)?,
    ];
    Ok(vec![
        exclusive::<MachineExec>(EXEC_SLOTS, exec),
        exclusive::<MachineAdd>(TARGET_SLOTS, add),
        plain::<MachineList>(&MachineList::new(), &MachineList::new())?,
        plain::<MachineConnect>(
            &MachineConnect::new(machine()?),
            &MachineConnect::new(machine()?),
        )?,
        plain::<MachineRename>(
            &MachineRename::new(machine()?, machine()?),
            &MachineRename::new(machine()?, machine()?),
        )?,
        plain::<MachineRm>(&MachineRm::new(machine()?), &MachineRm::new(machine()?))?,
    ])
}

fn file_and_doc_cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let edits = || {
        vec![Edit {
            old: "a".to_owned(),
            new: "b".to_owned(),
        }]
    };
    let doc = || -> Result<DocName, Box<dyn Error>> { Ok(DocName::new("d")?) };
    Ok(vec![
        plain::<FileRead>(
            &FileRead::new(path())
                .cwd(path())
                .limit(5)
                .offset(1)
                .on(machine()?),
            &FileRead::new(path()),
        )?,
        plain::<FileWrite>(
            &FileWrite::new(path(), vec![1]).cwd(path()).on(machine()?),
            &FileWrite::new(path(), vec![1]),
        )?,
        plain::<FileEdit>(
            &FileEdit::new(path(), edits()).cwd(path()).on(machine()?),
            &FileEdit::new(path(), edits()),
        )?,
        plain::<DocGet>(&DocGet::new(doc()?).on(machine()?), &DocGet::new(doc()?))?,
        plain::<DocSet>(
            &DocSet::new(doc()?, json!(1), DocVersion(1)).on(machine()?),
            &DocSet::new(doc()?, json!(1), DocVersion(1)),
        )?,
    ])
}

/// Every one of the SDK's 32 methods, each built through its own `Method` type.
fn cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let mut cases = server_cases()?;
    cases.extend(terminal_cases()?);
    cases.extend(machine_cases()?);
    cases.extend(file_and_doc_cases()?);
    Ok(cases)
}

/// A method as the fixture lists it.
struct FixtureMethod {
    name: String,
    /// Every param name.
    all: BTreeSet<String>,
    /// The names of the required params.
    required: BTreeSet<String>,
}

fn fixture_methods(fixture: &Value) -> Result<Vec<FixtureMethod>, Box<dyn Error>> {
    let methods = fixture["methods"]
        .as_array()
        .ok_or("the fixture has no methods array")?;
    methods
        .iter()
        .map(|method| {
            let name = method["name"].as_str().ok_or("a method has no name")?;
            let params = method["params"]
                .as_array()
                .ok_or("a method has no params")?;
            let mut all = BTreeSet::new();
            let mut required = BTreeSet::new();
            for param in params {
                let param_name = param["name"].as_str().ok_or("a param has no name")?;
                all.insert(param_name.to_owned());
                if param["required"] == json!(true) {
                    required.insert(param_name.to_owned());
                }
            }
            Ok(FixtureMethod {
                name: name.to_owned(),
                all,
                required,
            })
        })
        .collect()
}

#[test]
fn method_names_equal_the_fixture() -> TestResult {
    let fixture = fixture()?;
    let ours: Vec<&str> = cases()?.iter().map(|case| case.method).collect();
    let ours_set: BTreeSet<&str> = ours.iter().copied().collect();
    assert_eq!(ours.len(), ours_set.len(), "a method is listed twice");
    let theirs = fixture_methods(&fixture)?;
    let theirs_set: BTreeSet<&str> = theirs.iter().map(|method| method.name.as_str()).collect();
    assert_eq!(ours_set, theirs_set);
    assert_eq!(ours_set.len(), 32);
    Ok(())
}

#[test]
fn params_match_the_fixture_names() -> TestResult {
    let fixture = fixture()?;
    let theirs = fixture_methods(&fixture)?;
    let mut failures = Vec::new();
    for case in cases()? {
        let Some(FixtureMethod { all, required, .. }) =
            theirs.iter().find(|method| method.name == case.method)
        else {
            failures.push(format!("{}: not in the fixture", case.method));
            continue;
        };
        let slots: BTreeSet<usize> = case.variants.iter().map(|variant| variant.slot).collect();
        if slots != (0..case.slots).collect() {
            failures.push(format!(
                "{}: the cases build variants {slots:?}, expected every one below {}",
                case.method, case.slots
            ));
        }
        let mut union = BTreeSet::new();
        for variant in &case.variants {
            let expected_full: BTreeSet<String> = all
                .iter()
                .filter(|name| !variant.lacks.contains(&name.as_str()))
                .cloned()
                .collect();
            if variant.full != expected_full {
                failures.push(format!(
                    "{}: every setter sends {:?}, expected {expected_full:?}",
                    case.method, variant.full
                ));
            }
            let expected_minimal: BTreeSet<String> = required
                .iter()
                .cloned()
                .chain(variant.extra.iter().map(|name| (*name).to_owned()))
                .collect();
            if variant.minimal != expected_minimal {
                failures.push(format!(
                    "{}: the constructor alone sends {:?}, expected {expected_minimal:?}",
                    case.method, variant.minimal
                ));
            }
            union.extend(variant.full.iter().cloned());
        }
        if &union != all {
            failures.push(format!(
                "{}: the variants together send {union:?}, expected {all:?}",
                case.method
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

/// What the round-trip checks found.
#[derive(Default)]
struct Report {
    /// The schemas a check ran for.
    checked: BTreeSet<String>,
    /// How each check that failed differs.
    failures: Vec<String>,
}

/// Reads the instance of `schema` into `T`, writes it back, and records how it differs.
fn check_round_trip<T: DeserializeOwned + Serialize>(
    fixture: &Value,
    schema: &str,
    report: &mut Report,
) {
    report.checked.insert(schema.to_owned());
    let failures = &mut report.failures;
    let built = match instance(fixture, schema) {
        Ok(built) => built,
        Err(reason) => {
            failures.push(format!("{schema}: cannot build an instance: {reason}"));
            return;
        }
    };
    match serde_json::from_value::<T>(built.clone()) {
        Err(error) => failures.push(format!("{schema}: cannot read {built}: {error}")),
        Ok(value) => match serde_json::to_value(&value) {
            Err(error) => failures.push(format!("{schema}: cannot write it back: {error}")),
            Ok(written) if written != built => {
                failures.push(format!("{schema}: wrote {written}, expected {built}"));
            }
            Ok(_) => {}
        },
    }
}

#[test]
fn schema_types_round_trip_fixture_instances() -> TestResult {
    let fixture = fixture()?;
    let fixture = &fixture;
    let mut report = Report::default();
    let report = &mut report;
    check_round_trip::<AttachResult>(fixture, "AttachResult", report);
    check_round_trip::<Children>(fixture, "Children", report);
    check_round_trip::<CoreInfo>(fixture, "CoreInfo", report);
    check_round_trip::<DisplayFacts>(fixture, "Display", report);
    check_round_trip::<DocGetResult>(fixture, "DocGetResult", report);
    check_round_trip::<DocSetResult>(fixture, "DocSetResult", report);
    check_round_trip::<Edit>(fixture, "Edit", report);
    check_round_trip::<ExecResult>(fixture, "ExecResult", report);
    check_round_trip::<FileReadResult>(fixture, "FileReadResult", report);
    check_round_trip::<FileResult>(fixture, "FileResult", report);
    check_round_trip::<FirstMessage>(fixture, "FirstMessage", report);
    check_round_trip::<Foreground>(fixture, "Foreground", report);
    check_round_trip::<HelloResult>(fixture, "HelloResult", report);
    check_round_trip::<HolderInfo>(fixture, "HolderInfo", report);
    check_round_trip::<IdleOrWorking>(fixture, "IdleOrWorking", report);
    check_round_trip::<InitialSize>(fixture, "InitialSize", report);
    check_round_trip::<InterruptResult>(fixture, "InterruptResult", report);
    check_round_trip::<KeysResult>(fixture, "KeysResult", report);
    check_round_trip::<ListResult>(fixture, "ListResult", report);
    check_round_trip::<LoginSession>(fixture, "LoginSession", report);
    check_round_trip::<Machine>(fixture, "Machine", report);
    check_round_trip::<MachineListResult>(fixture, "MachineListResult", report);
    check_round_trip::<MachineView>(fixture, "MachineView", report);
    check_round_trip::<PutResult>(fixture, "PutResult", report);
    check_round_trip::<ReadResult>(fixture, "ReadResult", report);
    check_round_trip::<RemoveResult>(fixture, "RemoveResult", report);
    check_round_trip::<RunResult>(fixture, "RunResult", report);
    check_round_trip::<SendResult>(fixture, "SendResult", report);
    check_round_trip::<Status>(fixture, "Status", report);
    check_round_trip::<StatusResult>(fixture, "StatusResult", report);
    check_round_trip::<StopResult>(fixture, "StopResult", report);
    check_round_trip::<SubscribeResult>(fixture, "SubscribeResult", report);
    check_round_trip::<TemplateInfo>(fixture, "TemplateInfo", report);
    check_round_trip::<TemplatesResult>(fixture, "TemplatesResult", report);
    check_round_trip::<Terminal>(fixture, "Terminal", report);
    check_round_trip::<WaitResult>(fixture, "WaitResult", report);
    let named: BTreeSet<String> = fixture["components"]["schemas"]
        .as_object()
        .ok_or("the fixture has no component schemas")?
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        report.checked, named,
        "the round-trip checks and the fixture's schemas differ"
    );
    assert!(report.failures.is_empty(), "{}", report.failures.join("\n"));
    Ok(())
}

/// Accepts only a method whose reply type is `Output`, and returns its wire name.
const fn reply_of<Output, M: Method<Output = Output>>() -> &'static str {
    M::NAME
}

#[test]
fn every_method_reads_its_reply_into_the_schema_type() {
    let names = [
        reply_of::<HelloResult, HelloParams>(),
        reply_of::<SubscribeResult, EventsSubscribe>(),
        reply_of::<RunResult, TerminalRun>(),
        reply_of::<ListResult, TerminalList>(),
        reply_of::<Terminal, TerminalGet>(),
        reply_of::<Terminal, TerminalKill>(),
        reply_of::<RemoveResult, TerminalRm>(),
        reply_of::<Terminal, TerminalKeep>(),
        reply_of::<Terminal, TerminalLabel>(),
        reply_of::<ReadResult, TerminalRead>(),
        reply_of::<AttachResult, TerminalAttach>(),
        reply_of::<SendResult, TerminalSend>(),
        reply_of::<WaitResult, TerminalWait>(),
        reply_of::<KeysResult, TerminalKeys>(),
        reply_of::<InterruptResult, TerminalInterrupt>(),
        reply_of::<PutResult, TerminalPut>(),
        reply_of::<StatusResult, ServerStatus>(),
        reply_of::<StopResult, ServerStop>(),
        reply_of::<StopResult, ServerRestart>(),
        reply_of::<TemplatesResult, TemplatesList>(),
        reply_of::<ExecResult, MachineExec>(),
        reply_of::<Machine, MachineAdd>(),
        reply_of::<MachineListResult, MachineList>(),
        reply_of::<MachineView, MachineConnect>(),
        reply_of::<Machine, MachineRename>(),
        reply_of::<RemoveResult, MachineRm>(),
        reply_of::<DocGetResult, DocGet>(),
        reply_of::<DocSetResult, DocSet>(),
        reply_of::<FileReadResult, FileRead>(),
        reply_of::<FileResult, FileWrite>(),
        reply_of::<FileResult, FileEdit>(),
        reply_of::<Value, RpcDiscover>(),
    ];
    assert_eq!(names.len(), 32);
}
