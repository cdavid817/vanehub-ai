use super::connection_mapping::map_tools;
use super::connection_support::{
    finish_protocol, finish_protocol_with_status, http_parts, status_error, stdio_process_parts,
};
use super::legacy_sse::LegacySseTransport;
use super::managed_session::ManagedMcpSession;
use super::runtime_logging::McpRuntimeLogContext;
use super::streamable_http::BoundedStreamableHttpTransport;
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpLimits, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::{
    McpFailureCode, ServerConfiguration, ToolDescriptor, TransportType,
};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::ServiceExt;

pub(super) async fn test_server(
    server: &ServerConfiguration,
    control: &McpExecutionControl,
    log_context: &McpRuntimeLogContext,
) -> Result<Vec<ToolDescriptor>, McpRuntimeError> {
    match server.transport_type() {
        TransportType::Stdio => test_stdio(server, control, log_context).await,
        TransportType::Sse => test_legacy_sse(server, control).await,
        TransportType::StreamableHttp => test_streamable_http(server, control).await,
    }
}

pub(super) async fn call_tool(
    server: &ServerConfiguration,
    params: CallToolRequestParams,
    control: &McpExecutionControl,
    log_context: &McpRuntimeLogContext,
) -> Result<CallToolResult, McpRuntimeError> {
    match server.transport_type() {
        TransportType::Stdio => call_stdio(server, params, control, log_context).await,
        TransportType::Sse => call_legacy_sse(server, params, control).await,
        TransportType::StreamableHttp => call_streamable_http(server, params, control).await,
    }
}

async fn test_stdio(
    server: &ServerConfiguration,
    control: &McpExecutionControl,
    log_context: &McpRuntimeLogContext,
) -> Result<Vec<ToolDescriptor>, McpRuntimeError> {
    let (command, args, environment) = stdio_process_parts(server)?;
    ManagedMcpSession::spawn_stdio(
        command,
        args,
        &environment,
        McpLimits::DEFAULT.protocol_message_bytes,
        McpLimits::DEFAULT.stderr_bytes,
        log_context.clone(),
        |stdout, stdin| async move {
            let client = ()
                .serve((stdout, stdin))
                .await
                .map_err(|error| runtime_error(McpFailureCode::Transport, error))?;
            let result = client
                .peer()
                .list_all_tools()
                .await
                .map(map_tools)
                .map_err(|error| runtime_error(McpFailureCode::Protocol, error));
            let close = client.cancel().await;
            finish_protocol(result, close)
        },
    )?
    .run(control)
    .await
}

async fn call_stdio(
    server: &ServerConfiguration,
    params: CallToolRequestParams,
    control: &McpExecutionControl,
    log_context: &McpRuntimeLogContext,
) -> Result<CallToolResult, McpRuntimeError> {
    let (command, args, environment) = stdio_process_parts(server)?;
    ManagedMcpSession::spawn_stdio(
        command,
        args,
        &environment,
        McpLimits::DEFAULT.protocol_message_bytes,
        McpLimits::DEFAULT.stderr_bytes,
        log_context.clone(),
        move |stdout, stdin| async move {
            let client = ()
                .serve((stdout, stdin))
                .await
                .map_err(|error| runtime_error(McpFailureCode::Transport, error))?;
            let result = client
                .peer()
                .call_tool(params)
                .await
                .map_err(|error| runtime_error(McpFailureCode::Protocol, error));
            let close = client.cancel().await;
            finish_protocol(result, close)
        },
    )?
    .run(control)
    .await
}

