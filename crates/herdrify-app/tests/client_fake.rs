//! `Client` and `HerdrApp` driven through a scripted transport written outside the crate.
//!
//! The fake transport answers each run with the next output it was given and records the argv it
//! was asked to run. The tests use the public API only. The outputs they script are what
//! herdr-app 0.1.17 was recorded printing, except the ones a comment marks as made up.
//!
//! Responsibilities:
//!
//! - the fake transport itself: replies in order, argvs recorded
//! - each command output shape, and the listing, status and command list, decoded from a reply
//! - each way a run fails, with the exact text of the error it becomes
//! - the version check: which versions it accepts, and that an entrypoint makes it once
//! - `raw`, which returns fields no type names and refuses output that is not JSON
//! - that a value a call refuses runs nothing
//!
//! Non-responsibilities:
//!
//! - the sorting of a run into success or failure in isolation, which the protocol unit tests cover
//! - each command's arguments, which the command table test and the command unit tests cover

#[path = "support/fake_transport.rs"]
pub mod fake_transport;

use std::fmt::Display;

use fake_transport::{FakeTransport, TestError, ended, success};
use herdrify_app::protocol::{
    Ack, Direction, Exit, Invocation, Place, RunOutput, SplitDirection, TabNext,
};
use herdrify_app::{Client, Error, HerdrApp, Transport, TransportError};
use serde_json::{Value, json};

type TestResult = Result<(), TestError>;

/// What `herdr-app --version` printed.
const VERSION_0_1_17: &str = "herdr-app 0.1.17\n";

/// What `herdr-app do <command>` printed for a command that has nothing to return.
const ACK: &str = r#"{"ok":true}"#;

/// What `herdr-app ls` printed with one space, one terminal that has a sub agent, and a space
/// closed recently.
const LS: &str = r#"{
  "spaces": [
    {
      "id": "sp791801470e39e979233f19213e2f5b3b",
      "name": "hzprobe",
      "folder": "/Users/hiraq",
      "machine": null,
      "active": true,
      "tabs": [
        {
          "id": "tbac537ebad46b69af5db7cafb14f4a078",
          "name": "1",
          "active": true,
          "settled": false,
          "terminals": [
            {
              "terminal": "t1",
              "pane": "p13436eac9efd61a6971094912e82edd7",
              "status": null,
              "app": null,
              "name": null,
              "sub_agents": [
                "t2"
              ]
            }
          ]
        }
      ]
    }
  ],
  "recent": [
    {
      "name": "gone",
      "folder": "/Users/hiraq"
    }
  ]
}
"#;

/// What `herdr-app status` printed while a floating node, a broken mod and three overlays were
/// there.
const STATUS: &str = r#"{
  "zoom": 1.0,
  "font_size": 22.86,
  "ui_font_size": 13.0,
  "space": "sp791801470e39e979233f19213e2f5b3b",
  "tab": "tbac537ebad46b69af5db7cafb14f4a078",
  "focused_pane": "p13436eac9efd61a6971094912e82edd7",
  "layout": "win98",
  "agent": null,
  "floats": {
    "clippy": {
      "edge": "left",
      "align": "end",
      "vertical": true,
      "dragging": false,
      "hovered": false
    }
  },
  "overlays": [
    "frame-welcome",
    "broken",
    "win98"
  ],
  "errors": [
    "mod broken: Unexpected token '('\n    at /private/tmp/hzw/mods/broken/main.js?v=8:1:1\n"
  ]
}
"#;

/// A `status` with one more key, as a later herdr-app release may print. The key is made up.
const STATUS_WITH_EXTRA: &str = r#"{
  "zoom": 1.0,
  "font_size": 22.86,
  "ui_font_size": 13.0,
  "space": "sp791801470e39e979233f19213e2f5b3b",
  "tab": "tbac537ebad46b69af5db7cafb14f4a078",
  "focused_pane": "p13436eac9efd61a6971094912e82edd7",
  "layout": "win98",
  "agent": null,
  "floats": {},
  "overlays": [],
  "errors": [],
  "battery": {"percent": 80}
}
"#;

/// The `space.select` and `space.next` entries of what `herdr-app commands` printed, in that
/// order, with each argument on one line. The real listing has other entries around them.
const COMMANDS: &str = r#"[
  {
    "name": "space.select",
    "args": [
      {"name": "index", "required": false},
      {"name": "id", "required": false}
    ],
    "about": "Show a space (by position or id)."
  },
  {
    "name": "space.next",
    "args": [],
    "about": "Show the next space."
  }
]
"#;

