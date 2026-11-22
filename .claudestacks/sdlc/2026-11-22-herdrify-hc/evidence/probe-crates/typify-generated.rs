#[doc = "`AttachResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct AttachResult {
    #[doc = "The entire attach path accepts AUTOMATIC_INPUT (kind 6). Absent means use DATA."]
    pub automatic_input: bool,
    #[doc = "Stream channel: first paint, then live bytes. Send keys as DATA on it;\nRESIZE, FOCUS, FACTS, CREDIT and CLOSE frames control it."]
    pub channel: u32,
    #[doc = "True only when the entire path sends RESIZE before replacement paints.\nFalse or absent means the observer must track size changes separately."]
    pub resize_notifications: bool,
    pub terminal: Terminal,
}
#[doc = "`Children`"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum Children {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "all")]
    All,
}
impl ::std::fmt::Display for Children {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Any => f.write_str("any"),
            Self::All => f.write_str("all"),
        }
    }
}
impl ::std::str::FromStr for Children {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "any" => Ok(Self::Any),
            "all" => Ok(Self::All),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for Children {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Children {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "`CoreInfo`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct CoreInfo {
    #[doc = "Permanent random id, reset only when state is wiped."]
    pub id: ::std::string::String,
    #[doc = "Display name, the hostname by default."]
    pub name: ::std::string::String,
}
#[doc = "What a viewer's own terminal looks like, reported by the client."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, Default)]
pub struct Display {
    #[doc = "Default background as `#rrggbb`."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub bg: ::std::option::Option<::std::string::String>,
    #[doc = "Cell size in pixels: `[width, height]`."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub cell: ::std::option::Option<[u32; 2usize]>,
    #[doc = "Cursor color as `#rrggbb`."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub cursor: ::std::option::Option<::std::string::String>,
    #[doc = "Default foreground as `#rrggbb`."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub fg: ::std::option::Option<::std::string::String>,
}
#[doc = "`DocGetResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct DocGetResult {
    pub value: ::serde_json::Value,
    #[doc = "Zero when never written. Deletions retain their version."]
    pub version: u64,
}
#[doc = "`DocSetResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct DocSetResult {
    pub version: u64,
}
#[doc = "`Edit`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Edit {
    pub new: ::std::string::String,
    #[doc = "Exact text, found exactly once in the file as it was."]
    pub old: ::std::string::String,
}
#[doc = "`ExecResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ExecResult {
    #[doc = "Its exit code; null when a signal ended it."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub code: ::std::option::Option<i32>,
    #[doc = "How long it ran, in milliseconds."]
    pub ms: u64,
    #[doc = "The signal that ended it (Unix)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub signal: ::std::option::Option<i32>,
    #[doc = "What it wrote, base64: the first 1 MB."]
    pub stderr: ::std::string::String,
    #[doc = "What it wrote, base64: the first 8 MB."]
    pub stdout: ::std::string::String,
    #[doc = "It ran out of time and was ended: the output is what came so far."]
    pub timed_out: bool,
    #[doc = "It wrote more than was kept (it still ran to its end)."]
    pub truncated: bool,
}
#[doc = "`FileReadResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct FileReadResult {
    #[doc = "The first and last line given (last < first: none)."]
    pub first: u32,
    pub last: u32,
    #[doc = "Lines in the file."]
    pub lines: u32,
    #[doc = "The lines as they are in the file, line ends kept."]
    pub text: ::std::string::String,
}
#[doc = "`FileResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct FileResult {
    pub bytes: u32,
    #[doc = "The file written, absolute."]
    pub path: ::std::string::String,
}
#[doc = "`FirstMessage`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct FirstMessage {
    #[doc = "Why it was not sent."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub reason: ::std::option::Option<::std::string::String>,
    pub sent: bool,
    #[doc = "Whether the agent started working on it (see terminal.send)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub started: ::std::option::Option<bool>,
    #[doc = "The turn when it went in (see terminal.send)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub turn: ::std::option::Option<u64>,
}
#[doc = "`Foreground`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Foreground {
    pub argv: ::std::vec::Vec<::std::string::String>,
    #[doc = "Its working directory now (resolved by the OS)."]
    pub cwd: ::std::string::String,
    #[doc = "Its name as the OS has it (`node`, `vim`, `sh`)."]
    pub name: ::std::string::String,
    #[doc = "The foreground process group's leader."]
    pub pid: i32,
}
#[doc = "`HelloResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct HelloResult {
    pub core: CoreInfo,
    pub features: ::std::vec::Vec<::std::string::String>,
    pub framed: bool,
    pub generation: u32,
    #[doc = "True: systemd lingering is enabled; false: programs stop at last logout.\nNull: unsupported, not applicable or unknown."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub keeps_running_after_logout: ::std::option::Option<bool>,
    #[doc = "OS login inherited by this core. Absent when unavailable or unsupported."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub login_session: ::std::option::Option<LoginSession>,
    #[doc = "Account for the administrator's loginctl enable-linger command."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub logout_linger_user: ::std::option::Option<::std::string::String>,
    pub methods: ::std::vec::Vec<::std::string::String>,
    pub version: ::std::string::String,
}
#[doc = "`HolderInfo`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct HolderInfo {
    #[doc = "Same build as the core: takes new terminals. Others drain."]
    pub current: bool,
    pub instance: ::std::string::String,
    pub pid: u32,
    #[doc = "Live terminals it runs."]
    pub terminals: u32,
}
#[doc = "`IdleOrWorking`"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum IdleOrWorking {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "working")]
    Working,
}
impl ::std::fmt::Display for IdleOrWorking {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Idle => f.write_str("idle"),
            Self::Working => f.write_str("working"),
        }
    }
}
impl ::std::str::FromStr for IdleOrWorking {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "idle" => Ok(Self::Idle),
            "working" => Ok(Self::Working),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for IdleOrWorking {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for IdleOrWorking {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "`InitialSize`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct InitialSize {
    #[doc = "Initial terminal width in columns (at least 1)."]
    pub cols: ::std::num::NonZeroU16,
    #[doc = "Initial terminal height in rows (at least 1)."]
    pub rows: ::std::num::NonZeroU16,
}
#[doc = "`InterruptResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct InterruptResult {
    #[doc = "The key sent: esc for programs that report status, else ctrl+c."]
    pub key: ::std::string::String,
    #[doc = "stopped (left working, or a shell got its prompt back), exited,\nstill_working (no change in time), or unknown (it doesn't report status)."]
    pub outcome: ::std::string::String,
}
#[doc = "`KeysResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct KeysResult {
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status: ::std::option::Option<::std::string::String>,
    pub turn: u64,
}
#[doc = "`ListResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ListResult {
    #[doc = "Finished terminals left out (see `all`)."]
    pub hidden_finished: u32,
    #[doc = "Machines added here and their lines: one that is not connected is\nlisted here, not waited on."]
    pub machines: ::std::vec::Vec<MachineView>,
    #[doc = "This machine's terminals, then each connected machine's (`machine/t7`)."]
    pub terminals: ::std::vec::Vec<Terminal>,
}
#[doc = "The OS login a process inherited, rather than text restored from its environment."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct LoginSession {
    #[doc = "Unknown provenance never triggers automatic replacement."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub gui: ::std::option::Option<bool>,
    pub id: u32,
    pub user: u32,
}
#[doc = "`Machine`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Machine {
    #[doc = "What that core calls itself (its hostname)."]
    pub host: ::std::string::String,
    #[doc = "The other core's permanent id."]
    pub id: ::std::string::String,
    #[doc = "Last observed systemd logout protection; null for old or unsupported cores."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub keeps_running_after_logout: ::std::option::Option<bool>,
    #[doc = "Remote account for loginctl enable-linger, when applicable."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub logout_linger_user: ::std::option::Option<::std::string::String>,
    #[doc = "What this core calls it: `rohan` in `rohan/t7`."]
    pub name: ::std::string::String,
    #[doc = "The hc session there (null: its default)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub session: ::std::option::Option<::std::string::String>,
    #[doc = "How it is reached: an ssh target, or (`via`) a command."]
    pub target: ::std::string::String,
    #[doc = "Short-lived (a sandbox for a task): clients may show it apart from the\nmachines that stay. hc treats it like any other."]
    #[serde(default)]
    pub temporary: bool,
    #[doc = "Its hc version when last seen."]
    pub version: ::std::string::String,
    #[doc = "`target` is a command that runs a shell line there, given as its last\nargument, like ssh (a provider's own CLI, ssh with its options)."]
    #[serde(default)]
    pub via: bool,
}
#[doc = "`MachineListResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct MachineListResult {
    pub machines: ::std::vec::Vec<MachineView>,
}
#[doc = "A machine as `machine ls` and `ls` show it: its profile and its line now."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct MachineView {
    #[doc = "What that core calls itself (its hostname)."]
    pub host: ::std::string::String,
    #[doc = "The other core's permanent id."]
    pub id: ::std::string::String,
    #[doc = "Last observed systemd logout protection; null for old or unsupported cores."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub keeps_running_after_logout: ::std::option::Option<bool>,
    #[doc = "Remote account for loginctl enable-linger, when applicable."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub logout_linger_user: ::std::option::Option<::std::string::String>,
    #[doc = "What this core calls it: `rohan` in `rohan/t7`."]
    pub name: ::std::string::String,
    #[doc = "The hc session there (null: its default)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub session: ::std::option::Option<::std::string::String>,
    #[doc = "Since when it is not connected (ms since the epoch)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub since_ms: ::std::option::Option<u64>,
    #[doc = "connecting, connected, needs_login or unreachable."]
    pub state: ::std::string::String,
    #[doc = "How it is reached: an ssh target, or (`via`) a command."]
    pub target: ::std::string::String,
    #[doc = "Short-lived (a sandbox for a task): clients may show it apart from the\nmachines that stay. hc treats it like any other."]
    #[serde(default)]
    pub temporary: bool,
    #[doc = "Its hc version when last seen."]
    pub version: ::std::string::String,
    #[doc = "`target` is a command that runs a shell line there, given as its last\nargument, like ssh (a provider's own CLI, ssh with its options)."]
    #[serde(default)]
    pub via: bool,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub why: ::std::option::Option<::std::string::String>,
}
#[doc = "`PutResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct PutResult {
    pub bytes: u32,
    #[doc = "Where it landed, as the terminal's machine sees it."]
    pub path: ::std::string::String,
}
#[doc = "`ReadResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ReadResult {
    #[doc = "False when part of what was asked for may be missing."]
    pub complete: bool,
    #[doc = "\"screen\", \"history\", or \"stitched\" (a full-screen app, read by scrolling it)."]
    pub how: ::std::string::String,
    pub text: ::std::string::String,
    #[doc = "Why only the screen: \"working\", \"blocked\" or \"error\" (a full-screen app\nis not scrolled in these states)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub why: ::std::option::Option<::std::string::String>,
}
#[doc = "`RemoveResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct RemoveResult {
    pub removed: ::std::string::String,
}
#[doc = "`RunResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct RunResult {
    #[doc = "`t7`: this terminal forever on this core."]
    pub address: ::std::string::String,
    pub argv: ::std::vec::Vec<::std::string::String>,
    pub cols: u16,
    pub created_ms: u64,
    pub cwd: ::std::string::String,
    #[serde(default)]
    pub die_with_parent: bool,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub exit_code: ::std::option::Option<i32>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub finished_ms: ::std::option::Option<u64>,
    #[doc = "Template runs: whether the first message went in."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub first_message: ::std::option::Option<FirstMessage>,
    #[doc = "What runs in it right now and where (null once it has finished)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub foreground: ::std::option::Option<Foreground>,
    #[doc = "Which terminal the program believes it runs in (TERM_PROGRAM and\nversion), fixed at start; XTVERSION answers with it."]
    #[serde(default)]
    pub identity: ::std::string::String,
    #[doc = "Replace a lost program with this machine’s shell, in place. Exit with a code and kill remain final."]
    #[serde(default)]
    pub keep: bool,
    pub labels: ::std::collections::HashMap<::std::string::String, ::std::string::String>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub name: ::std::option::Option<::std::string::String>,
    #[doc = "Its parent gets notes about it (see `--no-notify`)."]
    #[serde(default)]
    pub notify: bool,
    #[doc = "With `wait`: its text then, like `terminal.read` (the screen, plus\nhistory unless it is a full-screen app)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub output: ::std::option::Option<::std::string::String>,
    #[doc = "The terminal that started it (`t3`), set once at start."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub parent: ::std::option::Option<::std::string::String>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub paste_gap_ms: ::std::option::Option<u64>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub pid: ::std::option::Option<u32>,
    #[doc = "Why its program ended or is pending (`spawn_pending` means its initial\nspawn timed out, but the holder may still be starting it):\nexited, killed, server_stop, host_shutdown (Windows\nis ending the session, including logout), signaled (a kept program\ndied by signal without an exit code), holder_lost (its holder\ndied), stopped_by_restart (nothing ran it any more after a restart)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub reason: ::std::option::Option<::std::string::String>,
    #[doc = "Other 7501 records by id (programs reporting on several things)."]
    #[serde(default, skip_serializing_if = "::std::collections::HashMap::is_empty")]
    pub records: ::std::collections::HashMap<::std::string::String, Status>,
    #[doc = "Whether the program ever sent a 7501 report."]
    #[serde(default)]
    pub reported: bool,
    #[doc = "Removed right after it exits (once a waiting parent has the exit code)."]
    #[serde(default)]
    pub rm: bool,
    pub rows: u16,
    #[doc = "\"exact\", or \"inexact\" after a core crash (older history may be missing)."]
    pub screen: ::std::string::String,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub signal: ::std::option::Option<i32>,
    #[doc = "\"live\" or \"finished\"."]
    pub state: ::std::string::String,
    #[doc = "The program's own word for what it's doing (OSC 7501 root record);\nnull if it never said, or after it exited."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status: ::std::option::Option<Status>,
    #[doc = "When that word last changed, ms since the epoch; null if it never said."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status_ms: ::std::option::Option<u64>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub stop: ::std::option::Option<::std::string::String>,
    #[doc = "Template overrides: the key that submits a message, the stop key, the pause\nbetween paste and submit."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub submit: ::std::option::Option<::std::string::String>,
    #[doc = "The template that started it."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub template: ::std::option::Option<::std::string::String>,
    #[serde(default)]
    pub title: ::std::string::String,
    #[doc = "Goes up each time the status leaves `working`."]
    #[serde(default)]
    pub turn: u64,
    #[doc = "With `wait`: how the wait ended (exit, a status, or timeout: still running)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub wait: ::std::option::Option<WaitResult>,
}
#[doc = "`SendResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct SendResult {
    #[doc = "Whether it went `working` within a few seconds; null when it doesn't\nreport status, so nobody can tell."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub started: ::std::option::Option<bool>,
    #[doc = "The program's status word when the text went in (null: it never said)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status: ::std::option::Option<::std::string::String>,
    #[doc = "The turn when the text went in."]
    pub turn: u64,
    #[doc = "With `wait`: how that turn ended."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub wait: ::std::option::Option<WaitResult>,
}
#[doc = "One 7501 record. Words are the program's own; nothing is translated."]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Status {
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub app: ::std::option::Option<::std::string::String>,
    #[doc = "permission, question or auth (blocked only, when the program said)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub kind: ::std::option::Option<::std::string::String>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub msg: ::std::option::Option<::std::string::String>,
    #[doc = "0..100 (working or blocked, when the program said)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub progress: ::std::option::Option<u8>,
    #[doc = "idle, working, done, blocked or error."]
    pub state: ::std::string::String,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub title: ::std::option::Option<::std::string::String>,
}
#[doc = "`StatusResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct StatusResult {
    pub core: CoreInfo,
    pub finished: u32,
    #[doc = "The holder new terminals go to."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub holder: ::std::option::Option<HolderInfo>,
    #[doc = "Every holder: after an update, older ones run their terminals to the end."]
    pub holders: ::std::vec::Vec<HolderInfo>,
    #[doc = "The package manager this hc came from (homebrew, nix, mise, or\nsystem package manager): it updates hc, `hc update` won't.\nNull for a replaceable plain install."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub installed_by: ::std::option::Option<::std::string::String>,
    #[doc = "True: systemd lingering is enabled; false: programs stop at last logout.\nNull: unsupported, not applicable or unknown. Checked on every request."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub keeps_running_after_logout: ::std::option::Option<bool>,
    pub live: u32,
    #[doc = "Account for the administrator's loginctl enable-linger command."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub logout_linger_user: ::std::option::Option<::std::string::String>,
    pub pid: u32,
    pub uptime_ms: u64,
    pub version: ::std::string::String,
}
#[doc = "`StopResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct StopResult {
    pub stopped: bool,
}
#[doc = "`SubscribeResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct SubscribeResult {
    #[doc = "Framed mode: the channel events arrive on (JSON lines, with credit).\nPlain mode: none; this connection now carries only notifications."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub channel: ::std::option::Option<u32>,
    #[doc = "Events after this sequence follow. After a `missed` snapshot, ignore\nevents whose seq is not above the snapshot's."]
    pub seq: u64,
    #[doc = "Matching terminals here and on connected machines (`machine/t7`)."]
    pub terminals: ::std::vec::Vec<Terminal>,
}
#[doc = "`TemplateInfo`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct TemplateInfo {
    pub command: ::std::string::String,
    pub description: ::std::string::String,
    #[doc = "Set when the file can't be used."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub error: ::std::option::Option<::std::string::String>,
    pub name: ::std::string::String,
}
#[doc = "`TemplatesResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct TemplatesResult {
    pub templates: ::std::vec::Vec<TemplateInfo>,
}
#[doc = "`Terminal`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Terminal {
    #[doc = "`t7`: this terminal forever on this core."]
    pub address: ::std::string::String,
    pub argv: ::std::vec::Vec<::std::string::String>,
    pub cols: u16,
    pub created_ms: u64,
    pub cwd: ::std::string::String,
    #[serde(default)]
    pub die_with_parent: bool,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub exit_code: ::std::option::Option<i32>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub finished_ms: ::std::option::Option<u64>,
    #[doc = "A template first message that was not sent; retained for later inspection."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub first_message: ::std::option::Option<FirstMessage>,
    #[doc = "What runs in it right now and where (null once it has finished)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub foreground: ::std::option::Option<Foreground>,
    #[doc = "Which terminal the program believes it runs in (TERM_PROGRAM and\nversion), fixed at start; XTVERSION answers with it."]
    #[serde(default)]
    pub identity: ::std::string::String,
    #[doc = "Replace a lost program with this machine’s shell, in place. Exit with a code and kill remain final."]
    #[serde(default)]
    pub keep: bool,
    pub labels: ::std::collections::HashMap<::std::string::String, ::std::string::String>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub name: ::std::option::Option<::std::string::String>,
    #[doc = "Its parent gets notes about it (see `--no-notify`)."]
    #[serde(default)]
    pub notify: bool,
    #[doc = "The terminal that started it (`t3`), set once at start."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub parent: ::std::option::Option<::std::string::String>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub paste_gap_ms: ::std::option::Option<u64>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub pid: ::std::option::Option<u32>,
    #[doc = "Why its program ended or is pending (`spawn_pending` means its initial\nspawn timed out, but the holder may still be starting it):\nexited, killed, server_stop, host_shutdown (Windows\nis ending the session, including logout), signaled (a kept program\ndied by signal without an exit code), holder_lost (its holder\ndied), stopped_by_restart (nothing ran it any more after a restart)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub reason: ::std::option::Option<::std::string::String>,
    #[doc = "Other 7501 records by id (programs reporting on several things)."]
    #[serde(default, skip_serializing_if = "::std::collections::HashMap::is_empty")]
    pub records: ::std::collections::HashMap<::std::string::String, Status>,
    #[doc = "Whether the program ever sent a 7501 report."]
    #[serde(default)]
    pub reported: bool,
    #[doc = "Removed right after it exits (once a waiting parent has the exit code)."]
    #[serde(default)]
    pub rm: bool,
    pub rows: u16,
    #[doc = "\"exact\", or \"inexact\" after a core crash (older history may be missing)."]
    pub screen: ::std::string::String,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub signal: ::std::option::Option<i32>,
    #[doc = "\"live\" or \"finished\"."]
    pub state: ::std::string::String,
    #[doc = "The program's own word for what it's doing (OSC 7501 root record);\nnull if it never said, or after it exited."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status: ::std::option::Option<Status>,
    #[doc = "When that word last changed, ms since the epoch; null if it never said."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status_ms: ::std::option::Option<u64>,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub stop: ::std::option::Option<::std::string::String>,
    #[doc = "Template overrides: the key that submits a message, the stop key, the pause\nbetween paste and submit."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub submit: ::std::option::Option<::std::string::String>,
    #[doc = "The template that started it."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub template: ::std::option::Option<::std::string::String>,
    #[serde(default)]
    pub title: ::std::string::String,
    #[doc = "Goes up each time the status leaves `working`."]
    #[serde(default)]
    pub turn: u64,
}
#[doc = "`WaitResult`"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct WaitResult {
    #[doc = "With `children`: one result per child it reports on."]
    pub children: ::std::vec::Vec<WaitResult>,
    #[doc = "False on timeout: a check-in, the terminal is still running."]
    pub ended: bool,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub exit_code: ::std::option::Option<i32>,
    #[doc = "With a message: the sender's address (the text is already in your input);\nnull when a person sent it."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub from: ::std::option::Option<::std::string::String>,
    #[doc = "status, exit (including program loss in a kept terminal), output, timeout,\nor not_started (after a send: it did not start working; a plain wait\nstill follows that send's turn); with\n`children`: children; message (someone messaged the waiter, any\nwait); for each child in `children`: status, exit, or running (still\nworking)."]
    pub reason: ::std::string::String,
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub signal: ::std::option::Option<i32>,
    #[doc = "The exact word the program reported (current word on timeout or output)."]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub status: ::std::option::Option<::std::string::String>,
    pub terminal: ::std::string::String,
    pub turn: u64,
}
#[doc = " Error types."]
pub mod error {
    #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
