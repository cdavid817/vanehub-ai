use super::relay_jsonrpc::JsonRpcId;
use crate::contexts::tooling::mcp::application::McpRuntimeError;
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use std::fmt;
use std::io::Write;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RelayFailure {
    code: McpFailureCode,
}

impl RelayFailure {
    pub(super) const fn new(code: McpFailureCode) -> Self {
        Self { code }
    }

    pub(super) fn from_reqwest(error: &reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::new(McpFailureCode::Timeout)
        } else {
            Self::new(McpFailureCode::Transport)
        }
    }

    pub(super) const fn code(self) -> McpFailureCode {
        self.code
    }

    pub(super) fn classification(self) -> &'static str {
        self.code.as_str()
    }

    pub(super) fn write_response(
        self,
        output: &mut impl Write,
        id: &JsonRpcId,
    ) -> Result<(), Self> {
        serde_json::to_writer(
            &mut *output,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": id.to_value(),
                "error": {
                    "code": self.protocol_code(),
                    "message": self.protocol_message(),
                }
            }),
        )
        .map_err(|_| Self::new(McpFailureCode::Transport))?;
        output
            .write_all(b"\n")
            .and_then(|()| output.flush())
            .map_err(|_| Self::new(McpFailureCode::Transport))
    }

    fn protocol_code(self) -> i32 {
        match self.code {
            McpFailureCode::Timeout => -32001,
            McpFailureCode::Cancelled => -32002,
            McpFailureCode::LimitExceeded => -32003,
            McpFailureCode::UpstreamHttp => -32004,
            McpFailureCode::Cleanup => -32005,
            McpFailureCode::Validation => -32600,
            McpFailureCode::Protocol => -32603,
            McpFailureCode::Spawn | McpFailureCode::Transport => -32000,
        }
    }

    fn protocol_message(self) -> &'static str {
        match self.code {
            McpFailureCode::Timeout => "MCP request timed out",
            McpFailureCode::Cancelled => "MCP request was cancelled",
            McpFailureCode::Protocol => "MCP upstream returned an invalid protocol response",
            McpFailureCode::UpstreamHttp => "MCP HTTP upstream returned an error",
            McpFailureCode::LimitExceeded => "MCP response exceeded a safety limit",
            McpFailureCode::Transport => "MCP upstream transport failed",
            McpFailureCode::Cleanup => "MCP upstream cleanup failed",
            McpFailureCode::Validation => "MCP request validation failed",
            McpFailureCode::Spawn => "MCP upstream could not be started",
        }
    }
}

impl From<McpRuntimeError> for RelayFailure {
    fn from(error: McpRuntimeError) -> Self {
        Self::new(error.code())
    }
}

impl fmt::Display for RelayFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.safe_message())
    }
}

impl std::error::Error for RelayFailure {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_emit_fixed_bounded_protocol_data_without_diagnostics() {
        let mut output = Vec::new();
        RelayFailure::new(McpFailureCode::LimitExceeded)
            .write_response(&mut output, &JsonRpcId::String("request-1".to_string()))
            .expect("safe response");
        let value: serde_json::Value = serde_json::from_slice(&output).expect("JSON-RPC");
        assert_eq!(value["id"], "request-1");
        assert_eq!(value["error"]["code"], -32003);
        assert_eq!(
            value["error"]["message"],
            "MCP response exceeded a safety limit"
        );
        assert_eq!(
            RelayFailure::new(McpFailureCode::Protocol).classification(),
            "protocol"
        );
    }
}
