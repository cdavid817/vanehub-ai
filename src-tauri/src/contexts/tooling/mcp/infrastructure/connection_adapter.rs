use super::connection_mapping::{map_call_result, validate_call_tool};
use super::connection_session;
use super::runtime_logging::{self, McpRuntimeLogContext};
use crate::contexts::tooling::mcp::application::{
    McpConnectionPort, McpExecutionControl, McpRuntimeError,
};
use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, ServerConfiguration, ToolCallOutcome, ToolDescriptor, TransportType,
};
use async_trait::async_trait;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RmcpConnectionAdapter;

impl RmcpConnectionAdapter {
    async fn test_inner(
        &self,
        server: &ServerConfiguration,
        control: &McpExecutionControl,
        log_context: &McpRuntimeLogContext,
    ) -> Result<Vec<ToolDescriptor>, McpRuntimeError> {
        connection_session::test_server(server, control, log_context).await
    }

    async fn call_tool_inner(
        &self,
        server: &ServerConfiguration,
        tool_name: &str,
        arguments: serde_json::Value,
        control: &McpExecutionControl,
        log_context: &McpRuntimeLogContext,
    ) -> Result<ToolCallOutcome, McpRuntimeError> {
        let params = validate_call_tool(tool_name, arguments)?;
        connection_session::call_tool(server, params, control, log_context)
            .await
            .map(map_call_result)
    }
}

#[async_trait]
impl McpConnectionPort for RmcpConnectionAdapter {
    async fn test(
        &self,
        server: &ServerConfiguration,
        control: &McpExecutionControl,
        operation_id: Option<&str>,
    ) -> ConnectionOutcome {
        let started = Instant::now();
        let log_context = McpRuntimeLogContext::for_server(server, operation_id);
        if server.transport_type() == TransportType::Stdio {
            runtime_logging::record_command_start(&log_context);
        }
        let result = self.test_inner(server, control, &log_context).await;
        let duration_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(tools) => {
                runtime_logging::record_success(&log_context, started.elapsed());
                ConnectionOutcome::connected(tools, duration_ms)
            }
            Err(error) => {
                runtime_logging::record_error(&log_context, &error, started.elapsed());
                ConnectionOutcome::failed_with_code(error.to_string(), error.code(), duration_ms)
            }
        }
    }

    async fn call_tool(
        &self,
        server: &ServerConfiguration,
        tool_name: &str,
        arguments: serde_json::Value,
        control: &McpExecutionControl,
    ) -> ToolCallOutcome {
        let started = Instant::now();
        let log_context = McpRuntimeLogContext::for_server(server, None);
        if server.transport_type() == TransportType::Stdio {
            runtime_logging::record_command_start(&log_context);
        }
        let result = self
            .call_tool_inner(server, tool_name, arguments, control, &log_context)
            .await;
        match result {
            Ok(outcome) => {
                runtime_logging::record_success(&log_context, started.elapsed());
                outcome
            }
            Err(error) => {
                runtime_logging::record_error(&log_context, &error, started.elapsed());
                ToolCallOutcome::failed_with_code(error.to_string(), error.code())
            }
        }
    }
}
