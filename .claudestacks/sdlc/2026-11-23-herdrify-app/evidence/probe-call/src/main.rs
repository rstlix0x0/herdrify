//! Probe: rustc 1.94 compiles the herdrify-app `Call` shape of spec §2/§7/§8 and its future is
//! `Send`: a `Transport` trait with a return-position `impl Future + Send`, a `Client<T>` holding
//! `Arc<T>` with an inherent `async fn run<C: Command>`, and `Call<T, C>` owning a `Client<T>`
//! clone, whose `IntoFuture` is a `'static` `Pin<Box<dyn Future + Send>>`.
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
struct Error(String);

struct RunOutput {
    stdout: Vec<u8>,
}

trait Transport: Send + Sync + 'static {
    fn run(&self, argv: Vec<String>) -> impl Future<Output = Result<RunOutput, Error>> + Send;
}

trait Command: Serialize {
    const NAME: &'static str;
    type Output: DeserializeOwned;
}

struct Client<T: Transport> {
    transport: Arc<T>,
}

impl<T: Transport> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self { transport: Arc::clone(&self.transport) }
    }
}

impl<T: Transport> Client<T> {
    fn new(transport: T) -> Self {
        Self { transport: Arc::new(transport) }
    }

    async fn run<C: Command>(&self, command: C) -> Result<C::Output, Error> {
        let args = serde_json::to_value(&command).map_err(|e| Error(e.to_string()))?;
        let out = self.transport.run(vec!["do".into(), C::NAME.into(), args.to_string()]).await?;
        serde_json::from_slice(&out.stdout).map_err(|e| Error(e.to_string()))
    }
}

#[must_use]
struct Call<T: Transport, C> {
    client: Client<T>,
    command: Result<C, Error>,
}

// dyn: `IntoFuture` must name its future type, and an async fn's future cannot be named on stable.
type BoxedCall<O> = Pin<Box<dyn Future<Output = Result<O, Error>> + Send>>;

impl<T, C> IntoFuture for Call<T, C>
where
    T: Transport,
    C: Command + Send + 'static,
    C::Output: Send,
{
    type Output = Result<C::Output, Error>;
    type IntoFuture = BoxedCall<C::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.client.run(self.command?).await })
    }
}

struct Fake;
impl Transport for Fake {
    fn run(&self, argv: Vec<String>) -> impl Future<Output = Result<RunOutput, Error>> + Send {
        async move {
            println!("argv = {argv:?}");
            Ok(RunOutput { stdout: br#"{"ok":true}"#.to_vec() })
        }
    }
}

#[derive(Serialize)]
struct TabNext {}
#[derive(Debug, Deserialize)]
struct Ack {
    #[allow(dead_code)]
    ok: bool,
}
impl Command for TabNext {
    const NAME: &'static str = "tab.next";
    type Output = Ack;
}

fn assert_send<F: Send>(_: &F) {}

#[tokio::main]
async fn main() {
    let client = Client::new(Fake);
    let call = Call { client: client.clone(), command: Ok(TabNext {}) };
    let fut = call.into_future();
    assert_send(&fut);
    let spawned = tokio::spawn(fut).await.expect("join");
    println!("spawned call result = {spawned:?}");
    let refused: Call<Fake, TabNext> = Call { client, command: Err(Error("invalid name".into())) };
    println!("refused call result = {:?}", refused.await);
}
