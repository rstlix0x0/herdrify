# airsl 0.1.4 excerpts

Source: the published crate as unpacked by cargo, `~/.cargo/registry/src/index.crates.io-*/airsl-0.1.4/`. Each block is the raw output of the command shown, run on 2026-11-22.

```text
$ grep -n 'tokio' Cargo.toml; echo "(grep exit: $?)"
(grep exit: 1)
```

```text
$ grep -n '^\[dependencies.mlua\]' -A9 Cargo.toml
107:[dependencies.mlua]
108-version = "0.12"
109-features = [
110-    "lua54",
111-    "vendored",
112-    "serialize",
113-    "send",
114-]
115-
116-[dependencies.regex]
```

```text
$ sed -n '215p;237p' src/engine.rs
    pub fn eval(&self, script: &Script) -> Result<()> {
    pub fn eval_to<T: FromLuaMulti>(&self, script: &Script) -> Result<T> {
```

```text
$ sed -n '54,59p' src/sandbox/grant_set.rs
pub struct GrantSet {
    reach: GrantReach,
    fs: FsGrant,
    env: EnvGrant,
    proc: ProcGrant,
}
```

```text
$ sed -n '97,117p' src/modules/registry.rs
pub trait HostModule: Send + Sync {
    /// The key this module is installed under in the `airsstack` table.
    fn name(&self) -> &ModuleName;

    /// Populates `table` with this module's functions.
    ///
    /// `context` carries the policy the engine was built with. A module needing no authority — a
    /// pure computation like `path` — ignores it; one that guards an operation captures the grants
    /// it needs into the functions it creates, and checks them there rather than here.
    ///
    /// # Errors
    ///
    /// Returns an error when a function cannot be created or set, which the engine reports as
    /// [`Error::ModuleInstall`].
    fn install(
        &self,
        lua: &mlua::Lua,
        table: &mlua::Table,
        context: &InstallContext<'_>,
    ) -> Result<()>;
}
```

```text
$ grep -n 'pub const fn grants\|pub const fn policy' src/modules/registry.rs
46:    pub const fn policy(&self) -> &Policy {
52:    pub const fn grants(&self) -> &GrantSet {
```

```text
$ grep -n 'pub const MANIFEST_FILE' src/extension/manifest.rs
40:pub const MANIFEST_FILE: &str = "extension.toml";
```

```text
$ grep -n 'pub fn negotiate' src/extension/negotiate.rs
275:pub fn negotiate(manifest: &Manifest, ceiling: &Ceiling, modules: &ModuleSet) -> Negotiation {
```

```text
$ grep -n 'pub struct Ceiling' src/extension/ceiling.rs
18:pub struct Ceiling(Policy);
```

```text
$ grep -n 'pub trait Approver' src/extension/approver.rs
68:pub trait Approver: Send + Sync {
```

```text
$ grep -n 'allows(' src/modules/proc.rs
115:                if !g.is_unrestricted() && !g.proc().allows(&program) {
182:                if !g.is_unrestricted() && !g.proc().allows(&program) {
```


## Async surface and transitive tokio (added after review)

```text
$ grep -rn 'async fn\|impl Future' src; echo "(grep exit: $?)"
(grep exit: 1)
```

```text
$ grep -rn 'pub fn eval' src | head -3   # control: the same search shape finds sync fns
src/engine.rs:215:    pub fn eval(&self, script: &Script) -> Result<()> {
src/engine.rs:237:    pub fn eval_to<T: FromLuaMulti>(&self, script: &Script) -> Result<T> {
```

```text
$ (in evidence/probe) cargo tree --locked -e normal -i tokio
tokio v1.54.1
└── herdrify-adr-probe v0.0.0 (/Users/hiraq/.herdr/worktrees/herdrify/workspace-skeleton/.claudestacks/sdlc/adrs/evidence/probe)
```

## More excerpts added after review

```text
$ awk 'NR>=219 && NR<=227 {print NR": "$0}' src/sandbox/grants.rs
219: }
220: 
221: /// Which executables a script may run.
222: ///
223: /// Matched on the program name as written, not on a resolved path. That is a real limitation, and
224: /// the crate's sandbox documentation records it: an allowlist of names is defeated by anything that
225: /// can put its own `git` earlier on `PATH`, so this grant is only as strong as the write grants
226: /// sitting alongside it.
227: #[derive(Debug, Clone, PartialEq, Eq, Default)]
```

