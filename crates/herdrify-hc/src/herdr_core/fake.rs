//! A fake core for `HerdrCore`'s unit tests, over the connection layer's in-memory link.
//!
//! [`connected`] answers the handshake and returns a client and the peer, [`connected_at`] does the
//! same for a core of a chosen version, and [`serve`] runs one call against the peer: it reads the
//! request the call sent, answers it with a given result and returns both the request and the
//! call's output. [`request_of`] reads the request a call has built, for comparing it with one
//! built from the `protocol` types by hand.

use std::error::Error as StdError;
use std::future::IntoFuture;

use serde_json::{Value, json};

use super::{Call, HerdrCoreError};
use crate::connection::Client;
use crate::connection::memory_link::{MemoryPeer, MemoryTransport, link};
use crate::protocol::{Channel, Frame, FrameKind};

/// The version `connected` answers `hello` with.
const DEFAULT_VERSION: &str = "0.1.21";

/// The reply a core of `version` sends to `hello`.
fn hello_reply(version: &str) -> String {
    json!({
        "id": 0,
        "jsonrpc": "2.0",
        "result": {
            "core": {"id": "c", "name": "n"},
            "features": [],
            "framed": true,
            "generation": 1,
            "methods": [],
            "version": version,
        },
    })
    .to_string()
}

/// A test failure.
pub(super) type TestError = Box<dyn StdError + Send + Sync>;

/// A client connected over the in-memory link to a core of version 0.1.21, and the peer on its
/// other end.
///
/// # Errors
///
/// Fails when the handshake does not complete.
pub(super) async fn connected() -> Result<(Client<MemoryTransport>, MemoryPeer), TestError> {
    connected_at(DEFAULT_VERSION).await
}

/// A client connected over the in-memory link to a core that answers `hello` with `version`, and
/// the peer on its other end.
///
/// # Errors
///
/// Fails when the handshake does not complete.
pub(super) async fn connected_at(
    version: &str,
) -> Result<(Client<MemoryTransport>, MemoryPeer), TestError> {
    let (transport, mut peer) = link();
    peer.reply_line(&hello_reply(version));
    let client = Client::connect(transport).await?;
    peer.line().await?;
    Ok((client, peer))
}

/// Runs `call`, answers its request with `result`, and returns the request's `method` and
/// `params` together with the call's output.
///
/// # Errors
///
/// Fails when the call sends nothing, sends something that is not a request, or panics.
pub(super) async fn serve<C>(
    peer: &mut MemoryPeer,
    call: C,
    result: Value,
) -> Result<(Value, C::Output), TestError>
where
    C: IntoFuture,
    C::IntoFuture: Send + 'static,
    C::Output: Send + 'static,
{
    let running = tokio::spawn(call.into_future());
    let frame = peer.frame().await?;
    let request: Value = serde_json::from_slice(&frame.payload)?;
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let reply = json!({"id": id, "jsonrpc": "2.0", "result": result});
    peer.push(Frame {
        channel: Channel::RPC,
        kind: FrameKind::Data,
        payload: reply.to_string().into_bytes(),
    });
    let output = running.await?;
    let sent = json!({"method": request["method"], "params": request["params"]});
    Ok((sent, output))
}

/// The request `call` has built.
///
/// # Errors
///
/// Fails with the error an option or argument recorded.
pub(super) fn request_of<P>(call: &Call<MemoryTransport, P>) -> Result<&P, TestError> {
    call.request()
        .map_err(|error: &HerdrCoreError| error.to_string().into())
}
