//! The request that asks the core to describe its own methods.
//!
//! Responsibilities:
//!
//! - [`RpcDiscover`], serialized as the empty parameter object the schema lists
//!
//! Non-responsibilities:
//!
//! - interpreting the description, whose shape is the core's `OpenRPC` document and is returned as
//!   JSON

use serde::Serialize;
use serde_json::Value;

use crate::protocol::Method;

/// The parameters of `rpc.discover`: none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RpcDiscover {}

impl RpcDiscover {
    /// An `rpc.discover` request.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Method for RpcDiscover {
    const NAME: &'static str = "rpc.discover";
    type Params = Self;
    type Output = Value;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::RpcDiscover;
    use crate::protocol::Method;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn rpc_request_has_no_params_and_its_wire_name() -> TestResult {
        assert_eq!(serde_json::to_value(RpcDiscover::new())?, json!({}));
        assert_eq!(RpcDiscover::NAME, "rpc.discover");
        Ok(())
    }
}
