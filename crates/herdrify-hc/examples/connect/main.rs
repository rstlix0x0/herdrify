//! Reach a herdr-core and ask how it is doing.
//!
//! With no arguments this reaches the core that `hc connect` reaches, through `HerdrCore`. The
//! arguments pick another way in:
//!
//! - `--session <name>`: a named core on this machine
//! - `--ssh <host>`: the core on another machine, through `ssh <host> hc connect`
//! - `--stream`: the local core again, over a transport built by hand from a child's pipes
//! - `--client`: the same report with the `Client` and the typed requests, no `HerdrCore`

pub mod client;
pub mod entrypoint;
pub mod status;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const USAGE: &str = "usage: connect [--session <name> | --ssh <host> | --stream | --client]";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => status::report(&entrypoint::local().await?).await,
        ["--session", name] => status::report(&entrypoint::session(name).await?).await,
        ["--ssh", host] => status::report(&entrypoint::ssh(host).await?).await,
        ["--stream"] => status::report(&entrypoint::by_hand().await?).await,
        ["--client"] => client::report().await,
        _ => Err(USAGE.into()),
    }
}
