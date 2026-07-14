use crate::mcp::models::{McpServerConfig, McpTestResult, McpToolInfo, McpTransportType};
use crate::AppError;
use http::{HeaderName, HeaderValue};
use rmcp::{
    ServiceExt,
    transport::{
        ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MCP_TEST_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn test_connection(config: &McpServerConfig) -> McpTestResult {
    let started = Instant::now();
    let result = tokio::time::timeout(MCP_TEST_TIMEOUT, test_connection_inner(config)).await;
    let duration_ms = Some(started.elapsed().as_millis() as u64);

    match result {
        Ok(Ok(tools)) => McpTestResult {
            success: true,
            tools,
            error: None,
            duration_ms,
        },
        Ok(Err(error)) => McpTestResult {
            success: false,
            tools: Vec::new(),
            error: Some(error.to_string()),
            duration_ms,
        },
        Err(_) => McpTestResult {
            success: false,
            tools: Vec::new(),
            error: Some("MCP connection timed out after 15 seconds.".to_string()),
            duration_ms,
        },
    }
}

async fn test_connection_inner(config: &McpServerConfig) -> Result<Vec<McpToolInfo>, AppError> {
    match config.transport_type {
        McpTransportType::Stdio => test_stdio(config).await,
        McpTransportType::Sse => test_url_transport(config).await,
        McpTransportType::StreamableHttp => Err(AppError::McpConnection(
            "streamable_http is reserved for a later MCP release in VaneHub P1".to_string(),
        )),
    }
}

async fn test_stdio(config: &McpServerConfig) -> Result<Vec<McpToolInfo>, AppError> {
    let command = config
        .command
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation("stdio MCP server requires command".to_string()))?;
    let args = config.args.clone().unwrap_or_default();
    let env = config.env.clone().unwrap_or_default();

    let transport =
        TokioChildProcess::new(tokio::process::Command::new(command).configure(|cmd| {
            cmd.args(args);
            cmd.envs(env);
        }))
        .map_err(|error| AppError::McpConnection(error.to_string()))?;

    let client = ().serve(transport).await.map_err(mcp_error)?;
    let tools = client.peer().list_all_tools().await.map_err(mcp_error)?;
    let _ = client.cancel().await;

    Ok(map_tools(tools))
}

async fn test_url_transport(config: &McpServerConfig) -> Result<Vec<McpToolInfo>, AppError> {
    let url = config
        .url
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation("URL MCP server requires url".to_string()))?;
    let mut headers = HashMap::new();
    for (name, value) in config.headers.clone().unwrap_or_default() {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| AppError::Validation(format!("invalid header name '{name}': {error}")))?;
        let header_value = HeaderValue::from_str(&value)
            .map_err(|error| AppError::Validation(format!("invalid header value for '{name}': {error}")))?;
        headers.insert(header_name, header_value);
    }

    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url.clone()).custom_headers(headers),
    );
    let client = ().serve(transport).await.map_err(mcp_error)?;
    let tools = client.peer().list_all_tools().await.map_err(mcp_error)?;
    let _ = client.cancel().await;

    Ok(map_tools(tools))
}

fn map_tools(tools: Vec<rmcp::model::Tool>) -> Vec<McpToolInfo> {
    tools
        .into_iter()
        .map(|tool| McpToolInfo {
            name: tool.name.into_owned(),
            description: tool.description.map(|description| description.into_owned()),
            input_schema: Some(serde_json::Value::Object(tool.input_schema.as_ref().clone())),
        })
        .collect()
}

fn mcp_error(error: impl std::fmt::Display) -> AppError {
    AppError::McpConnection(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::models::{McpScope, McpServerConfig};
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    fn fixture_path(name: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    #[tokio::test]
    async fn stdio_fixture_initializes_and_lists_tools() {
        let config = McpServerConfig {
            name: "stdio-fixture".to_string(),
            transport_type: McpTransportType::Stdio,
            command: Some("node".to_string()),
            args: Some(vec![fixture_path("mcp_stdio_server.cjs")]),
            env: None,
            url: None,
            headers: None,
            description: None,
            active: true,
            scope: McpScope::User,
            project_path: None,
        };

        let result = test_connection(&config).await;

        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "fixture_echo");
    }

    #[tokio::test]
    async fn http_fixture_initializes_and_lists_tools() {
        let mut child = Command::new("node")
            .arg(fixture_path("mcp_http_server.cjs"))
            .arg("0")
            .stdout(Stdio::piped())
            .spawn()
            .expect("start fixture");
        let stdout = child.stdout.take().expect("stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).expect("ready line");
        let url = line
            .trim()
            .strip_prefix("READY ")
            .expect("ready prefix")
            .to_string();

        let config = McpServerConfig {
            name: "http-fixture".to_string(),
            transport_type: McpTransportType::Sse,
            command: None,
            args: None,
            env: None,
            url: Some(url),
            headers: None,
            description: None,
            active: true,
            scope: McpScope::User,
            project_path: None,
        };

        let result = test_connection(&config).await;
        let _ = child.kill();

        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "fixture_http_echo");
    }
}
