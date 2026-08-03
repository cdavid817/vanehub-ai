use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationKind, OperationLog, OperationLogPort,
    OperationsApi,
};
use crate::contexts::tooling::mcp::application::{
    ConnectionTestResult, McpApplicationError, McpClockPort, McpLoggingPort, McpOperationPort,
    McpProjectPathPort, StartedOperation,
};
use crate::contexts::tooling::mcp::domain::{ConnectionOutcome, McpFailureCode, ToolDescriptor};
use crate::platform::clock::SystemClock;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemMcpClock;

impl McpClockPort for SystemMcpClock {
    fn now(&self) -> String {
        SystemClock.unix_seconds()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CurrentProjectPathAdapter;

impl McpProjectPathPort for CurrentProjectPathAdapter {
    fn current_project_path(&self) -> Result<String, McpApplicationError> {
        crate::platform::filesystem::current_directory()
            .map(|path| path.to_string_lossy().to_string())
            .map_err(|error| McpApplicationError::Storage(error.to_string()))
    }
}

#[derive(Clone)]
pub(crate) struct McpOperationAdapter {
    operations: OperationsApi,
}

impl McpOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl McpOperationPort for McpOperationAdapter {
    fn start_connection_test(
        &self,
        server_name: &str,
    ) -> Result<StartedOperation, McpApplicationError> {
        self.operations
            .start(
                OperationKind::Mcp,
                Some(server_name.to_string()),
                Some(format!("Testing MCP server {server_name}")),
            )
            .map(|operation| StartedOperation {
                id: operation.id,
                related_entity_id: operation.related_entity_id,
                message: operation.message,
                created_at: operation.created_at,
                updated_at: operation.updated_at,
            })
            .map_err(operation_error)
    }

    fn append_log(&self, operation_id: &str, line: String) -> Result<(), McpApplicationError> {
        self.operations
            .append_log(operation_id, line)
            .map(|_| ())
            .map_err(operation_error)
    }

    fn connection_test_cancellation(
        &self,
        operation_id: &str,
    ) -> Result<crate::contexts::tooling::mcp::application::McpCancellation, McpApplicationError>
    {
        self.operations
            .cancellation_flag(operation_id)
            .map(crate::contexts::tooling::mcp::application::McpCancellation::from_shared)
            .map_err(operation_error)
    }

    fn complete_connection_test(
        &self,
        operation_id: &str,
        result: &ConnectionTestResult,
    ) -> Result<(), McpApplicationError> {
        let payload = ConnectionTestPayload::from(result);
        let value = serde_json::to_value(payload)
            .map_err(|error| McpApplicationError::Validation(error.to_string()))?;
        self.operations
            .complete(operation_id, Some(value))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn fail_connection_test(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<(), McpApplicationError> {
        self.operations
            .fail(operation_id, error)
            .map(|_| ())
            .map_err(operation_error)
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedMcpLoggingAdapter {
    operation_logging: Arc<dyn OperationLogPort>,
    diagnostic_logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedMcpLoggingAdapter {
    pub(crate) fn new<T>(logging: Arc<T>) -> Self
    where
        T: OperationLogPort + DiagnosticLogPort + 'static,
    {
        Self {
            operation_logging: logging.clone(),
            diagnostic_logging: logging,
        }
    }
}

impl McpLoggingPort for UnifiedMcpLoggingAdapter {
    fn record_connection_outcome(
        &self,
        operation_id: &str,
        server_name: &str,
        outcome: &ConnectionOutcome,
    ) -> Result<(), McpApplicationError> {
        let mut context = BTreeMap::new();
        context.insert("serverName".to_string(), server_name.to_string());
        context.insert("durationMs".to_string(), outcome.duration_ms().to_string());
        context.insert("toolCount".to_string(), outcome.tools().len().to_string());
        if let Some(code) = outcome.error_code() {
            context.insert("errorCode".to_string(), code.as_str().to_string());
        }
        self.operation_logging
            .write_operation(OperationLog {
                operation_id: operation_id.to_string(),
                severity: if outcome.is_success() {
                    LogSeverity::Info
                } else {
                    LogSeverity::Warn
                },
                category: "mcp.operation".to_string(),
                message: if outcome.is_success() {
                    "MCP connection test passed".to_string()
                } else {
                    "MCP connection test failed".to_string()
                },
                context,
            })
            .map_err(|error| McpApplicationError::Storage(error.to_string()))
    }

    fn record_catalog_rejection(
        &self,
        server_name: &str,
        error_code: McpFailureCode,
    ) -> Result<(), McpApplicationError> {
        let mut context = BTreeMap::new();
        context.insert("serverName".to_string(), server_name.to_string());
        context.insert("errorCode".to_string(), error_code.as_str().to_string());
        self.diagnostic_logging
            .write_diagnostic(DiagnosticLog {
                severity: LogSeverity::Warn,
                category: "mcp.catalog".to_string(),
                message: "Excluded an invalid cached MCP tool catalog".to_string(),
                context,
            })
            .map_err(|error| McpApplicationError::Storage(error.to_string()))
    }

    fn record_catalog_overflow(
        &self,
        omitted_tools: usize,
        maximum_tools: usize,
    ) -> Result<(), McpApplicationError> {
        let mut context = BTreeMap::new();
        context.insert("omittedTools".to_string(), omitted_tools.to_string());
        context.insert("maximumTools".to_string(), maximum_tools.to_string());
        self.diagnostic_logging
            .write_diagnostic(DiagnosticLog {
                severity: LogSeverity::Warn,
                category: "mcp.catalog".to_string(),
                message: "Omitted MCP tools that exceeded the provider catalog limit".to_string(),
                context,
            })
            .map_err(|error| McpApplicationError::Storage(error.to_string()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionTestPayload {
    success: bool,
    operation_id: Option<String>,
    tools: Vec<ToolPayload>,
    error: Option<String>,
    error_code: Option<&'static str>,
    duration_ms: Option<u64>,
}

impl From<&ConnectionTestResult> for ConnectionTestPayload {
    fn from(result: &ConnectionTestResult) -> Self {
        Self {
            success: result.success,
            operation_id: Some(result.operation_id.clone()),
            tools: result.tools.iter().map(ToolPayload::from).collect(),
            error: result.error.clone(),
            error_code: result.error_code.map(McpFailureCode::as_str),
            duration_ms: Some(result.duration_ms),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolPayload {
    name: String,
    description: Option<String>,
    input_schema: Option<Value>,
}

impl From<&ToolDescriptor> for ToolPayload {
    fn from(tool: &ToolDescriptor) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        }
    }
}

fn operation_error(error: impl std::fmt::Display) -> McpApplicationError {
    McpApplicationError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_operation_payload_keeps_connection_result_contract() {
        let result = ConnectionTestResult {
            success: true,
            operation_id: "op-fixed".to_string(),
            tools: vec![ToolDescriptor {
                name: "search".to_string(),
                description: None,
                input_schema: Some(serde_json::json!({ "type": "object" })),
            }],
            error: None,
            error_code: Some(McpFailureCode::Timeout),
            duration_ms: 17,
        };

        let value = serde_json::to_value(ConnectionTestPayload::from(&result)).expect("payload");

        assert_eq!(value["success"], true);
        assert_eq!(value["operationId"], "op-fixed");
        assert_eq!(value["tools"][0]["inputSchema"]["type"], "object");
        assert_eq!(value["durationMs"], 17);
        assert!(value["error"].is_null());
        assert_eq!(value["errorCode"], "timeout");
    }
}
