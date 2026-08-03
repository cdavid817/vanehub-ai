use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::{McpFailureCode, ServerConfiguration};
use crate::platform::{network, process};
use http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::BTreeMap;
use url::Url;

type StdioProcessParts<'a> = (&'a str, &'a [String], BTreeMap<String, String>);

pub(super) fn stdio_process_parts(
    server: &ServerConfiguration,
) -> Result<StdioProcessParts<'_>, McpRuntimeError> {
    let command = server.command().ok_or_else(|| {
        McpRuntimeError::with_diagnostic(
            McpFailureCode::Validation,
            "stdio MCP server requires command",
        )
    })?;
    process::validate_executable(command).map_err(|error| match error {
        process::ProcessError::InvalidExecutable(message) => {
            runtime_error(McpFailureCode::Validation, message)
        }
        error => runtime_error(McpFailureCode::Spawn, error),
    })?;
    Ok((
        command,
        server.args().unwrap_or_default(),
        server.env().cloned().unwrap_or_default(),
    ))
}

pub(super) fn http_parts(
    server: &ServerConfiguration,
    control: &McpExecutionControl,
) -> Result<(reqwest::Client, Url, HeaderMap), McpRuntimeError> {
    let url = server.url().ok_or_else(|| {
        McpRuntimeError::with_diagnostic(McpFailureCode::Validation, "URL MCP server requires url")
    })?;
    let url = Url::parse(url).map_err(|error| runtime_error(McpFailureCode::Validation, error))?;
    let mut headers = HeaderMap::new();
    for (name, value) in server.headers().cloned().unwrap_or_default() {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| runtime_error(McpFailureCode::Validation, error))?;
        let header_value = HeaderValue::from_str(&value)
            .map_err(|error| runtime_error(McpFailureCode::Validation, error))?;
        headers.insert(header_name, header_value);
    }
    let client = network::no_redirect_http_client(control.remaining()?)
        .map_err(|error| runtime_error(McpFailureCode::Transport, error))?;
    Ok((client, url, headers))
}

pub(super) fn finish_protocol<T, Q, E: std::fmt::Display>(
    result: Result<T, McpRuntimeError>,
    close: Result<Q, E>,
) -> Result<T, McpRuntimeError> {
    match (result, close) {
        (result, Ok(_)) => result,
        (Err(primary), Err(_)) => Err(primary),
        (Ok(_), Err(error)) => Err(runtime_error(McpFailureCode::Cleanup, error)),
    }
}

pub(super) fn finish_protocol_with_status<T, Q, E: std::fmt::Display>(
    result: Result<T, McpRuntimeError>,
    close: Result<Q, E>,
    status: Option<McpFailureCode>,
) -> Result<T, McpRuntimeError> {
    match (result, close) {
        (result, Ok(_)) => result,
        (Err(primary), Err(_)) => Err(primary),
        (Ok(_), Err(error)) => Err(status_error(status, McpFailureCode::Cleanup, error)),
    }
}

pub(super) fn status_error(
    status: Option<McpFailureCode>,
    fallback: McpFailureCode,
    error: impl std::fmt::Display,
) -> McpRuntimeError {
    runtime_error(status.unwrap_or(fallback), error)
}

fn runtime_error(code: McpFailureCode, error: impl std::fmt::Display) -> McpRuntimeError {
    McpRuntimeError::with_diagnostic(code, error.to_string())
}