async fn test_legacy_sse(
    server: &ServerConfiguration,
    control: &McpExecutionControl,
) -> Result<Vec<ToolDescriptor>, McpRuntimeError> {
    let (client, url, headers) = http_parts(server, control)?;
    let operation_control = control.clone();
    ManagedMcpSession::spawn_http(
        move || async move {
            let (transport, status) = LegacySseTransport::connect(
                client,
                url,
                headers,
                operation_control,
                McpLimits::DEFAULT.protocol_message_bytes,
            )
            .await?;
            let client = ().serve(transport).await.map_err(|error| {
                status_error(status.failure(), McpFailureCode::Transport, error)
            })?;
            let result = client
                .peer()
                .list_all_tools()
                .await
                .map(map_tools)
                .map_err(|error| status_error(status.failure(), McpFailureCode::Protocol, error));
            let close = client.cancel().await;
            finish_protocol_with_status(result, close, status.failure())
        },
        |_| async { Ok(()) },
    )
    .run(control)
    .await
}

async fn call_legacy_sse(
    server: &ServerConfiguration,
    params: CallToolRequestParams,
    control: &McpExecutionControl,
) -> Result<CallToolResult, McpRuntimeError> {
    let (client, url, headers) = http_parts(server, control)?;
    let operation_control = control.clone();
    ManagedMcpSession::spawn_http(
        move || async move {
            let (transport, status) = LegacySseTransport::connect(
                client,
                url,
                headers,
                operation_control,
                McpLimits::DEFAULT.protocol_message_bytes,
            )
            .await?;
            let client = ().serve(transport).await.map_err(|error| {
                status_error(status.failure(), McpFailureCode::Transport, error)
            })?;
            let result =
                client.peer().call_tool(params).await.map_err(|error| {
                    status_error(status.failure(), McpFailureCode::Protocol, error)
                });
            let close = client.cancel().await;
            finish_protocol_with_status(result, close, status.failure())
        },
        |_| async { Ok(()) },
    )
    .run(control)
    .await
}

async fn test_streamable_http(
    server: &ServerConfiguration,
    control: &McpExecutionControl,
) -> Result<Vec<ToolDescriptor>, McpRuntimeError> {
    let (client, url, headers) = http_parts(server, control)?;
    let (transport, status, lease) = BoundedStreamableHttpTransport::new(
        client,
        url,
        headers,
        control.clone(),
        McpLimits::DEFAULT.protocol_message_bytes,
    );
    let shutdown_lease = lease.clone();
    ManagedMcpSession::spawn_http(
        move || async move {
            let client = ().serve(transport).await.map_err(|error| {
                status_error(status.failure(), McpFailureCode::Transport, error)
            })?;
            let result = client
                .peer()
                .list_all_tools()
                .await
                .map(map_tools)
                .map_err(|error| status_error(status.failure(), McpFailureCode::Protocol, error));
            let close = client.cancel().await;
            finish_protocol_with_status(result, close, status.failure())
        },
        move |deadline| async move { shutdown_lease.shutdown(deadline).await },
    )
    .run(control)
    .await
}

async fn call_streamable_http(
    server: &ServerConfiguration,
    params: CallToolRequestParams,
    control: &McpExecutionControl,
) -> Result<CallToolResult, McpRuntimeError> {
    let (client, url, headers) = http_parts(server, control)?;
    let (transport, status, lease) = BoundedStreamableHttpTransport::new(
        client,
        url,
        headers,
        control.clone(),
        McpLimits::DEFAULT.protocol_message_bytes,
    );
    let shutdown_lease = lease.clone();
    ManagedMcpSession::spawn_http(
        move || async move {
            let client = ().serve(transport).await.map_err(|error| {
                status_error(status.failure(), McpFailureCode::Transport, error)
            })?;
            let result =
                client.peer().call_tool(params).await.map_err(|error| {
                    status_error(status.failure(), McpFailureCode::Protocol, error)
                });
            let close = client.cancel().await;
            finish_protocol_with_status(result, close, status.failure())
        },
        move |deadline| async move { shutdown_lease.shutdown(deadline).await },
    )
    .run(control)
    .await
}

fn runtime_error(code: McpFailureCode, error: impl std::fmt::Display) -> McpRuntimeError {
    McpRuntimeError::with_diagnostic(code, error.to_string())
}