/// What a refused `tab.new` printed on stdout, on one line. The recorded stdout is the same object
/// pretty-printed.
const REFUSED: &str = r#"{"error":"tab.new focus is true or false"}"#;

/// What `herdr-app` printed on stderr when no window answered.
const NO_WINDOW: &str = "herdr-app: no Herdr window at /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)\n";

/// What `herdr-app` printed on stderr when the arguments were not JSON.
const NOT_JSON: &str = "herdr-app: args are not JSON: key must be a string at line 1 column 2\n";

/// The argv of the one run an app makes while it connects.
fn version_run() -> Vec<String> {
    vec!["--version".to_owned()]
}

/// A client over a fake that answers with `outputs`, and the fake, to read what ran.
fn client_over(
    outputs: impl IntoIterator<Item = RunOutput>,
) -> (Client<FakeTransport>, FakeTransport) {
    let fake = FakeTransport::new(outputs);
    (Client::new(fake.clone()), fake)
}

/// An app over a fake that answers its version check and then `outputs`, and the fake.
async fn app_over(
    outputs: impl IntoIterator<Item = RunOutput>,
) -> Result<(HerdrApp<FakeTransport>, FakeTransport), TestError> {
    let fake = FakeTransport::new(std::iter::once(success(VERSION_0_1_17)).chain(outputs));
    let app = HerdrApp::from_client(Client::new(fake.clone())).await?;
    Ok((app, fake))
}

/// The JSON object the last run passed to `herdr-app do <name>`, or an error when the last run
/// was not `do <name> <json>`.
fn sent_arguments(fake: &FakeTransport, name: &str) -> Result<Value, TestError> {
    let argvs = fake.argvs();
    let last = argvs.last().ok_or("nothing ran")?;
    match last.as_slice() {
        [verb, command, json] if verb == "do" && command == name => Ok(serde_json::from_str(json)?),
        _ => Err(format!("the last run was not `do {name} <json>`: {last:?}").into()),
    }
}

/// The text of an optional identifier.
fn text<D: Display>(value: Option<&D>) -> Option<String> {
    value.map(ToString::to_string)
}

/// The error of `ls` over a fake whose one output is `output`.
async fn ls_error(output: RunOutput) -> Result<Error, TestError> {
    let (client, _) = client_over([output]);
    client
        .ls()
        .await
        .err()
        .ok_or_else(|| "the call succeeded".into())
}

#[tokio::test]
async fn fake_transport_replays_in_order() -> TestResult {
    // The second output is made up.
    let fake = FakeTransport::new([success(VERSION_0_1_17), success("{\"spaces\":[]}")]);

    let first = fake.run(&Invocation::version()).await?;
    let second = fake.clone().run(&Invocation::ls()).await?;

    assert_eq!(first.stdout, VERSION_0_1_17.as_bytes());
    assert_eq!(second.stdout, b"{\"spaces\":[]}");
    assert_eq!(
        fake.argvs(),
        [vec!["--version".to_owned()], vec!["ls".to_owned()]],
    );
    Ok(())
}

