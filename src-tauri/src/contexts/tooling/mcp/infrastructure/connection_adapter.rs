use crate::contexts::tooling::mcp::application::McpConnectionPort;
use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, ServerConfiguration, ToolDescriptor, TransportType,
};
use crate::platform::{network, process};
use async_trait::async_trait;
use http::{HeaderName, HeaderValue};
use rmcp::{
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig, ConfigureCommandExt,
        StreamableHttpClientTransport, TokioChildProcess,
    },
    ServiceExt,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MCP_TEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy)]
pub(crate) struct RmcpConnectionAdapter {
    timeout: Duration,
}

impl Default for RmcpConnectionAdapter {
    fn default() -> Self {
        Self {
            timeout: MCP_TEST_TIMEOUT,
        }
    }
}

impl RmcpConnectionAdapter {
    #[cfg(test)]
    fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    async fn test_inner(
        &self,
        server: &ServerConfiguration,
    ) -> Result<Vec<ToolDescriptor>, String> {
        match server.transport_type() {
            TransportType::Stdio => test_stdio(server).await,
            TransportType::Sse => test_http(server, self.timeout).await,
            TransportType::StreamableHttp => Err(connection_error(
                "streamable_http is reserved for a later MCP release in VaneHub P1",
            )),
        }
    }
}

#[async_trait]
impl McpConnectionPort for RmcpConnectionAdapter {
    async fn test(&self, server: &ServerConfiguration) -> ConnectionOutcome {
        let started = Instant::now();
        let result = tokio::time::timeout(self.timeout, self.test_inner(server)).await;
        let duration_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(Ok(tools)) => ConnectionOutcome::connected(tools, duration_ms),
            Ok(Err(error)) => ConnectionOutcome::failed(error, duration_ms),
            Err(_) => ConnectionOutcome::failed(
                format!(
                    "MCP connection timed out after {} seconds.",
                    self.timeout.as_secs().max(1)
                ),
                duration_ms,
            ),
        }
    }
}

async fn test_stdio(server: &ServerConfiguration) -> Result<Vec<ToolDescriptor>, String> {
    let process = stdio_command(server)?;
    let transport =
        TokioChildProcess::new(process).map_err(|error| connection_error(error.to_string()))?;
    let client = ().serve(transport).await.map_err(rmcp_error)?;
    let tools = client.peer().list_all_tools().await.map_err(rmcp_error)?;
    let _ = client.cancel().await;
    Ok(map_tools(tools))
}

fn stdio_command(server: &ServerConfiguration) -> Result<tokio::process::Command, String> {
    let command = server
        .command()
        .ok_or_else(|| validation_error("stdio MCP server requires command"))?;
    let mut process = process::tokio_command(command).map_err(|error| match error {
        process::ProcessError::InvalidExecutable(message) => validation_error(message),
        error => connection_error(error.to_string()),
    })?;
    process.args(server.args().unwrap_or_default());
    if let Some(environment) = server.env() {
        process.envs(environment);
    }
    Ok(process.configure(|_| {}))
}

async fn test_http(
    server: &ServerConfiguration,
    timeout: Duration,
) -> Result<Vec<ToolDescriptor>, String> {
    let url = server
        .url()
        .ok_or_else(|| validation_error("URL MCP server requires url"))?;
    let mut headers = HashMap::new();
    for (name, value) in server.headers().cloned().unwrap_or_default() {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| validation_error(format!("invalid header name '{name}': {error}")))?;
        let header_value = HeaderValue::from_str(&value).map_err(|error| {
            validation_error(format!("invalid header value for '{name}': {error}"))
        })?;
        headers.insert(header_name, header_value);
    }
    let client = network::no_redirect_http_client(timeout).map_err(|error| error.to_string())?;
    let config =
        StreamableHttpClientTransportConfig::with_uri(url.to_string()).custom_headers(headers);
    let transport = StreamableHttpClientTransport::with_client(client, config);
    let client = ().serve(transport).await.map_err(rmcp_error)?;
    let tools = client.peer().list_all_tools().await.map_err(rmcp_error)?;
    let _ = client.cancel().await;
    Ok(map_tools(tools))
}

fn map_tools(tools: Vec<rmcp::model::Tool>) -> Vec<ToolDescriptor> {
    tools
        .into_iter()
        .map(|tool| ToolDescriptor {
            name: tool.name.into_owned(),
            description: tool.description.map(|description| description.into_owned()),
            input_schema: Some(serde_json::Value::Object(
                tool.input_schema.as_ref().clone(),
            )),
        })
        .collect()
}

