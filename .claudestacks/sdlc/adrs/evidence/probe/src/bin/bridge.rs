//! Proves the sync-Lua → async-Rust bridge herdrify relies on, against airsl 0.1.4 + tokio.
//!
//! A host module `hc.read(id)` calls an async function through a captured
//! `tokio::runtime::Handle::block_on`. Case A runs the airsl eval inside
//! `tokio::task::spawn_blocking` (the supported shape). Case B runs the same eval directly on
//! an async worker thread (the forbidden shape) and reports what happens.

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use airsl::{Engine, HostModule, InstallContext, ModuleName, Policy, RootTable, Script};
use tokio::runtime::Handle;

/// Stands in for an async `Terminals` port call (e.g. hc `terminal.read`).
async fn fake_read(id: String) -> String {
    tokio::time::sleep(Duration::from_millis(10)).await;
    format!("screen of {id}")
}

#[derive(Debug)]
struct Hc {
    name: ModuleName,
    handle: Handle,
}

impl HostModule for Hc {
    fn name(&self) -> &ModuleName {
        &self.name
    }

    fn install(
        &self,
        lua: &airsl::mlua::Lua,
        table: &airsl::mlua::Table,
        _context: &InstallContext<'_>,
    ) -> Result<(), airsl::Error> {
        let fail = |e: airsl::mlua::Error| airsl::Error::lua("hc", e);
        let handle = self.handle.clone();
        let read = lua
            .create_function(move |_, id: String| Ok(handle.block_on(fake_read(id))))
            .map_err(fail)?;
        table.set("read", read).map_err(fail)?;
        Ok(())
    }
}

fn build_engine(handle: Handle) -> Result<Engine, airsl::Error> {
    let mut modules = airsl::modules::stdlib()?;
    modules.insert(Box::new(Hc {
        name: ModuleName::new("hc")?,
        handle,
    }))?;
    Engine::builder()
        .policy(Policy::confined())
        .root_table(RootTable::new("herdrify")?)
        .stdlib(modules)
        .build()
}

fn script() -> Result<Script, airsl::Error> {
    Script::from_source("return herdrify.hc.read(\"t7\")", "component.lua")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine = Arc::new(build_engine(Handle::current())?);

    // Case A: eval on a blocking thread; block_on bridges to async.
    let a_engine = Arc::clone(&engine);
    let a = tokio::task::spawn_blocking(move || -> Result<String, airsl::Error> {
        a_engine.eval_to::<String>(&script()?)
    })
    .await?;
    match a {
        Ok(v) => println!("case A (spawn_blocking): ok -> {v}"),
        Err(e) => println!("case A (spawn_blocking): error -> {e}"),
    }

    // Case B: eval directly on an async worker thread.
    let b_engine = Arc::clone(&engine);
    let b = tokio::spawn(async move { b_engine.eval_to::<String>(&script()?) }).await;
    match b {
        Ok(Ok(v)) => println!("case B (async context): ok -> {v}"),
        Ok(Err(e)) => println!("case B (async context): error -> {e}"),
        Err(join) if join.is_panic() => {
            let payload = join.into_panic();
            let msg = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "<non-string panic>".to_owned());
            println!("case B (async context): panic -> {msg}");
        }
        Err(join) => println!("case B (async context): join error -> {join}"),
    }
    Ok(())
}