#[tokio::test]
async fn every_output_shape_decodes() -> TestResult {
    let (app, fake) = app_over([
        success(
            r#"{"space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tb2a277d2ac143c65c063d5cad51bd15b6","terminal":"t1"}"#,
        ),
        success(
            r#"{"space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tbb63e1218cf7f2f97c1c1e5635eabaab7","terminal":"t3"}"#,
        ),
        success(
            r#"{"pane":"pe0e9b9419f5307e4ff1787b9baa09549","space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tb5d44f73fbf622bc82a2047fce2db12ba"}"#,
        ),
        success(r#"{"pane":"p1c355abaaadefb36dc823c35187f482f","terminal":"t4"}"#),
        success(r#"{"pane":"pfb28ec149feaca1d490376841d34d26a"}"#),
        success(r#"{"pane":"p5a7bda7ce45003740be823ba2432f3a8","terminal":"t5"}"#),
        success(
            r#"{"space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tb7034c52158c180a628e5128a41f43af0","terminal":"t7"}"#,
        ),
        success(
            r#"{"pane":"p2710fecc3619dcb2414e51346abb1878","pending":true,"tab":"tb009b7157e3a91ebcf22a6548ec03f8d7","terminal":"t2"}"#,
        ),
        success(r#"{"pane":"p2710fecc3619dcb2414e51346abb1878"}"#),
        success(r#"{"pending":true,"terminal":"t2"}"#),
        success(ACK),
        // An acknowledgement that carries more than `ok`. This reply is made up.
        success(r#"{"extra":[1,2],"ok":true}"#),
        // No success of `pane.resize` was recorded, so this reply is made up too: any JSON value
        // comes back as it was.
        success(r#"{"anything":[1,2]}"#),
        success(LS),
        success(STATUS),
        success(COMMANDS),
    ])
    .await?;

    // space.new: Created
    let created = app.spaces().new_space().name("hzprobe").await?;
    assert_eq!(created.space.as_str(), "sp6c039a6d414cc352015b391a2d6279b7");
    assert_eq!(created.tab.as_str(), "tb2a277d2ac143c65c063d5cad51bd15b6");
    assert_eq!(created.terminal.as_str(), "t1");
    assert_eq!(
        sent_arguments(&fake, "space.new")?,
        json!({"name": "hzprobe"}),
    );

    // tab.new: TabMade, a shell tab with a terminal and a mod tab with a pane
    let shell_tab = app.tabs().new_tab().name("tb").await?;
    assert_eq!(shell_tab.tab.as_str(), "tbb63e1218cf7f2f97c1c1e5635eabaab7");
    assert_eq!(text(shell_tab.terminal.as_ref()), Some("t3".to_owned()));
    assert_eq!(text(shell_tab.pane.as_ref()), None);
    let mod_tab = app.tabs().new_mod_tab("files").await?;
    assert_eq!(mod_tab.space.as_str(), "sp6c039a6d414cc352015b391a2d6279b7");
    assert_eq!(text(mod_tab.terminal.as_ref()), None);
    assert_eq!(
        text(mod_tab.pane.as_ref()),
        Some("pe0e9b9419f5307e4ff1787b9baa09549".to_owned()),
    );

    // pane.split: Split, with a terminal and without
    let split = app.panes().split(SplitDirection::Right).await?;
    assert_eq!(split.pane.as_str(), "p1c355abaaadefb36dc823c35187f482f");
    assert_eq!(text(split.terminal.as_ref()), Some("t4".to_owned()));
    let split_mod = app.panes().split_mod(SplitDirection::Down, "files").await?;
    assert_eq!(split_mod.pane.as_str(), "pfb28ec149feaca1d490376841d34d26a");
    assert_eq!(text(split_mod.terminal.as_ref()), None);

    check_terminals(&app).await?;
    check_replies(&app).await?;
    check_window_views(&app).await
}

/// `terminal.open`, `terminal.keep` and `terminal.park` decode their recorded replies.
async fn check_terminals(app: &HerdrApp<FakeTransport>) -> TestResult {
    // terminal.open: Opened, beside a pane and in a new tab
    let beside = app.terminals().open().place(Place::Down).await?;
    assert_eq!(beside.terminal.as_str(), "t5");
    assert_eq!(
        text(beside.pane.as_ref()),
        Some("p5a7bda7ce45003740be823ba2432f3a8".to_owned()),
    );
    assert_eq!(text(beside.tab.as_ref()), None);
    let in_tab = app.terminals().open().await?;
    assert_eq!(in_tab.terminal.as_str(), "t7");
    assert_eq!(
        text(in_tab.tab.as_ref()),
        Some("tb7034c52158c180a628e5128a41f43af0".to_owned()),
    );
    assert_eq!(text(in_tab.pane.as_ref()), None);

    // terminal.keep: Kept, a first keep with all four fields and a repeat with only the pane
    let first_keep = app.terminals().keep("t2").place(Place::Right).await?;
    assert_eq!(
        first_keep.pane.as_str(),
        "p2710fecc3619dcb2414e51346abb1878"
    );
    assert_eq!(
        text(first_keep.tab.as_ref()),
        Some("tb009b7157e3a91ebcf22a6548ec03f8d7".to_owned()),
    );
    assert_eq!(text(first_keep.terminal.as_ref()), Some("t2".to_owned()));
    assert_eq!(first_keep.pending, Some(true));
    let repeat_keep = app.terminals().keep("t2").await?;
    assert_eq!(
        repeat_keep.pane.as_str(),
        "p2710fecc3619dcb2414e51346abb1878"
    );
    assert_eq!(text(repeat_keep.tab.as_ref()), None);
    assert_eq!(text(repeat_keep.terminal.as_ref()), None);
    assert_eq!(repeat_keep.pending, None);

    // terminal.park: Parked
    let parked = app.terminals().park("t2").await?;
    assert_eq!(parked.terminal.as_str(), "t2");
    assert!(parked.pending);
    Ok(())
}

/// An acknowledgement, one that carries more than `ok`, and a reply with no recorded shape.
async fn check_replies(app: &HerdrApp<FakeTransport>) -> TestResult {
    // an acknowledgement, and one that carries more than `ok`
    assert_eq!(app.spaces().next().await?, Ack);
    assert_eq!(app.tabs().next().await?, Ack);

    // a command with no recorded shape
    let raw = app.panes().resize(Direction::Left).await?;
    assert_eq!(raw.0, json!({"anything": [1, 2]}));
    Ok(())
}

/// `ls`, `status` and `commands` decode their recorded replies.
async fn check_window_views(app: &HerdrApp<FakeTransport>) -> TestResult {
    // ls
    let listing = app.ls().await?;
    assert_eq!(listing.spaces.len(), 1);
    let space = &listing.spaces[0];
    assert_eq!(space.id.as_str(), "sp791801470e39e979233f19213e2f5b3b");
    assert_eq!(space.name, "hzprobe");
    assert_eq!(space.folder, "/Users/hiraq");
    assert_eq!(text(space.machine.as_ref()), None);
    assert!(space.active);
    let terminal = &space.tabs[0].terminals[0];
    assert_eq!(terminal.terminal.as_str(), "t1");
    assert_eq!(
        text(terminal.pane.as_ref()),
        Some("p13436eac9efd61a6971094912e82edd7".to_owned()),
    );
    assert_eq!(terminal.status, None);
    assert_eq!(
        terminal
            .sub_agents
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["t2"],
    );
    assert_eq!(listing.recent.len(), 1);
    assert_eq!(listing.recent[0].name, "gone");

    // status
    let status = app.status().await?;
    assert_eq!(status.layout.as_str(), "win98");
    assert!((status.font_size - 22.86).abs() < f64::EPSILON);
    assert_eq!(
        text(status.space.as_ref()),
        Some("sp791801470e39e979233f19213e2f5b3b".to_owned()),
    );
    assert_eq!(text(status.agent.as_ref()), None);
    assert_eq!(status.floats.len(), 1);
    assert_eq!(status.overlays.len(), 3);
    assert_eq!(status.errors.len(), 1);

    // commands
    let commands = app.commands().await?;
    assert_eq!(
        commands
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>(),
        ["space.select", "space.next"],
    );
    assert_eq!(commands[0].args.len(), 2);
    assert!(!commands[0].args[0].required);
    assert_eq!(commands[1].about, "Show the next space.");
    Ok(())
}

#[tokio::test]
async fn every_failure_maps_to_its_error() -> TestResult {
    // Refused: exit 1 and an error object on stdout
    let (app, _) = app_over([ended(Exit::Code(1), REFUSED, "")]).await?;
    let refused = app
        .tabs()
        .new_tab()
        .await
        .err()
        .ok_or("the call succeeded")?;
    assert!(matches!(
        &refused,
        Error::Refused { invocation, message }
            if invocation == "do tab.new" && message == "tab.new focus is true or false"
    ));
    assert_eq!(
        refused.to_string(),
        "herdr-app do tab.new refused: tab.new focus is true or false",
    );
    // The refusal text was recorded for `status` and `app.quit`. Printed for `ls` it is made up.
    let closed = ls_error(ended(
        Exit::Code(1),
        r#"{"error":"the window did not answer"}"#,
        "",
    ))
    .await?;
    assert_eq!(
        closed.to_string(),
        "herdr-app ls refused: the window did not answer",
    );

    // NoWindow and Cli: exit 2, told apart by the text on stderr
    let no_window = ls_error(ended(Exit::Code(2), "", NO_WINDOW)).await?;
    assert!(matches!(&no_window, Error::NoWindow { .. }));
    assert_eq!(
        no_window.to_string(),
        "herdr-app has no window to answer: no Herdr window at /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)",
    );
    let cli = ls_error(ended(Exit::Code(2), "", NOT_JSON)).await?;
    assert!(matches!(&cli, Error::Cli { .. }));
    assert_eq!(
        cli.to_string(),
        "herdr-app did not accept the invocation: args are not JSON: key must be a string at line 1 column 2",
    );

    check_unexpected_endings().await?;
    check_decode_failures().await?;
    check_start_and_version_failures().await
}

/// Any other ending, by exit code and by signal, with and without stderr.
async fn check_unexpected_endings() -> TestResult {
    // Every ending in this function is made up: none of them was recorded.
    let code = ls_error(ended(Exit::Code(3), "", "boom\n")).await?;
    assert!(matches!(
        &code,
        Error::Unexpected {
            exit: Exit::Code(3),
            ..
        }
    ));
    assert_eq!(code.to_string(), "herdr-app exited with code 3: boom");
    let silent = ls_error(ended(Exit::Code(1), "not json", "")).await?;
    assert!(matches!(
        &silent,
        Error::Unexpected {
            exit: Exit::Code(1),
            ..
        }
    ));
    assert_eq!(
        silent.to_string(),
        "herdr-app exited with code 1: (no output)",
    );
    let signal = ls_error(ended(Exit::Signal, "", "")).await?;
    assert!(matches!(
        &signal,
        Error::Unexpected {
            exit: Exit::Signal,
            ..
        }
    ));
    assert_eq!(
        signal.to_string(),
        "herdr-app was ended by a signal: (no output)",
    );
    let signal_text = ls_error(ended(Exit::Signal, "", "killed\n")).await?;
    assert_eq!(
        signal_text.to_string(),
        "herdr-app was ended by a signal: killed",
    );
    Ok(())
}

/// A successful run whose stdout is not the shape the call reads.
async fn check_decode_failures() -> TestResult {
    // Every output in this function is made up: each is a successful run with the wrong shape.
    let missing = ls_error(success(r#"{"recent":[]}"#)).await?;
    assert!(matches!(&missing, Error::Decode { invocation, .. } if invocation == "ls"));
    assert_eq!(
        missing.to_string(),
        "cannot read the output of herdr-app ls: missing field `spaces` at line 1 column 13",
    );
    let empty = ls_error(success("")).await?;
    assert_eq!(
        empty.to_string(),
        "cannot read the output of herdr-app ls: EOF while parsing a value at line 1 column 0",
    );
    let (app, _) = app_over([success(r#"{"nothing":1}"#)]).await?;
    let not_ack = app
        .spaces()
        .next()
        .await
        .err()
        .ok_or("the call succeeded")?;
    assert_eq!(
        not_ack.to_string(),
        r#"cannot read the output of herdr-app do space.next: expected "ok": true"#,
    );

    Ok(())
}

/// A `--version` that prints no version, and a program that could not be started.
async fn check_start_and_version_failures() -> TestResult {
    // The output is made up: a `--version` that prints no patch number.
    let (client, _) = client_over([success("herdr-app 0.1")]);
    let unreadable = client.version().await.err().ok_or("the call succeeded")?;
    assert!(matches!(&unreadable, Error::InvalidVersion(_)));
    assert_eq!(
        unreadable.to_string(),
        r#"cannot read a herdr-app version from "herdr-app 0.1""#,
    );

    // Transport: a program that could not be started passes through unchanged
    let (client, _) = client_over([]);
    let unstarted = client.ls().await.err().ok_or("the call succeeded")?;
    assert!(matches!(
        &unstarted,
        Error::Transport(TransportError::NotFound { .. })
    ));
    assert_eq!(
        unstarted.to_string(),
        "cannot start `fake herdr-app: no output was scripted for this run`: it was not found",
    );
    Ok(())
}

#[tokio::test]
async fn version_checks() -> TestResult {
    // Only 0.1.17 was recorded. The other version strings are made up, to reach each side of the
    // 0.1.16 floor.
    for (printed, accepted) in [
        ("herdr-app 0.1.16\n", "0.1.16"),
        ("herdr-app 0.1.17\n", "0.1.17"),
        ("herdr-app 0.2.0\n", "0.2.0"),
    ] {
        let (client, _) = client_over([success(printed)]);
        assert_eq!(client.check_version().await?.to_string(), accepted);
    }

    for (printed, refused) in [
        ("herdr-app 0.1.15\n", "0.1.15"),
        ("herdr-app 0.1.16-rc.1\n", "0.1.16-rc.1"),
    ] {
        let (client, _) = client_over([success(printed)]);
        let error = client
            .check_version()
            .await
            .err()
            .ok_or("an old version was accepted")?;
        assert!(matches!(&error, Error::UnsupportedVersion { .. }));
        assert_eq!(
            error.to_string(),
            format!(
                "herdr-app {refused} is older than 0.1.16, the oldest version this crate supports"
            ),
        );
    }

    // `version` reports what herdr-app prints and leaves the judging to `check_version`
    let (client, _) = client_over([success("herdr-app 0.1.15\n")]);
    assert_eq!(client.version().await?.to_string(), "0.1.15");
    Ok(())
}

#[tokio::test]
async fn raw_reads_unnamed_fields() -> TestResult {
    let (client, fake) = client_over([success(STATUS_WITH_EXTRA), success(STATUS_WITH_EXTRA)]);

    let value = client.raw(Invocation::status()).await?;
    assert_eq!(value["layout"], "win98");
    assert_eq!(value["battery"], json!({"percent": 80}));

    // the typed call reads the same output and leaves the field it does not name
    let status = client.status().await?;
    assert_eq!(status.layout.as_str(), "win98");
    assert_eq!(
        fake.argvs(),
        [vec!["status".to_owned()], vec!["status".to_owned()]],
    );
    Ok(())
}

#[tokio::test]
async fn raw_version_is_decode() -> TestResult {
    let (client, _) = client_over([success(VERSION_0_1_17)]);

    let error = client
        .raw(Invocation::version())
        .await
        .err()
        .ok_or("the call succeeded")?;

    assert!(matches!(&error, Error::Decode { invocation, .. } if invocation == "--version"));
    assert_eq!(
        error.to_string(),
        "cannot read the output of herdr-app --version: expected value at line 1 column 1",
    );
    Ok(())
}

#[tokio::test]
async fn herdr_app_from_client_checks_once() -> TestResult {
    let (app, fake) = app_over([success(ACK), success(ACK), success(ACK)]).await?;

    assert_eq!(app.version().to_string(), "0.1.17");
    assert_eq!(fake.argvs(), [version_run()]);

    app.spaces().next().await?;
    app.clone().tabs().next().await?;
    app.client().run(TabNext::new()).await?;

    let argvs = fake.argvs();
    assert_eq!(argvs.len(), 4);
    assert_eq!(
        argvs.iter().filter(|argv| **argv == version_run()).count(),
        1,
        "later calls did not check the version again: {argvs:?}",
    );

    // an older version is refused when the app is built; the version string is made up
    let (client, _) = client_over([success("herdr-app 0.1.15\n")]);
    let refused = HerdrApp::from_client(client)
        .await
        .err()
        .ok_or("an old version was accepted")?;
    assert!(matches!(refused, Error::UnsupportedVersion { .. }));
    Ok(())
}

#[tokio::test]
async fn invalid_plain_values_run_nothing() -> TestResult {
    let (app, fake) = app_over([success(ACK)]).await?;
    assert_eq!(fake.argvs(), [version_run()]);

    let empty_name = app
        .spaces()
        .rename("")
        .await
        .err()
        .ok_or("an empty name ran")?;
    assert!(matches!(
        &empty_name,
        Error::InvalidArgument { argument, .. } if argument == "name"
    ));
    assert_eq!(
        empty_name.to_string(),
        "invalid name: a space name cannot be empty",
    );

    let zero_index = app
        .spaces()
        .select_index(0)
        .await
        .err()
        .ok_or("index 0 ran")?;
    assert!(matches!(
        &zero_index,
        Error::InvalidArgument { argument, .. } if argument == "index"
    ));
    assert_eq!(
        zero_index.to_string(),
        "invalid index: 0 is out of range: positions start at 1",
    );

    let empty_optional = app
        .spaces()
        .new_space()
        .name("")
        .await
        .err()
        .ok_or("an empty name ran")?;
    assert_eq!(
        empty_optional.to_string(),
        "invalid name: a space name cannot be empty",
    );

    assert_eq!(
        fake.argvs(),
        [version_run()],
        "a refused value ran herdr-app"
    );

    // the one scripted output is still there, and a valid call takes it
    app.spaces().rename("api").await?;
    assert_eq!(fake.argvs().len(), 2);
    assert_eq!(
        sent_arguments(&fake, "space.rename")?,
        json!({"name": "api"}),
    );
    Ok(())
}
