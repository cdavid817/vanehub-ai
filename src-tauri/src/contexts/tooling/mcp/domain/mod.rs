use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

const INVALID_NAME_MESSAGE: &str =
    "MCP server name must be kebab-case letters, digits, and hyphens";

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum McpDomainError {
    #[error("{INVALID_NAME_MESSAGE}")]
    InvalidServerName,
    #[error("stdio MCP server requires command")]
    MissingStdioCommand,
    #[error("URL MCP server requires url")]
    MissingUrl,
    #[error("project MCP server requires project path")]
    MissingProjectPath,
    #[error("unsupported MCP transport: {0}")]
    InvalidTransport(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpFailureCode {
    Validation,
    Spawn,
    Timeout,
    Cancelled,
    Protocol,
    UpstreamHttp,
    LimitExceeded,
    Transport,
    Cleanup,
}

impl McpFailureCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Spawn => "spawn",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Protocol => "protocol",
            Self::UpstreamHttp => "upstream_http",
            Self::LimitExceeded => "limit_exceeded",
            Self::Transport => "transport",
            Self::Cleanup => "cleanup",
        }
    }

    pub(crate) fn safe_message(self) -> &'static str {
        match self {
            Self::Validation => "MCP request validation failed.",
            Self::Spawn => "The MCP server could not be started.",
            Self::Timeout => "The MCP operation timed out.",
            Self::Cancelled => "The MCP operation was cancelled.",
            Self::Protocol => "The MCP server returned an invalid protocol response.",
            Self::UpstreamHttp => "The MCP HTTP server returned an error.",
            Self::LimitExceeded => "The MCP operation exceeded a safety limit.",
            Self::Transport => "The MCP transport failed.",
            Self::Cleanup => "The MCP operation could not release all resources.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ServerName(String);

impl ServerName {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, McpDomainError> {
        let value = value.into();
        let valid = !value.is_empty()
            && !value.starts_with('-')
            && !value.ends_with('-')
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !valid {
            return Err(McpDomainError::InvalidServerName);
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportType {
    Stdio,
    Sse,
    StreamableHttp,
}

impl TransportType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Sse => "sse",
            Self::StreamableHttp => "streamable_http",
        }
    }

    pub(crate) fn from_persisted(value: &str) -> Result<Self, McpDomainError> {
        match value {
            "stdio" => Ok(Self::Stdio),
            "sse" => Ok(Self::Sse),
            "streamable_http" => Ok(Self::StreamableHttp),
            other => Err(McpDomainError::InvalidTransport(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    User,
    Project,
}

impl Scope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }

    pub(crate) fn from_persisted(value: &str) -> Self {
        match value {
            "project" => Self::Project,
            _ => Self::User,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerConfigurationDraft {
    pub(crate) name: String,
    pub(crate) transport_type: TransportType,
    pub(crate) command: Option<String>,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) env: Option<BTreeMap<String, String>>,
    pub(crate) url: Option<String>,
    pub(crate) headers: Option<BTreeMap<String, String>>,
    pub(crate) description: Option<String>,
    pub(crate) active: bool,
    pub(crate) scope: Scope,
    pub(crate) project_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerConfiguration {
    name: ServerName,
    transport_type: TransportType,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
    url: Option<String>,
    headers: Option<BTreeMap<String, String>>,
    description: Option<String>,
    active: bool,
    scope: Scope,
    project_path: Option<String>,
}

impl ServerConfiguration {
    pub(crate) fn create(draft: ServerConfigurationDraft) -> Result<Self, McpDomainError> {
        let name = ServerName::parse(draft.name)?;
        match draft.transport_type {
            TransportType::Stdio
                if draft
                    .command
                    .as_deref()
                    .is_none_or(|command| command.trim().is_empty()) =>
            {
                return Err(McpDomainError::MissingStdioCommand);
            }
            TransportType::Sse | TransportType::StreamableHttp
                if draft.url.as_deref().is_none_or(|url| url.trim().is_empty()) =>
            {
                return Err(McpDomainError::MissingUrl);
            }
            _ => {}
        }
        let project_path = match draft.scope {
            Scope::User => None,
            Scope::Project => Some(
                draft
                    .project_path
                    .filter(|path| !path.trim().is_empty())
                    .ok_or(McpDomainError::MissingProjectPath)?,
            ),
        };
        Ok(Self {
            name,
            transport_type: draft.transport_type,
            command: draft.command,
            args: draft.args,
            env: draft.env,
            url: draft.url,
            headers: draft.headers,
            description: draft.description,
            active: draft.active,
            scope: draft.scope,
            project_path,
        })
    }

    pub(crate) fn name(&self) -> &ServerName {
        &self.name
    }

    pub(crate) fn transport_type(&self) -> TransportType {
        self.transport_type
    }

    pub(crate) fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    pub(crate) fn args(&self) -> Option<&[String]> {
        self.args.as_deref()
    }

    pub(crate) fn env(&self) -> Option<&BTreeMap<String, String>> {
        self.env.as_ref()
    }

    pub(crate) fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub(crate) fn headers(&self) -> Option<&BTreeMap<String, String>> {
        self.headers.as_ref()
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn scope(&self) -> Scope {
        self.scope
    }

    pub(crate) fn project_path(&self) -> Option<&str> {
        self.project_path.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolDescriptor {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) input_schema: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionStatus {
    Connected,
    Disconnected,
    Error,
    Disabled,
}

impl ConnectionStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Error => "error",
            Self::Disabled => "disabled",
        }
    }

    pub(crate) fn from_persisted(value: Option<&str>) -> Self {
        match value {
            Some("connected") => Self::Connected,
            Some("error") => Self::Error,
            Some("disabled") => Self::Disabled,
            _ => Self::Disconnected,
        }
    }

    pub(crate) fn visible_for(self, active: bool) -> Self {
        if active {
            self
        } else {
            Self::Disabled
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConnectionOutcome {
    Connected {
        tools: Vec<ToolDescriptor>,
        duration_ms: u64,
    },
    Failed {
        error: String,
        error_code: Option<McpFailureCode>,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ServerStatus {
    pub(crate) name: ServerName,
    pub(crate) connection_status: ConnectionStatus,
    pub(crate) tools: Vec<ToolDescriptor>,
    pub(crate) last_connected: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) error_code: Option<McpFailureCode>,
    pub(crate) duration_ms: Option<u64>,
}

impl ConnectionOutcome {
    pub(crate) fn connected(tools: Vec<ToolDescriptor>, duration_ms: u64) -> Self {
        Self::Connected { tools, duration_ms }
    }

    #[cfg(test)]
    pub(crate) fn failed(error: impl Into<String>, duration_ms: u64) -> Self {
        Self::Failed {
            error: error.into(),
            error_code: None,
            duration_ms,
        }
    }

    pub(crate) fn failed_with_code(
        error: impl Into<String>,
        error_code: McpFailureCode,
        duration_ms: u64,
    ) -> Self {
        Self::Failed {
            error: error.into(),
            error_code: Some(error_code),
            duration_ms,
        }
    }

    pub(crate) fn is_success(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    pub(crate) fn status(&self) -> ConnectionStatus {
        match self {
            Self::Connected { .. } => ConnectionStatus::Connected,
            Self::Failed { .. } => ConnectionStatus::Error,
        }
    }

    pub(crate) fn tools(&self) -> &[ToolDescriptor] {
        match self {
            Self::Connected { tools, .. } => tools,
            Self::Failed { .. } => &[],
        }
    }

    pub(crate) fn error(&self) -> Option<&str> {
        match self {
            Self::Connected { .. } => None,
            Self::Failed { error, .. } => Some(error),
        }
    }

    pub(crate) fn error_code(&self) -> Option<McpFailureCode> {
        match self {
            Self::Connected { .. } => None,
            Self::Failed { error_code, .. } => *error_code,
        }
    }

    pub(crate) fn duration_ms(&self) -> u64 {
        match self {
            Self::Connected { duration_ms, .. } | Self::Failed { duration_ms, .. } => *duration_ms,
        }
    }
}

/// The result of a single `tools/call` invocation against an MCP server — kept separate from
/// `ConnectionOutcome` (which describes a connectivity *test*, not a tool result) even though
/// both represent "success or failure as data, never a Rust `Result` error" for the same reason:
/// a tool that ran but reported failure, or a server that couldn't be reached, are both outcomes
/// the calling agent should see and react to, not exceptions in this layer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolCallOutcome {
    pub(crate) content: String,
    pub(crate) is_error: bool,
    pub(crate) error_code: Option<McpFailureCode>,
}

impl ToolCallOutcome {
    pub(crate) fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            error_code: None,
        }
    }

    pub(crate) fn failed(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
            error_code: None,
        }
    }

    pub(crate) fn failed_with_code(content: impl Into<String>, error_code: McpFailureCode) -> Self {
        Self {
            content: content.into(),
            is_error: true,
            error_code: Some(error_code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(transport_type: TransportType) -> ServerConfigurationDraft {
        ServerConfigurationDraft {
            name: "fixture-tools-2".to_string(),
            transport_type,
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: None,
            url: Some("http://localhost:8000/mcp".to_string()),
            headers: None,
            description: None,
            active: true,
            scope: Scope::User,
            project_path: Some("ignored-for-user".to_string()),
        }
    }

    #[test]
    fn server_identity_accepts_only_stable_kebab_case_names() {
        assert_eq!(
            ServerName::parse("filesystem-tools-2")
                .expect("valid name")
                .as_str(),
            "filesystem-tools-2"
        );
        for invalid in ["", "Bad_Name", "-leading", "trailing-", "two words"] {
            assert_eq!(
                ServerName::parse(invalid).expect_err("invalid name"),
                McpDomainError::InvalidServerName
            );
        }
    }

    #[test]
    fn transport_configuration_requires_the_matching_endpoint() {
        let mut stdio = draft(TransportType::Stdio);
        stdio.command = Some("  ".to_string());
        assert_eq!(
            ServerConfiguration::create(stdio).expect_err("missing command"),
            McpDomainError::MissingStdioCommand
        );

        let mut http = draft(TransportType::Sse);
        http.url = None;
        assert_eq!(
            ServerConfiguration::create(http).expect_err("missing url"),
            McpDomainError::MissingUrl
        );
    }

    #[test]
    fn persisted_transport_values_are_explicit_and_unknown_values_fail_closed() {
        assert_eq!(
            TransportType::from_persisted("stdio").expect("stdio"),
            TransportType::Stdio
        );
        assert_eq!(
            TransportType::from_persisted("sse").expect("sse"),
            TransportType::Sse
        );
        assert_eq!(
            TransportType::from_persisted("streamable_http").expect("http"),
            TransportType::StreamableHttp
        );
        assert_eq!(
            TransportType::from_persisted("future_transport").expect_err("unknown"),
            McpDomainError::InvalidTransport("future_transport".to_string())
        );
    }

    #[test]
    fn scope_controls_project_ownership_without_leaking_stale_paths() {
        let user = ServerConfiguration::create(draft(TransportType::Stdio)).expect("user config");
        assert_eq!(user.project_path(), None);

        let mut project = draft(TransportType::Stdio);
        project.scope = Scope::Project;
        project.project_path = None;
        assert_eq!(
            ServerConfiguration::create(project).expect_err("missing project"),
            McpDomainError::MissingProjectPath
        );
    }

    #[test]
    fn connection_outcomes_have_explicit_status_tools_error_and_duration_semantics() {
        let tool = ToolDescriptor {
            name: "search".to_string(),
            description: None,
            input_schema: Some(serde_json::json!({ "type": "object" })),
        };
        let success = ConnectionOutcome::connected(vec![tool.clone()], 17);
        assert!(success.is_success());
        assert_eq!(success.status(), ConnectionStatus::Connected);
        assert_eq!(success.tools(), &[tool]);
        assert_eq!(success.error(), None);
        assert_eq!(success.duration_ms(), 17);

        let failure = ConnectionOutcome::failed("handshake failed", 23);
        assert!(!failure.is_success());
        assert_eq!(failure.status(), ConnectionStatus::Error);
        assert!(failure.tools().is_empty());
        assert_eq!(failure.error(), Some("handshake failed"));
        assert_eq!(failure.error_code(), None);
        assert_eq!(failure.duration_ms(), 23);
        assert_eq!(
            ConnectionStatus::Connected.visible_for(false),
            ConnectionStatus::Disabled
        );
    }

    #[test]
    fn tool_call_outcomes_have_explicit_content_and_error_semantics() {
        let success = ToolCallOutcome::success("42");
        assert_eq!(success.content, "42");
        assert!(!success.is_error);
        assert_eq!(success.error_code, None);

        let failure = ToolCallOutcome::failed("boom");
        assert_eq!(failure.content, "boom");
        assert!(failure.is_error);
        assert_eq!(failure.error_code, None);
    }

    #[test]
    fn failure_codes_have_stable_safe_values() {
        let codes = [
            (McpFailureCode::Validation, "validation"),
            (McpFailureCode::Spawn, "spawn"),
            (McpFailureCode::Timeout, "timeout"),
            (McpFailureCode::Cancelled, "cancelled"),
            (McpFailureCode::Protocol, "protocol"),
            (McpFailureCode::UpstreamHttp, "upstream_http"),
            (McpFailureCode::LimitExceeded, "limit_exceeded"),
            (McpFailureCode::Transport, "transport"),
            (McpFailureCode::Cleanup, "cleanup"),
        ];

        for (code, expected) in codes {
            assert_eq!(code.as_str(), expected);
            assert!(!code.safe_message().is_empty());
        }
    }
}