fn rmcp_error(error: impl std::fmt::Display) -> String {
    connection_error(error.to_string())
}

fn connection_error(message: impl std::fmt::Display) -> String {
    format!("MCP connection failed: {message}")
}

fn validation_error(message: impl std::fmt::Display) -> String {
    format!("validation error: {message}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::mcp::domain::{Scope, ServerConfigurationDraft, TransportType};
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

    fn server(
        transport_type: TransportType,
        command: Option<String>,
        args: Option<Vec<String>>,
        url: Option<String>,
    ) -> ServerConfiguration {
        ServerConfiguration::create(ServerConfigurationDraft {
            name: "fixture-tools".to_string(),
            transport_type,
            command,
            args,
            env: None,
            url,
            headers: None,
            description: None,
            active: true,
            scope: Scope::User,
            project_path: None,
        })
        .expect("server")
    }

    #[tokio::test]
    async fn stdio_fixture_initializes_and_lists_tools_through_platform_process() {
        let server = server(
            TransportType::Stdio,
            Some("node".to_string()),
            Some(vec![fixture_path("mcp_stdio_server.cjs")]),
            None,
        );

        let outcome = RmcpConnectionAdapter::default().test(&server).await;

        assert!(outcome.is_success(), "{:?}", outcome.error());
        assert_eq!(outcome.tools().len(), 1);
        assert_eq!(outcome.tools()[0].name, "fixture_echo");
    }

    #[tokio::test]
    async fn http_fixture_uses_platform_network_client_and_lists_tools() {
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
        let server = server(TransportType::Sse, None, None, Some(url));

        let outcome = RmcpConnectionAdapter::default().test(&server).await;
        let _ = child.kill();
        let _ = child.wait();

        assert!(outcome.is_success(), "{:?}", outcome.error());
        assert_eq!(outcome.tools()[0].name, "fixture_http_echo");
    }

    #[tokio::test]
    async fn total_timeout_returns_the_stable_connection_result_shape() {
        let server = server(
            TransportType::Stdio,
            Some("node".to_string()),
            Some(vec![
                "-e".to_string(),
                "setTimeout(() => {}, 5000)".to_string(),
            ]),
            None,
        );

        let outcome = RmcpConnectionAdapter::with_timeout(Duration::from_millis(100))
            .test(&server)
            .await;

        assert!(!outcome.is_success());
        assert_eq!(
            outcome.error(),
            Some("MCP connection timed out after 1 seconds.")
        );
        assert!(outcome.duration_ms() >= 90);
    }

    #[test]
    fn stdio_arguments_remain_literal_and_are_not_interpreted_by_a_shell() {
        let server = server(
            TransportType::Stdio,
            Some("node".to_string()),
            Some(vec![
                "literal; echo should-not-run".to_string(),
                "$(also-literal)".to_string(),
            ]),
            None,
        );

        let command = stdio_command(&server).expect("command");
        let args = command
            .as_std()
            .get_args()
            .map(|value| value.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec!["literal; echo should-not-run", "$(also-literal)"]
        );
    }

    #[tokio::test]
    async fn invalid_headers_and_reserved_transport_keep_legacy_error_messages() {
        let mut draft = ServerConfigurationDraft {
            name: "http-tools".to_string(),
            transport_type: TransportType::Sse,
            command: None,
            args: None,
            env: None,
            url: Some("http://localhost:1/mcp".to_string()),
            headers: Some([("bad header".to_string(), "value".to_string())].into()),
            description: None,
            active: true,
            scope: Scope::User,
            project_path: None,
        };
        let invalid_header = ServerConfiguration::create(draft.clone()).expect("server");
        let outcome = RmcpConnectionAdapter::default().test(&invalid_header).await;
        assert!(outcome
            .error()
            .is_some_and(|error| error.starts_with("validation error: invalid header name")));

        draft.transport_type = TransportType::StreamableHttp;
        draft.headers = None;
        let reserved = ServerConfiguration::create(draft).expect("server");
        let outcome = RmcpConnectionAdapter::default().test(&reserved).await;
        assert_eq!(
            outcome.error(),
            Some(
                "MCP connection failed: streamable_http is reserved for a later MCP release in VaneHub P1"
            )
        );
    }
}