```text
$ awk 'NR>=255 && NR<=257 {print NR": "$0}' src/sandbox/grants.rs
255:     pub fn allows(&self, program: &str) -> bool {
256:         self.executables.contains(program)
257:     }
```

```text
$ awk 'NR>=36 && NR<=53 {print NR": "$0}' src/sandbox/grant_set.rs
36: /// use airsl::{GrantSet, Policy};
37: ///
38: /// // Built from the working directory, not a hardcoded literal: a unix-spelled absolute path has
39: /// // no drive on Windows, so it would resolve against whichever drive this doctest happens to run
40: /// // from rather than matching the paths checked below.
41: /// let repo = std::env::current_dir().unwrap().join("repo");
42: /// let index = repo.join(".index");
43: /// let policy = Policy::confined().with_grants(
44: ///     GrantSet::declared()
45: ///         .with_fs(|fs| fs.read(&repo).write(&index))
46: ///         .with_env(|env| env.read(["HOME"]))
47: ///         .with_proc(|proc| proc.allow(["git"])),
48: /// );
49: /// assert!(policy.grants().fs().allows_read(&repo.join("src")));
50: /// assert!(!policy.grants().fs().allows_write(&repo.join("src")));
51: /// ```
52: #[derive(Debug, Clone, PartialEq, Eq, Default)]
53: #[non_exhaustive]
```

```text
$ awk 'NR>=78 && NR<=86 {print NR": "$0}' src/builder.rs
78:     /// Replaces the global the host modules are installed under.
79:     ///
80:     /// Defaults to `airsstack`. An embedding host should set its own, so that a module contributed
81:     /// by a third party does not land in a namespace named after somebody else's system.
82:     #[must_use]
83:     pub fn root_table(mut self, root: RootTable) -> Self {
84:         self.root = root;
85:         self
86:     }
```

```text
$ awk 'NR>=1 && NR<=27 {print NR": "$0}' src/modules/registry.rs
1: //! The extension seam: how Rust code contributes functions to the `airsstack` Lua table.
2: //!
3: //! Exists so the engine does not need to know the set of modules at compile time. The built-in
4: //! standard library and a module contributed by a downstream crate go through the same trait and
5: //! the same uniqueness check, which is what lets `airsl` serve as a shared Lua integration point
6: //! rather than a fixed script runner.
7: //!
8: //! Responsibilities:
9: //!
10: //! - [`HostModule`], the trait a contributor implements.
11: //! - [`InstallContext`], what the engine tells a module about the policy it is being installed under.
12: //! - [`ModuleSet`], an ordered collection of modules with name uniqueness enforced on insert.
13: //!
14: //! Non-responsibilities: installation order effects. Modules are installed in insertion order and
15: //! must not depend on one another's presence.
16: 
17: use crate::error::{Error, Result};
18: use crate::sandbox::{GrantSet, Policy};
19: use crate::types::{ModuleName, RootTable};
20: 
21: /// What the engine tells a module while installing it.
22: ///
23: /// The authority a module enforces has to be the same object the policy reports, so it arrives at
24: /// installation rather than at construction. A module built with its own copy of the grants could
25: /// disagree with the engine it was installed into, and `airsl doctor` would describe the policy
26: /// while the module quietly enforced something else — which would make the report worthless
27: /// exactly where it matters. Handing the policy to `install` makes that disagreement unspellable.
```

```text
$ awk 'NR>=269 && NR<=275 {print NR": "$0}' src/extension/negotiate.rs
269: /// Intersects `manifest`'s request with `ceiling`.
270: ///
271: /// Required misses become denials, optional misses become reductions, and every grant from
272: /// either block lands in the returned policy. The language surface is the ceiling's; the
273: /// limits are the lower of request and ceiling on each axis.
274: #[must_use]
275: pub fn negotiate(manifest: &Manifest, ceiling: &Ceiling, modules: &ModuleSet) -> Negotiation {
```

