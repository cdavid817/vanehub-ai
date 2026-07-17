use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use thiserror::Error;

mod cli_parameters;
mod command_safety;
mod logging;
mod mcp;
mod network_proxy;
mod sdk;
mod skills;
mod tasks;

#[derive(Debug, Error)]
enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("agent not found: {0}")]
    AgentNotFound(String),
    #[error("agent is unavailable: {0}")]
    AgentUnavailable(String),
    #[error("unsupported interaction mode: {0}")]
    UnsupportedInteractionMode(String),
    #[error("no active agent selected")]
    NoActiveAgent,
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("launch failed: {0}")]
    LaunchFailed(String),
    #[error("MCP server not found: {0}")]
    McpServerNotFound(String),
    #[error("MCP connection failed: {0}")]
    McpConnection(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum NativeLogLevel {
    Error,
    Info,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeLogEvent<'a> {
    level: NativeLogLevel,
    category: &'a str,
    message: &'a str,
}

fn record_native_log(level: NativeLogLevel, category: &str, message: &str) {
    let event = NativeLogEvent {
        level,
        category,
        message,
    };
    match serde_json::to_string(&event) {
        Ok(line) => eprintln!("{line}"),
        Err(_) => eprintln!("[{category}] {message}"),
    }
    let mut context = BTreeMap::new();
    context.insert("source".to_string(), "native".to_string());
    let _ = logging::write_message(
        &fallback_log_dir(),
        match level {
            NativeLogLevel::Error => logging::LogLevel::Error,
            NativeLogLevel::Info => logging::LogLevel::Info,
        },
        category,
        message,
        context,
    );
}

fn record_native_error(category: &str, error: &AppError) {
    record_native_log(NativeLogLevel::Error, category, &error.to_string());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum InteractionMode {
    Browser,
    NativeDesktop,
    Cli,
}

impl InteractionMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::NativeDesktop => "native-desktop",
            Self::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum AvailabilityState {
    Available,
    Unavailable,
    NeedsAuth,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SessionLifecycleState {
    Idle,
    Starting,
    Running,
    Failed,
    Stopped,
}

impl SessionLifecycleState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchMetadata {
    kind: String,
    command: Option<String>,
    url: Option<String>,
    executable_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentRegistryEntry {
    id: String,
    display_name: String,
    provider: String,
    managed_sdk_dependency_id: Option<String>,
    launch: LaunchMetadata,
    supported_interaction_modes: Vec<InteractionMode>,
    availability_state: AvailabilityState,
    unavailable_reason: Option<String>,
    capability_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliToolStatus {
    agent_id: String,
    display_name: String,
    provider: String,
    executable_name: String,
    package_name: String,
    installed: Option<bool>,
    current_version: Option<String>,
    latest_version: Option<String>,
    available_versions: Vec<String>,
    detected_path: Option<String>,
    install_command: String,
    last_checked_at: Option<String>,
    last_error: Option<String>,
    last_operation_id: Option<String>,
    version_check_status: CliVersionCheckStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CliVersionCheckStatus {
    Unsupported,
    NotDetected,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy)]
struct CliToolDefinition {
    agent_id: &'static str,
    display_name: &'static str,
    provider: &'static str,
    executable_name: &'static str,
    package_name: &'static str,
}

const CLI_TOOL_DEFINITIONS: [CliToolDefinition; 4] = [
    CliToolDefinition {
        agent_id: "claude-code",
        display_name: "Anthropic Claude Code CLI",
        provider: "Anthropic",
        executable_name: "claude",
        package_name: "@anthropic-ai/claude-code",
    },
    CliToolDefinition {
        agent_id: "codex-cli",
        display_name: "OpenAI Codex CLI",
        provider: "OpenAI",
        executable_name: "codex",
        package_name: "@openai/codex",
    },
    CliToolDefinition {
        agent_id: "gemini-cli",
        display_name: "Google Gemini CLI",
        provider: "Google",
        executable_name: "gemini",
        package_name: "@google/gemini-cli",
    },
    CliToolDefinition {
        agent_id: "opencode",
        display_name: "OpenCode CLI",
        provider: "OpenCode",
        executable_name: "opencode",
        package_name: "opencode-ai",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowState {
    active_agent_id: Option<String>,
    active_interaction_mode: Option<InteractionMode>,
    lifecycle_state: SessionLifecycleState,
    intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadinessStatus {
    ready: bool,
    reason: Option<String>,
    requires_authentication: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchResult {
    operation_id: Option<String>,
    workflow: WorkflowState,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionDetails {
    agent_id: Option<String>,
    interaction_mode: Option<InteractionMode>,
    lifecycle_state: SessionLifecycleState,
    adapter: String,
    details: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    application_language: String,
    font_size: String,
    theme: String,
    default_folder_path: String,
    log_directory: String,
    network_proxy_url: String,
    network_proxy_bypass: String,
    logging_policy: logging::LoggingPolicy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingInput {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NodeInfo {
    available: bool,
    path: Option<String>,
    version: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    id: String,
    title: String,
    agent_id: String,
    interaction_mode: InteractionMode,
    lifecycle_state: SessionLifecycleState,
    folder: Option<String>,
    project_path: Option<String>,
    worktree_path: Option<String>,
    worktree_name: Option<String>,
    worktree_branch: Option<String>,
    runtime_session_id: Option<String>,
    pinned: bool,
    archived: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct KnownProject {
    path: String,
    display_name: String,
    is_git: bool,
    last_opened_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ProjectInspection {
    path: String,
    display_name: String,
    is_git: bool,
    git_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionInput {
    agent_id: String,
    interaction_mode: InteractionMode,
    title: Option<String>,
    folder: Option<String>,
    project_path: Option<String>,
    worktree: Option<CreateWorktreeInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateWorktreeInput {
    enabled: bool,
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatConfig {
    agent_id: String,
    interaction_mode: InteractionMode,
    permission_mode: String,
    provider_id: Option<String>,
    model_id: Option<String>,
    reasoning_depth: Option<String>,
    streaming: bool,
    thinking: bool,
    long_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ToolUseBlock {
    id: String,
    name: String,
    input: Option<serde_json::Value>,
    output: Option<serde_json::Value>,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenUsage {
    input: i64,
    output: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum UsageStatisticsRange {
    Today,
    Last7Days,
    Last30Days,
    All,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageStatistics {
    range: UsageStatisticsRange,
    total_tokens: i64,
    input_tokens: i64,
    output_tokens: i64,
    counted_messages: i64,
    counted_sessions: i64,
    generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatMessage {
    id: String,
    session_id: String,
    role: String,
    content: String,
    status: String,
    tool_use: Option<Vec<ToolUseBlock>>,
    thinking_content: Option<String>,
    token_usage: Option<TokenUsage>,
    error: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum ChatStreamEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        session_id: String,
        message_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Token {
        session_id: String,
        message_id: String,
        content_delta: String,
    },
    #[serde(rename_all = "camelCase")]
    Thinking {
        session_id: String,
        message_id: String,
        content_delta: String,
    },
    #[serde(rename_all = "camelCase")]
    ToolUse {
        session_id: String,
        message_id: String,
        tool_use: ToolUseBlock,
    },
    #[serde(rename_all = "camelCase")]
    Completed {
        session_id: String,
        message_id: String,
        token_usage: Option<TokenUsage>,
    },
    #[serde(rename_all = "camelCase")]
    Failed {
        session_id: String,
        message_id: String,
        error: String,
    },
    #[serde(rename_all = "camelCase")]
    Cancelled {
        session_id: String,
        message_id: String,
    },
}

#[derive(Debug, PartialEq)]
enum ParsedAgentEvent {
    Token(String),
    Thinking(String),
    ToolUse(ToolUseBlock),
    SessionId(String),
    Completed,
    Failed(String),
    Empty,
}

trait AgentOutputParser {
    fn parse_line(&self, line: &str) -> ParsedAgentEvent;
}

fn parser_for_agent(agent_id: &str) -> Box<dyn AgentOutputParser> {
    if agent_id == "claude-code" {
        Box::new(ClaudeCodeParser)
    } else if matches!(agent_id, "codex-cli" | "gemini-cli" | "opencode") {
        Box::new(StructuredJsonParser)
    } else {
        Box::new(GenericLineParser)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PromptDelivery {
    Stdin,
    Argument,
}

#[derive(Debug, Clone, PartialEq)]
struct CliInvocationSpec {
    executable: String,
    args: Vec<String>,
    prompt_delivery: PromptDelivery,
}

fn build_cli_invocation_spec(
    agent: &AgentRegistryEntry,
    executable: String,
    prompt: &str,
    runtime_session_id: Option<&str>,
    managed_args: &[String],
) -> Result<CliInvocationSpec, String> {
    let mut args = Vec::new();
    let prompt_delivery = match agent.id.as_str() {
        "claude-code" => {
            args.extend_from_slice(managed_args);
            args.extend([
                "-p".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--verbose".to_string(),
            ]);
            if let Some(session_id) = runtime_session_id.filter(|value| !value.trim().is_empty()) {
                args.extend(["--resume".to_string(), session_id.to_string()]);
            }
            PromptDelivery::Stdin
        }
        "codex-cli" => {
            args.extend(
                managed_args
                    .iter()
                    .filter(|argument| argument.as_str() != "--ephemeral")
                    .cloned(),
            );
            args.push("exec".to_string());
            if let Some(session_id) = runtime_session_id.filter(|value| !value.trim().is_empty()) {
                args.extend(["resume".to_string(), session_id.to_string()]);
            }
            if managed_args
                .iter()
                .any(|argument| argument == "--ephemeral")
            {
                args.push("--ephemeral".to_string());
            }
            args.extend(["--json".to_string(), "--".to_string(), "-".to_string()]);
            PromptDelivery::Stdin
        }
        "gemini-cli" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = runtime_session_id.filter(|value| !value.trim().is_empty()) {
                args.extend(["--resume".to_string(), session_id.to_string()]);
            }
            args.extend([
                "-p".to_string(),
                prompt.to_string(),
                "-o".to_string(),
                "stream-json".to_string(),
            ]);
            PromptDelivery::Argument
        }
        "opencode" => {
            args.push("run".to_string());
            args.extend_from_slice(managed_args);
            if let Some(session_id) = runtime_session_id.filter(|value| !value.trim().is_empty()) {
                args.extend(["--session".to_string(), session_id.to_string()]);
            }
            args.extend([
                "--format".to_string(),
                "json".to_string(),
                prompt.to_string(),
            ]);
            PromptDelivery::Argument
        }
        other => {
            return Err(format!(
                "{} is not supported by the CLI chat runtime.",
                other
            ));
        }
    };
    Ok(CliInvocationSpec {
        executable,
        args,
        prompt_delivery,
    })
}

fn mapped_chat_model(agent_id: &str, model_id: &str) -> Option<&'static str> {
    match (agent_id, model_id) {
        ("claude-code", "claude-opus-4-8") => Some("opus"),
        ("claude-code", "claude-sonnet-5" | "claude-sonnet-4-6") => Some("sonnet"),
        ("claude-code", "claude-haiku-4-5") => Some("haiku"),
        ("codex-cli", "gpt-5-5") => Some("gpt-5.5"),
        ("codex-cli", "gpt-5-4") => Some("gpt-5.4"),
        ("codex-cli", "gpt-5-2-codex") => Some("gpt-5.2-codex"),
        ("codex-cli", "gpt-5-1-codex-max") => Some("gpt-5.1-codex-max"),
        ("gemini-cli", "gemini-2-5-pro") => Some("gemini-2.5-pro"),
        ("gemini-cli", "gemini-2-5-flash") => Some("gemini-2.5-flash"),
        _ => None,
    }
}

fn effective_cli_selections(
    conn: &Connection,
    agent_id: &str,
    config: &ChatConfig,
) -> Result<BTreeMap<String, serde_json::Value>, AppError> {
    let mut selections = cli_parameters::load_selections(conn, agent_id)?;
    if let Some(model) = config
        .model_id
        .as_deref()
        .and_then(|model_id| mapped_chat_model(agent_id, model_id))
    {
        selections.insert(
            "model".to_string(),
            serde_json::Value::String(model.to_string()),
        );
    }
    if let Some(reasoning_depth) = config.reasoning_depth.as_deref() {
        match agent_id {
            "claude-code" => {
                selections.insert(
                    "effort".to_string(),
                    serde_json::Value::String(reasoning_depth.to_string()),
                );
            }
            "codex-cli" => {
                let effort = if reasoning_depth == "max" {
                    "xhigh"
                } else {
                    reasoning_depth
                };
                selections.insert(
                    "reasoningEffort".to_string(),
                    serde_json::Value::String(effort.to_string()),
                );
            }
            _ => {}
        }
    }
    match (agent_id, config.permission_mode.as_str()) {
        ("claude-code", "plan") => {
            selections.insert("permissionMode".to_string(), serde_json::json!("plan"));
        }
        ("claude-code", "agent" | "auto") => {
            selections.insert(
                "permissionMode".to_string(),
                serde_json::json!("acceptEdits"),
            );
        }
        ("codex-cli", "plan") => {
            selections.insert("sandbox".to_string(), serde_json::json!("read-only"));
            selections.insert(
                "approvalPolicy".to_string(),
                serde_json::json!("on-request"),
            );
        }
        ("codex-cli", "agent" | "auto") => {
            selections.insert("sandbox".to_string(), serde_json::json!("workspace-write"));
            selections.insert(
                "approvalPolicy".to_string(),
                serde_json::json!("on-request"),
            );
        }
        ("gemini-cli", "plan") => {
            selections.insert("approvalMode".to_string(), serde_json::json!("plan"));
        }
        ("gemini-cli", "agent" | "auto") => {
            selections.insert("approvalMode".to_string(), serde_json::json!("auto_edit"));
        }
        ("opencode", "plan") => {
            selections.insert("agent".to_string(), serde_json::json!("plan"));
        }
        ("opencode", "agent") => {
            selections.insert("agent".to_string(), serde_json::json!("build"));
        }
        ("opencode", "auto") => {
            selections.insert("autoApprove".to_string(), serde_json::json!(true));
        }
        _ => {}
    }
    if agent_id == "opencode" {
        selections.insert("thinking".to_string(), serde_json::json!(config.thinking));
    }
    cli_parameters::normalize_selections(agent_id, &selections)
}

struct GenericLineParser;

impl AgentOutputParser for GenericLineParser {
    fn parse_line(&self, line: &str) -> ParsedAgentEvent {
        if line.trim().is_empty() {
            ParsedAgentEvent::Empty
        } else {
            ParsedAgentEvent::Token(line.to_string())
        }
    }
}

struct ClaudeCodeParser;

impl AgentOutputParser for ClaudeCodeParser {
    fn parse_line(&self, line: &str) -> ParsedAgentEvent {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return ParsedAgentEvent::Empty;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            return ParsedAgentEvent::Token(line.to_string());
        };
        let event_type = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();

        match event_type {
            "system" | "session_init" => value
                .get("session_id")
                .or_else(|| value.get("sessionId"))
                .and_then(serde_json::Value::as_str)
                .map(|session_id| ParsedAgentEvent::SessionId(session_id.to_string()))
                .unwrap_or(ParsedAgentEvent::Empty),
            "assistant" | "assistant_delta" | "content_block_delta" => {
                let text = value
                    .pointer("/message/content/0/text")
                    .or_else(|| value.pointer("/delta/text"))
                    .or_else(|| value.get("text"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                if text.is_empty() {
                    ParsedAgentEvent::Empty
                } else {
                    ParsedAgentEvent::Token(text.to_string())
                }
            }
            "thinking" | "thinking_delta" => {
                let text = value
                    .pointer("/delta/thinking")
                    .or_else(|| value.get("thinking"))
                    .or_else(|| value.get("text"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                if text.is_empty() {
                    ParsedAgentEvent::Empty
                } else {
                    ParsedAgentEvent::Thinking(text.to_string())
                }
            }
            "tool_use" => {
                let tool = ToolUseBlock {
                    id: value
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("tool")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("tool")
                        .to_string(),
                    input: value.get("input").cloned(),
                    output: value.get("output").cloned(),
                    status: value
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("running")
                        .to_string(),
                };
                ParsedAgentEvent::ToolUse(tool)
            }
            "result" | "complete" | "completed" => ParsedAgentEvent::Completed,
            "error" | "failed" => {
                let message = value
                    .get("message")
                    .or_else(|| value.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Agent output reported an error.");
                ParsedAgentEvent::Failed(message.to_string())
            }
            _ => GenericLineParser.parse_line(line),
        }
    }
}

struct StructuredJsonParser;

impl StructuredJsonParser {
    fn text_value(value: &serde_json::Value) -> Option<String> {
        [
            "/delta/text",
            "/message/content/0/text",
            "/content/0/text",
            "/content/text",
            "/data/text",
        ]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
        .or_else(|| value.get("text").and_then(serde_json::Value::as_str))
        .or_else(|| value.get("content").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .filter(|text| !text.is_empty())
    }

    fn thinking_value(value: &serde_json::Value) -> Option<String> {
        [
            "/delta/thinking",
            "/thinking",
            "/reasoning",
            "/data/thinking",
        ]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .filter(|text| !text.is_empty())
    }

    fn session_id(value: &serde_json::Value) -> Option<String> {
        [
            "/session_id",
            "/sessionId",
            "/session/id",
            "/metadata/session_id",
            "/metadata/sessionId",
        ]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .filter(|text| !text.is_empty())
    }

    fn error_value(value: &serde_json::Value) -> Option<String> {
        value
            .get("message")
            .or_else(|| value.get("error"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                value
                    .pointer("/error/message")
                    .and_then(serde_json::Value::as_str)
            })
            .map(str::to_string)
            .filter(|text| !text.is_empty())
    }
}

impl AgentOutputParser for StructuredJsonParser {
    fn parse_line(&self, line: &str) -> ParsedAgentEvent {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return ParsedAgentEvent::Empty;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            return ParsedAgentEvent::Token(line.to_string());
        };
        let event_type = value
            .get("type")
            .or_else(|| value.get("event"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();

        if matches!(
            event_type,
            "error" | "failed" | "failure" | "turn.failed" | "run_error"
        ) {
            return ParsedAgentEvent::Failed(
                Self::error_value(&value)
                    .unwrap_or_else(|| "Agent CLI reported an error.".to_string()),
            );
        }
        if matches!(
            event_type,
            "result" | "done" | "complete" | "completed" | "turn.completed"
        ) {
            return ParsedAgentEvent::Completed;
        }
        if let Some(session_id) = Self::session_id(&value) {
            if matches!(
                event_type,
                "session" | "session_init" | "session_configured" | "start" | "started"
            ) {
                return ParsedAgentEvent::SessionId(session_id);
            }
        }
        if matches!(event_type, "thinking" | "thinking_delta" | "reasoning") {
            return Self::thinking_value(&value)
                .map(ParsedAgentEvent::Thinking)
                .unwrap_or(ParsedAgentEvent::Empty);
        }
        if matches!(event_type, "tool_use" | "tool" | "tool_call" | "tool.start") {
            let tool = ToolUseBlock {
                id: value
                    .get("id")
                    .or_else(|| value.pointer("/tool/id"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("tool")
                    .to_string(),
                name: value
                    .get("name")
                    .or_else(|| value.pointer("/tool/name"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("tool")
                    .to_string(),
                input: value
                    .get("input")
                    .or_else(|| value.pointer("/tool/input"))
                    .cloned(),
                output: value
                    .get("output")
                    .or_else(|| value.pointer("/tool/output"))
                    .cloned(),
                status: value
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("running")
                    .to_string(),
            };
            return ParsedAgentEvent::ToolUse(tool);
        }
        Self::text_value(&value)
            .map(ParsedAgentEvent::Token)
            .unwrap_or(ParsedAgentEvent::Empty)
    }
}

struct ActiveGeneration {
    message_id: String,
    process: Option<Child>,
}

#[derive(Debug, PartialEq)]
enum StopGenerationOutcome {
    NoActiveGeneration,
    SoftCancelled { message_id: String },
    ProcessKilled { message_id: String },
}

#[derive(Default)]
struct ChatRuntimeManager {
    active: Mutex<HashMap<String, ActiveGeneration>>,
}

impl ChatRuntimeManager {
    fn start(
        &self,
        session_id: String,
        message_id: String,
        process: Option<Child>,
    ) -> Result<(), AppError> {
        let mut active = self
            .active
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        active.insert(
            session_id,
            ActiveGeneration {
                message_id,
                process,
            },
        );
        Ok(())
    }

    fn complete(&self, session_id: &str) -> Result<(), AppError> {
        let mut active = self
            .active
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        active.remove(session_id);
        Ok(())
    }

    fn take_process(&self, session_id: &str) -> Result<Option<Child>, AppError> {
        let mut active = self
            .active
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        Ok(active
            .remove(session_id)
            .and_then(|mut generation| generation.process.take()))
    }

    fn stop(&self, session_id: &str) -> Result<StopGenerationOutcome, AppError> {
        let mut active = self
            .active
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let Some(mut generation) = active.remove(session_id) else {
            return Ok(StopGenerationOutcome::NoActiveGeneration);
        };
        if let Some(mut child) = generation.process.take() {
            child
                .kill()
                .map_err(|err| AppError::LaunchFailed(err.to_string()))?;
            let _ = child.wait();
            Ok(StopGenerationOutcome::ProcessKilled {
                message_id: generation.message_id,
            })
        } else {
            Ok(StopGenerationOutcome::SoftCancelled {
                message_id: generation.message_id,
            })
        }
    }
}

struct RegistryStore {
    db_path: PathBuf,
}

impl RegistryStore {
    fn new(data_dir: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&data_dir).map_err(|err| AppError::Storage(err.to_string()))?;
        Ok(Self {
            db_path: data_dir.join("vanehub.sqlite"),
        })
    }

    fn connection(&self) -> Result<Connection, AppError> {
        let conn = Connection::open(&self.db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&conn)?;
        seed_agents(&conn)?;
        skills::service::seed_builtin_skills(&conn)?;
        Ok(conn)
    }
}

fn migrate(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
        );
        "#,
    )?;

    apply_migration(conn, 1, "initial-schema", apply_initial_schema)?;
    apply_migration(
        conn,
        2,
        "agent-managed-sdk-dependency",
        apply_agent_sdk_dependency_migration,
    )?;
    apply_migration(
        conn,
        3,
        "session-management",
        apply_session_management_migration,
    )?;
    apply_migration(conn, 4, "chat-messages", apply_chat_messages_migration)?;
    apply_migration(conn, 5, "app-settings", apply_app_settings_migration)?;
    apply_migration(conn, 6, "cli-tool-status", apply_cli_tool_status_migration)?;
    apply_migration(conn, 7, "skill-management", skills::service::apply_schema)?;
    apply_migration(
        conn,
        8,
        "project-worktree-management",
        apply_project_worktree_migration,
    )?;
    apply_migration(
        conn,
        9,
        "session-runtime-metadata",
        apply_session_runtime_metadata_migration,
    )?;
    apply_migration(
        conn,
        10,
        "cli-parameter-settings",
        cli_parameters::apply_schema,
    )?;

    Ok(())
}

fn apply_project_worktree_migration(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS known_projects (
            path TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            is_git INTEGER NOT NULL DEFAULT 0,
            last_opened_at TEXT NOT NULL
        );
        "#,
    )?;
    for column in [
        "project_path",
        "worktree_path",
        "worktree_name",
        "worktree_branch",
    ] {
        if !table_has_column(conn, "sessions", column)? {
            conn.execute(
                &format!("ALTER TABLE sessions ADD COLUMN {column} TEXT"),
                [],
            )?;
        }
    }
    Ok(())
}

fn apply_session_runtime_metadata_migration(conn: &Connection) -> Result<(), AppError> {
    if !table_has_column(conn, "sessions", "runtime_session_id")? {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN runtime_session_id TEXT",
            [],
        )?;
    }
    Ok(())
}

fn apply_cli_tool_status_migration(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_tool_status (
            agent_id TEXT PRIMARY KEY,
            installed INTEGER,
            current_version TEXT,
            latest_version TEXT,
            available_versions TEXT NOT NULL DEFAULT '[]',
            detected_path TEXT,
            last_checked_at TEXT,
            last_error TEXT,
            last_operation_id TEXT,
            version_check_status TEXT NOT NULL DEFAULT 'not-detected'
        );
        "#,
    )?;
    Ok(())
}

fn apply_app_settings_migration(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn apply_chat_messages_migration(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'completed',
            content TEXT NOT NULL DEFAULT '',
            thinking_content TEXT,
            tool_use TEXT,
            token_input INTEGER DEFAULT 0,
            token_output INTEGER DEFAULT 0,
            metadata TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_created
            ON messages(session_id, created_at);
        "#,
    )?;
    Ok(())
}

fn apply_migration(
    conn: &Connection,
    version: i64,
    name: &str,
    migration: fn(&Connection) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let applied = conn
        .query_row(
            "SELECT 1 FROM schema_migrations WHERE version = ?1",
            params![version],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if applied {
        return Ok(());
    }

    if let Err(error) = migration(conn) {
        record_native_error("migration", &error);
        return Err(error);
    }
    conn.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        params![version, name],
    )?;
    Ok(())
}

fn apply_initial_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            provider TEXT NOT NULL,
            launch_kind TEXT NOT NULL,
            launch_command TEXT,
            launch_url TEXT,
            executable_name TEXT,
            managed_sdk_dependency_id TEXT
        );

        CREATE TABLE IF NOT EXISTS agent_modes (
            agent_id TEXT NOT NULL,
            mode TEXT NOT NULL,
            PRIMARY KEY (agent_id, mode),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS agent_capability_tags (
            agent_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (agent_id, tag),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS workflow_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            active_agent_id TEXT,
            active_interaction_mode TEXT,
            lifecycle_state TEXT NOT NULL,
            intent TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS session_details (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            adapter TEXT NOT NULL,
            message TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS mcp_servers (
            name TEXT PRIMARY KEY,
            transport_type TEXT NOT NULL DEFAULT 'stdio',
            command TEXT,
            args TEXT,
            env TEXT,
            url TEXT,
            headers TEXT,
            description TEXT,
            active INTEGER NOT NULL DEFAULT 1,
            scope TEXT NOT NULL DEFAULT 'user',
            project_path TEXT,
            last_connection_status TEXT,
            last_connected TEXT,
            last_error TEXT,
            last_tools TEXT,
            last_test_duration_ms INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO workflow_state (id, lifecycle_state, intent) VALUES (1, ?1, ?2)",
        params![
            SessionLifecycleState::Idle.as_str(),
            "Current development workflow"
        ],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO session_details (id, adapter, message) VALUES (1, ?1, ?2)",
        params!["none", "No active session."],
    )?;

    Ok(())
}

fn apply_agent_sdk_dependency_migration(conn: &Connection) -> Result<(), AppError> {
    if !table_has_column(conn, "agents", "managed_sdk_dependency_id")? {
        conn.execute(
            "ALTER TABLE agents ADD COLUMN managed_sdk_dependency_id TEXT",
            [],
        )?;
    }
    Ok(())
}

fn apply_session_management_migration(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            interaction_mode TEXT NOT NULL,
            lifecycle_state TEXT NOT NULL,
            folder TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            archived INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        );
        "#,
    )?;

    if !table_has_column(conn, "workflow_state", "active_session_id")? {
        conn.execute(
            "ALTER TABLE workflow_state ADD COLUMN active_session_id TEXT",
            [],
        )?;
    }

    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, AppError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn seed_agents(conn: &Connection) -> Result<(), AppError> {
    type SeedAgent = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
        Option<&'static str>,
        Option<&'static str>,
        Vec<&'static str>,
        Vec<&'static str>,
    );

    let agents: [SeedAgent; 4] = [
        (
            "claude-code",
            "Claude Code",
            "Anthropic",
            "cli",
            Some("claude"),
            None,
            Some("claude"),
            Some("claude-sdk"),
            vec!["cli", "native-desktop"],
            vec!["coding", "cli", "agent"],
        ),
        (
            "opencode",
            "OpenCode",
            "OpenCode",
            "cli",
            Some("opencode"),
            None,
            Some("opencode"),
            None,
            vec!["cli"],
            vec!["coding", "cli", "open-source"],
        ),
        (
            "codex-cli",
            "Codex CLI",
            "OpenAI",
            "cli",
            Some("codex"),
            None,
            Some("codex"),
            Some("codex-sdk"),
            vec!["cli", "native-desktop"],
            vec!["coding", "cli", "agent"],
        ),
        (
            "gemini-cli",
            "Gemini CLI",
            "Google",
            "cli",
            Some("gemini"),
            None,
            Some("gemini"),
            None,
            vec!["cli", "browser"],
            vec!["coding", "cli", "browser"],
        ),
    ];

    for (id, display_name, provider, kind, command, url, executable, sdk_dependency, modes, tags) in
        agents
    {
        conn.execute(
            "INSERT OR IGNORE INTO agents (id, display_name, provider, launch_kind, launch_command, launch_url, executable_name, managed_sdk_dependency_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, display_name, provider, kind, command, url, executable, sdk_dependency],
        )?;
        conn.execute(
            "UPDATE agents SET managed_sdk_dependency_id = ?1 WHERE id = ?2 AND managed_sdk_dependency_id IS NULL",
            params![sdk_dependency, id],
        )?;

        for mode in modes {
            conn.execute(
                "INSERT OR IGNORE INTO agent_modes (agent_id, mode) VALUES (?1, ?2)",
                params![id, mode],
            )?;
        }

        for tag in tags {
            conn.execute(
                "INSERT OR IGNORE INTO agent_capability_tags (agent_id, tag) VALUES (?1, ?2)",
                params![id, tag],
            )?;
        }
    }

    Ok(())
}

fn cli_tool_definition(agent_id: &str) -> Option<CliToolDefinition> {
    CLI_TOOL_DEFINITIONS
        .iter()
        .copied()
        .find(|definition| definition.agent_id == agent_id)
}

fn npm_executable() -> &'static str {
    if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    }
}

fn install_command_for(definition: CliToolDefinition) -> String {
    format!("npm install -g {}@latest", definition.package_name)
}

fn cli_package_install_args(definition: CliToolDefinition, target_version: &str) -> Vec<String> {
    vec![
        "install".to_string(),
        "-g".to_string(),
        format!("{}@{}", definition.package_name, target_version),
    ]
}

fn sanitized_cli_environment_context() -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("os".to_string(), std::env::consts::OS.to_string());
    context.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    context.insert(
        "pathConfigured".to_string(),
        std::env::var_os("PATH").is_some().to_string(),
    );
    context.insert(
        "npmConfigUserconfigConfigured".to_string(),
        std::env::var_os("NPM_CONFIG_USERCONFIG")
            .is_some()
            .to_string(),
    );
    context
}

struct CliPackageFailureDetails<'a> {
    target_version: &'a str,
    npm_executable: &'a str,
    args: &'a [String],
    stdout: Option<&'a str>,
    stderr: Option<&'a str>,
    exit_status: Option<&'a str>,
    timeout_reason: Option<&'a str>,
    error: &'a str,
}

fn cli_package_failure_context(
    definition: CliToolDefinition,
    operation_id: &str,
    details: CliPackageFailureDetails<'_>,
) -> BTreeMap<String, String> {
    let mut context = cli_definition_context(definition, operation_id);
    context.insert(
        "targetVersion".to_string(),
        details.target_version.to_string(),
    );
    context.insert(
        "npmExecutable".to_string(),
        details.npm_executable.to_string(),
    );
    context.insert("npmArguments".to_string(), details.args.join(" "));
    context.insert("error".to_string(), details.error.to_string());
    if let Some(stdout) = details.stdout {
        context.insert("stdout".to_string(), stdout.to_string());
    }
    if let Some(stderr) = details.stderr {
        context.insert("stderr".to_string(), stderr.to_string());
    }
    if let Some(exit_status) = details.exit_status {
        context.insert("exitStatus".to_string(), exit_status.to_string());
    }
    if let Some(timeout_reason) = details.timeout_reason {
        context.insert("timeoutReason".to_string(), timeout_reason.to_string());
    }
    for (key, value) in sanitized_cli_environment_context() {
        context.insert(key, value);
    }
    context
}

fn status_from_row(
    definition: CliToolDefinition,
    row: Option<(
        Option<i64>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )>,
) -> CliToolStatus {
    if let Some((
        installed,
        current_version,
        latest_version,
        available_versions,
        detected_path,
        last_checked_at,
        last_error,
        last_operation_id,
        version_check_status,
    )) = row
    {
        return CliToolStatus {
            agent_id: definition.agent_id.to_string(),
            display_name: definition.display_name.to_string(),
            provider: definition.provider.to_string(),
            executable_name: definition.executable_name.to_string(),
            package_name: definition.package_name.to_string(),
            installed: installed.map(|value| value != 0),
            current_version,
            latest_version,
            available_versions: serde_json::from_str::<Vec<String>>(&available_versions)
                .unwrap_or_default(),
            detected_path,
            install_command: install_command_for(definition),
            last_checked_at,
            last_error,
            last_operation_id,
            version_check_status: parse_cli_version_check_status(&version_check_status),
        };
    }

    CliToolStatus {
        agent_id: definition.agent_id.to_string(),
        display_name: definition.display_name.to_string(),
        provider: definition.provider.to_string(),
        executable_name: definition.executable_name.to_string(),
        package_name: definition.package_name.to_string(),
        installed: None,
        current_version: None,
        latest_version: None,
        available_versions: Vec::new(),
        detected_path: None,
        install_command: install_command_for(definition),
        last_checked_at: None,
        last_error: None,
        last_operation_id: None,
        version_check_status: CliVersionCheckStatus::NotDetected,
    }
}

fn parse_cli_version_check_status(value: &str) -> CliVersionCheckStatus {
    match value {
        "succeeded" => CliVersionCheckStatus::Succeeded,
        "failed" => CliVersionCheckStatus::Failed,
        "unsupported" => CliVersionCheckStatus::Unsupported,
        _ => CliVersionCheckStatus::NotDetected,
    }
}

fn cli_version_check_status_str(value: &CliVersionCheckStatus) -> &'static str {
    match value {
        CliVersionCheckStatus::Unsupported => "unsupported",
        CliVersionCheckStatus::NotDetected => "not-detected",
        CliVersionCheckStatus::Succeeded => "succeeded",
        CliVersionCheckStatus::Failed => "failed",
    }
}

fn load_cli_tool_statuses(conn: &Connection) -> Result<Vec<CliToolStatus>, AppError> {
    CLI_TOOL_DEFINITIONS
        .iter()
        .copied()
        .map(|definition| load_cli_tool_status(conn, definition))
        .collect()
}

fn should_start_initial_cli_refresh(conn: &Connection) -> Result<bool, AppError> {
    let count = conn.query_row("SELECT COUNT(*) FROM cli_tool_status", [], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(count == 0)
}

fn load_cli_tool_status(
    conn: &Connection,
    definition: CliToolDefinition,
) -> Result<CliToolStatus, AppError> {
    let row = conn
        .query_row(
            "SELECT installed, current_version, latest_version, available_versions, detected_path,
                    last_checked_at, last_error, last_operation_id, version_check_status
             FROM cli_tool_status WHERE agent_id = ?1",
            params![definition.agent_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?;
    Ok(status_from_row(definition, row))
}

fn save_cli_tool_status(conn: &Connection, status: &CliToolStatus) -> Result<(), AppError> {
    let available_versions = serde_json::to_string(&status.available_versions)
        .map_err(|error| AppError::Validation(error.to_string()))?;
    conn.execute(
        "INSERT INTO cli_tool_status (
            agent_id, installed, current_version, latest_version, available_versions, detected_path,
            last_checked_at, last_error, last_operation_id, version_check_status
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(agent_id) DO UPDATE SET
            installed = excluded.installed,
            current_version = excluded.current_version,
            latest_version = excluded.latest_version,
            available_versions = excluded.available_versions,
            detected_path = excluded.detected_path,
            last_checked_at = excluded.last_checked_at,
            last_error = excluded.last_error,
            last_operation_id = excluded.last_operation_id,
            version_check_status = excluded.version_check_status",
        params![
            status.agent_id,
            status.installed.map(|value| if value { 1 } else { 0 }),
            status.current_version,
            status.latest_version,
            available_versions,
            status.detected_path,
            status.last_checked_at,
            status.last_error,
            status.last_operation_id,
            cli_version_check_status_str(&status.version_check_status),
        ],
    )?;
    Ok(())
}

fn resolve_command_path(command_name: &str) -> Option<String> {
    let helper = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    let mut command = command_safety::std_command(helper).ok()?;
    command.arg(command_name);
    let output = command_output_with_timeout(&mut command, Duration::from_secs(2)).ok()?;
    if !output.success {
        return None;
    }
    output
        .stdout
        .lines()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn first_output_line(output: &CapturedCommandOutput) -> Option<String> {
    output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

fn detect_cli_tool(
    app: Option<&AppHandle>,
    registry: Option<&tasks::registry::TaskRegistry>,
    definition: CliToolDefinition,
    operation_id: &str,
) -> CliToolStatus {
    let now = current_timestamp();
    let detected_path = resolve_command_path(definition.executable_name);
    let mut current_version = None;
    let mut latest_version = None;
    let mut available_versions = Vec::new();
    let mut errors = Vec::new();

    if let Some(path) = detected_path.as_deref() {
        let mut command = match command_safety::std_command(path) {
            Ok(command) => command,
            Err(error) => {
                let reason = error.to_string();
                log_cli_detection_failure(
                    app,
                    registry,
                    operation_id,
                    definition,
                    "resolve-executable",
                    &reason,
                    None,
                );
                errors.push(reason);
                return CliToolStatus {
                    agent_id: definition.agent_id.to_string(),
                    display_name: definition.display_name.to_string(),
                    provider: definition.provider.to_string(),
                    executable_name: definition.executable_name.to_string(),
                    package_name: definition.package_name.to_string(),
                    installed: Some(true),
                    current_version,
                    latest_version,
                    available_versions,
                    detected_path: detected_path.clone(),
                    install_command: install_command_for(definition),
                    last_checked_at: Some(now),
                    last_error: Some(errors.join("; ")),
                    last_operation_id: Some(operation_id.to_string()),
                    version_check_status: CliVersionCheckStatus::Failed,
                };
            }
        };
        command.arg("--version");
        match command_output_with_timeout(&mut command, Duration::from_secs(3)) {
            Ok(output) if output.success => current_version = first_output_line(&output),
            Ok(output) => {
                let reason = first_output_line(&output)
                    .unwrap_or_else(|| format!("{} --version failed.", definition.executable_name));
                log_cli_detection_failure(
                    app,
                    registry,
                    operation_id,
                    definition,
                    "executable-version",
                    &reason,
                    Some(&output),
                );
                errors.push(reason);
            }
            Err(error) => {
                log_cli_detection_failure(
                    app,
                    registry,
                    operation_id,
                    definition,
                    "executable-version",
                    &error,
                    None,
                );
                errors.push(error);
            }
        }
    }

    match npm_view_package(definition.package_name, &["version"]) {
        Ok(version) => latest_version = Some(version),
        Err(error) => {
            let reason = error.to_string();
            log_cli_detection_failure(
                app,
                registry,
                operation_id,
                definition,
                "npm-view-version",
                &reason,
                None,
            );
            errors.push(reason);
        }
    }

    match npm_view_package(definition.package_name, &["versions", "--json"]) {
        Ok(raw) => available_versions = stable_versions_from_npm_json(&raw, 20),
        Err(error) => {
            let reason = error.to_string();
            log_cli_detection_failure(
                app,
                registry,
                operation_id,
                definition,
                "npm-view-versions",
                &reason,
                None,
            );
            errors.push(reason);
        }
    }

    let installed = detected_path.is_some();
    CliToolStatus {
        agent_id: definition.agent_id.to_string(),
        display_name: definition.display_name.to_string(),
        provider: definition.provider.to_string(),
        executable_name: definition.executable_name.to_string(),
        package_name: definition.package_name.to_string(),
        installed: Some(installed),
        current_version,
        latest_version,
        available_versions,
        detected_path,
        install_command: install_command_for(definition),
        last_checked_at: Some(now),
        last_error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
        last_operation_id: Some(operation_id.to_string()),
        version_check_status: if errors.is_empty() {
            CliVersionCheckStatus::Succeeded
        } else {
            CliVersionCheckStatus::Failed
        },
    }
}

fn npm_view_package(package_name: &str, view_args: &[&str]) -> Result<String, AppError> {
    let mut command = command_safety::std_command(npm_executable())?;
    let mut args = vec!["view", package_name];
    args.extend_from_slice(view_args);
    let audit_args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
    command_safety::audit_command("cli.npm.view", npm_executable(), &audit_args);
    command.args(args);
    let output = command_output_with_timeout(&mut command, Duration::from_secs(10))
        .map_err(AppError::LaunchFailed)?;
    if !output.success {
        return Err(AppError::Validation(
            first_output_line(&output).unwrap_or_else(|| "npm view failed".to_string()),
        ));
    }
    Ok(output.stdout.trim().to_string())
}

fn stable_versions_from_npm_json(raw: &str, limit: usize) -> Vec<String> {
    let Ok(versions) = serde_json::from_str::<Vec<String>>(raw) else {
        return Vec::new();
    };
    versions
        .into_iter()
        .filter(|version| is_stable_version(version))
        .rev()
        .take(limit)
        .collect()
}

fn cli_definition_context(
    definition: CliToolDefinition,
    operation_id: &str,
) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("operationId".to_string(), operation_id.to_string());
    context.insert("agentId".to_string(), definition.agent_id.to_string());
    context.insert(
        "displayName".to_string(),
        definition.display_name.to_string(),
    );
    context.insert("provider".to_string(), definition.provider.to_string());
    context.insert(
        "executableName".to_string(),
        definition.executable_name.to_string(),
    );
    context.insert(
        "packageName".to_string(),
        definition.package_name.to_string(),
    );
    context
}

fn write_cli_diagnostic_log_to_conn(
    conn: &Connection,
    level: logging::LogLevel,
    message: &str,
    context: BTreeMap<String, String>,
) -> Result<(), AppError> {
    logging::write_message(
        &active_log_dir_from_conn(conn)?,
        level,
        "cli.diagnostic",
        message,
        context,
    )
}

fn write_cli_diagnostic_log(
    app: &AppHandle,
    level: logging::LogLevel,
    message: &str,
    context: BTreeMap<String, String>,
) {
    let result = (|| -> Result<(), AppError> {
        let store = app.state::<Mutex<RegistryStore>>();
        let store = store
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let conn = store.connection()?;
        write_cli_diagnostic_log_to_conn(&conn, level, message, context)
    })();
    if let Err(error) = result {
        record_native_error("cli.diagnostic", &error);
    }
}

fn log_cli_detection_failure(
    app: Option<&AppHandle>,
    registry: Option<&tasks::registry::TaskRegistry>,
    operation_id: &str,
    definition: CliToolDefinition,
    attempted_operation: &str,
    reason: &str,
    output: Option<&CapturedCommandOutput>,
) {
    if let (Some(app), Some(registry)) = (app, registry) {
        append_cli_log(
            app,
            registry,
            operation_id,
            Some(definition.agent_id),
            &format!(
                "{} {} failed: {reason}",
                definition.display_name, attempted_operation
            ),
            logging::LogLevel::Warn,
        );
        let mut context = cli_definition_context(definition, operation_id);
        context.insert(
            "attemptedOperation".to_string(),
            attempted_operation.to_string(),
        );
        context.insert("reason".to_string(), reason.to_string());
        if let Some(output) = output {
            context.insert("stdout".to_string(), output.stdout.clone());
            context.insert("stderr".to_string(), output.stderr.clone());
            context.insert("exitStatus".to_string(), output.status.clone());
        }
        if reason.to_ascii_lowercase().contains("timed out") {
            context.insert("timeoutReason".to_string(), reason.to_string());
        }
        write_cli_diagnostic_log(
            app,
            logging::LogLevel::Warn,
            "CLI detection diagnostic failure.",
            context,
        );
    }
}

fn is_stable_version(version: &str) -> bool {
    !version.contains('-')
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
fn version_parts(version: &str) -> Option<Vec<u64>> {
    let trimmed = version.trim().trim_start_matches('v');
    if trimmed.contains('-') {
        return None;
    }
    let mut parts = Vec::new();
    for part in trimmed.split('.') {
        let numeric = part.parse::<u64>().ok()?;
        parts.push(numeric);
    }
    Some(parts)
}

#[cfg(test)]
fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let mut left_parts = version_parts(left)?;
    let mut right_parts = version_parts(right)?;
    let max_len = left_parts.len().max(right_parts.len());
    left_parts.resize(max_len, 0);
    right_parts.resize(max_len, 0);
    Some(left_parts.cmp(&right_parts))
}

#[derive(Debug)]
struct CapturedCommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
    status: String,
}

fn command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<CapturedCommandOutput, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| error.to_string())?;
                return Ok(CapturedCommandOutput {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                    status: output
                        .status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| output.status.to_string()),
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("command timed out".to_string());
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn command_exists(command_name: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg(command_name).output()
    } else {
        Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {command_name}"))
            .output()
    };

    output
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn parse_mode(value: &str) -> Result<InteractionMode, AppError> {
    match value {
        "browser" => Ok(InteractionMode::Browser),
        "native-desktop" => Ok(InteractionMode::NativeDesktop),
        "cli" => Ok(InteractionMode::Cli),
        other => Err(AppError::UnsupportedInteractionMode(other.to_string())),
    }
}

fn parse_lifecycle_state(value: &str) -> SessionLifecycleState {
    match value {
        "starting" => SessionLifecycleState::Starting,
        "running" => SessionLifecycleState::Running,
        "failed" => SessionLifecycleState::Failed,
        "stopped" => SessionLifecycleState::Stopped,
        _ => SessionLifecycleState::Idle,
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn fallback_log_dir() -> PathBuf {
    let root = std::env::var_os("VANEHUB_APP_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .map(|home| home.join(".vanehub"))
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    logging::default_log_dir(&root)
}

fn default_log_directory_for_conn(conn: &Connection) -> String {
    conn.path()
        .and_then(|path| PathBuf::from(path).parent().map(logging::default_log_dir))
        .unwrap_or_else(fallback_log_dir)
        .to_string_lossy()
        .to_string()
}

fn default_app_settings() -> AppSettings {
    AppSettings {
        application_language: "zh-CN".to_string(),
        font_size: "14px".to_string(),
        theme: "futuristic".to_string(),
        default_folder_path: String::new(),
        log_directory: fallback_log_dir().to_string_lossy().to_string(),
        network_proxy_url: String::new(),
        network_proxy_bypass: network_proxy::DEFAULT_BYPASS.to_string(),
        logging_policy: logging::policy(true),
    }
}

fn validate_setting_value(key: &str, value: &str) -> Result<(), AppError> {
    let valid = match key {
        "applicationLanguage" => matches!(value, "zh-CN" | "en"),
        "fontSize" => matches!(value, "12px" | "14px" | "16px" | "18px"),
        "theme" => matches!(value, "futuristic" | "minimal"),
        "defaultFolderPath" => true,
        "logDirectory" => !value.trim().is_empty(),
        "networkProxyUrl" => network_proxy::normalize_proxy_url(value).is_ok(),
        "networkProxyBypass" => network_proxy::normalize_bypass(value).is_ok(),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Invalid setting value for key '{key}'."
        )))
    }
}

fn normalize_setting_value(key: &str, value: &str) -> Result<String, AppError> {
    validate_setting_value(key, value)?;
    match key {
        "networkProxyUrl" => network_proxy::normalize_proxy_url(value),
        "networkProxyBypass" => network_proxy::normalize_bypass(value),
        _ => Ok(value.to_string()),
    }
}

fn load_setting_value(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

fn get_settings_from_conn(conn: &Connection) -> Result<AppSettings, AppError> {
    let mut defaults = default_app_settings();
    defaults.log_directory = default_log_directory_for_conn(conn);
    let application_language = load_setting_value(conn, "applicationLanguage")?
        .filter(|value| validate_setting_value("applicationLanguage", value).is_ok())
        .unwrap_or(defaults.application_language);
    let font_size = load_setting_value(conn, "fontSize")?
        .filter(|value| validate_setting_value("fontSize", value).is_ok())
        .unwrap_or(defaults.font_size);
    let theme = load_setting_value(conn, "theme")?
        .filter(|value| validate_setting_value("theme", value).is_ok())
        .unwrap_or(defaults.theme);
    let default_folder_path = load_setting_value(conn, "defaultFolderPath")?
        .filter(|value| validate_setting_value("defaultFolderPath", value).is_ok())
        .unwrap_or(defaults.default_folder_path);
    let log_directory = load_setting_value(conn, "logDirectory")?
        .filter(|value| validate_setting_value("logDirectory", value).is_ok())
        .unwrap_or(defaults.log_directory);
    let network_proxy_url = load_setting_value(conn, "networkProxyUrl")?
        .and_then(|value| network_proxy::normalize_proxy_url(&value).ok())
        .unwrap_or(defaults.network_proxy_url);
    let network_proxy_bypass = load_setting_value(conn, "networkProxyBypass")?
        .and_then(|value| network_proxy::normalize_bypass(&value).ok())
        .unwrap_or(defaults.network_proxy_bypass);
    network_proxy::apply(&network_proxy_url, &network_proxy_bypass)?;

    Ok(AppSettings {
        application_language,
        font_size,
        theme,
        default_folder_path,
        log_directory,
        network_proxy_url,
        network_proxy_bypass,
        logging_policy: logging::policy(true),
    })
}

fn save_setting_to_conn(
    conn: &Connection,
    key: &str,
    value: &str,
) -> Result<AppSettings, AppError> {
    let normalized_value = normalize_setting_value(key, value)?;
    if key == "logDirectory" {
        logging::validate_log_dir(&PathBuf::from(&normalized_value))?;
    }
    let now = current_timestamp();
    conn.execute(
        r#"
        INSERT INTO settings (key, value, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?3)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
        "#,
        params![key, normalized_value, now],
    )?;
    let settings = get_settings_from_conn(conn)?;
    network_proxy::apply(&settings.network_proxy_url, &settings.network_proxy_bypass)?;
    Ok(settings)
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_node_info() -> NodeInfo {
    let version = command_output("node", &["--version"]);
    let path = if cfg!(windows) {
        command_output("where", &["node"])
    } else {
        command_output("which", &["node"])
    }
    .and_then(|output| output.lines().next().map(str::trim).map(str::to_string))
    .filter(|value| !value.is_empty());

    match (path, version) {
        (Some(path), Some(version)) => NodeInfo {
            available: true,
            path: Some(path),
            version: Some(version),
            reason: None,
        },
        (path, version) => NodeInfo {
            available: false,
            path,
            version,
            reason: Some("Node.js executable or version could not be resolved.".to_string()),
        },
    }
}

fn load_session_from_row(row: &Row<'_>) -> Result<Session, rusqlite::Error> {
    let interaction_mode = row.get::<_, String>(3)?;
    let lifecycle_state = row.get::<_, String>(4)?;
    Ok(Session {
        id: row.get(0)?,
        title: row.get(1)?,
        agent_id: row.get(2)?,
        interaction_mode: parse_mode(&interaction_mode).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        lifecycle_state: parse_lifecycle_state(&lifecycle_state),
        folder: row.get(5)?,
        project_path: row.get(6)?,
        worktree_path: row.get(7)?,
        worktree_name: row.get(8)?,
        worktree_branch: row.get(9)?,
        runtime_session_id: row.get(10)?,
        pinned: row.get::<_, i64>(11)? != 0,
        archived: row.get::<_, i64>(12)? != 0,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn session_select_sql() -> &'static str {
    "SELECT id, title, agent_id, interaction_mode, lifecycle_state, folder, project_path, worktree_path, worktree_name, worktree_branch, runtime_session_id, pinned, archived, created_at, updated_at FROM sessions"
}

fn load_session(conn: &Connection, session_id: &str) -> Result<Session, AppError> {
    conn.query_row(
        &format!("{} WHERE id = ?1", session_select_sql()),
        params![session_id],
        load_session_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::SessionNotFound(session_id.to_string()))
}

fn display_name_for_path(path: &Path) -> String {
    if let Some(value) = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
    {
        value.to_string()
    } else {
        path.to_string_lossy().to_string()
    }
}

fn canonical_project_path(path: &str) -> Result<PathBuf, AppError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Project path is required.".to_string(),
        ));
    }
    let path = PathBuf::from(trimmed);
    std::fs::canonicalize(&path).map_err(|error| {
        record_native_log(
            NativeLogLevel::Error,
            "project.inspect",
            &format!("Project path unavailable: {trimmed}: {error}"),
        );
        AppError::Validation("Project unavailable".to_string())
    })
}

fn run_git_capture(args: &[&str]) -> Result<std::process::Output, AppError> {
    let audit_args = args
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    command_safety::audit_command("git.project", "git", &audit_args);
    command_safety::std_command("git")?
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            record_native_log(
                NativeLogLevel::Error,
                "git.project",
                &format!("Git command unavailable: {error}"),
            );
            AppError::Validation("Git unavailable".to_string())
        })
}

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

fn inspect_project_inner(path: &str) -> Result<ProjectInspection, AppError> {
    let canonical = canonical_project_path(path)?;
    let canonical_string = canonical.to_string_lossy().to_string();
    let output = run_git_capture(&["-C", &canonical_string, "rev-parse", "--show-toplevel"]);
    let git_root = match output {
        Ok(output) if output.status.success() => {
            let root = output_text(&output.stdout);
            if root.is_empty() {
                None
            } else {
                Some(root)
            }
        }
        Ok(output) => {
            record_native_log(
                NativeLogLevel::Info,
                "git.project",
                &format!(
                    "Git inspection reported non-repository. stdout={} stderr={}",
                    output_text(&output.stdout),
                    output_text(&output.stderr)
                ),
            );
            None
        }
        Err(AppError::Validation(message)) if message == "Git unavailable" => None,
        Err(error) => return Err(error),
    };

    Ok(ProjectInspection {
        path: canonical_string,
        display_name: display_name_for_path(&canonical),
        is_git: git_root.is_some(),
        git_root,
    })
}

fn upsert_known_project(conn: &Connection, inspection: &ProjectInspection) -> Result<(), AppError> {
    let now = current_timestamp();
    conn.execute(
        r#"
        INSERT INTO known_projects (path, display_name, is_git, last_opened_at)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(path) DO UPDATE SET
            display_name = excluded.display_name,
            is_git = excluded.is_git,
            last_opened_at = excluded.last_opened_at
        "#,
        params![
            inspection.path,
            inspection.display_name,
            if inspection.is_git { 1 } else { 0 },
            now
        ],
    )?;
    Ok(())
}

fn load_known_project_from_row(row: &Row<'_>) -> Result<KnownProject, rusqlite::Error> {
    Ok(KnownProject {
        path: row.get(0)?,
        display_name: row.get(1)?,
        is_git: row.get::<_, i64>(2)? != 0,
        last_opened_at: row.get(3)?,
    })
}

fn validate_worktree_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.chars().any(char::is_control)
    {
        return Err(AppError::Validation("Invalid worktree name".to_string()));
    }
    Ok(trimmed.to_string())
}

fn is_path_inside(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent) && child != parent
}

fn resolve_worktree_target(project_path: &Path, worktree_name: &str) -> Result<PathBuf, AppError> {
    let parent = project_path
        .parent()
        .ok_or_else(|| AppError::Validation("Project parent unavailable".to_string()))?;
    let project_name = project_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Validation("Project name unavailable".to_string()))?;
    let target = parent.join(format!("{project_name}-{worktree_name}"));
    if target.exists() {
        return Err(AppError::Validation(
            "Git worktree target exists".to_string(),
        ));
    }
    if is_path_inside(&target, project_path) {
        return Err(AppError::Validation("Invalid worktree target".to_string()));
    }
    Ok(target)
}

fn create_git_worktree(
    project_path: &Path,
    worktree_name: &str,
) -> Result<(String, String), AppError> {
    let safe_name = validate_worktree_name(worktree_name)?;
    let target = resolve_worktree_target(project_path, &safe_name)?;
    let project = project_path.to_string_lossy().to_string();
    let target_string = target.to_string_lossy().to_string();
    let branch = format!("vanehub/{safe_name}");
    let output = run_git_capture(&[
        "-C",
        &project,
        "worktree",
        "add",
        &target_string,
        "-b",
        &branch,
    ])?;
    if !output.status.success() {
        record_native_log(
            NativeLogLevel::Error,
            "git.worktree",
            &format!(
                "Git worktree failed. stdout={} stderr={}",
                output_text(&output.stdout),
                output_text(&output.stderr)
            ),
        );
        return Err(AppError::Validation("Git worktree failed".to_string()));
    }
    record_native_log(
        NativeLogLevel::Info,
        "git.worktree",
        &format!("Created worktree at {target_string} on {branch}"),
    );
    Ok((target_string, branch))
}

fn load_chat_message_from_row(row: &Row<'_>) -> Result<ChatMessage, rusqlite::Error> {
    let tool_use_json: Option<String> = row.get(6)?;
    let tool_use = tool_use_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Vec<ToolUseBlock>>(value).ok());
    let token_input = row.get::<_, Option<i64>>(7)?.unwrap_or(0);
    let token_output = row.get::<_, Option<i64>>(8)?.unwrap_or(0);
    let token_usage = if token_input > 0 || token_output > 0 {
        Some(TokenUsage {
            input: token_input,
            output: token_output,
        })
    } else {
        None
    };
    Ok(ChatMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        status: row.get(3)?,
        content: row.get(4)?,
        thinking_content: row.get(5)?,
        tool_use,
        token_usage,
        error: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn message_select_sql() -> &'static str {
    "SELECT id, session_id, role, status, content, thinking_content, tool_use, token_input, token_output, metadata, created_at, updated_at FROM messages"
}

fn insert_chat_message(
    conn: &Connection,
    session_id: &str,
    role: &str,
    status: &str,
    content: &str,
) -> Result<ChatMessage, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = current_timestamp();
    conn.execute(
        "INSERT INTO messages
         (id, session_id, role, status, content, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, session_id, role, status, content, now, now],
    )?;
    load_chat_message(conn, &id)
}

fn load_chat_message(conn: &Connection, message_id: &str) -> Result<ChatMessage, AppError> {
    conn.query_row(
        &format!("{} WHERE id = ?1", message_select_sql()),
        params![message_id],
        load_chat_message_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::Validation(format!("Message not found: {message_id}")))
}

fn list_chat_messages(
    conn: &Connection,
    session_id: &str,
    limit: Option<i64>,
    before_id: Option<&str>,
) -> Result<Vec<ChatMessage>, AppError> {
    load_session(conn, session_id)?;
    let page_size = limit.unwrap_or(50).clamp(1, 200);
    let mut messages = if let Some(before_id) = before_id {
        let mut stmt = conn.prepare(&format!(
            "{} WHERE session_id = ?1
             AND created_at < (SELECT created_at FROM messages WHERE id = ?2 AND session_id = ?1)
             ORDER BY created_at DESC LIMIT ?3",
            message_select_sql()
        ))?;
        let rows = stmt.query_map(
            params![session_id, before_id, page_size],
            load_chat_message_from_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(&format!(
            "{} WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            message_select_sql()
        ))?;
        let rows = stmt.query_map(params![session_id, page_size], load_chat_message_from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    messages.reverse();
    Ok(messages)
}

fn usage_range_start(range: UsageStatisticsRange) -> Option<String> {
    let now = Utc::now();
    let start = match range {
        UsageStatisticsRange::All => return None,
        UsageStatisticsRange::Today => now.date_naive().and_hms_opt(0, 0, 0)?,
        UsageStatisticsRange::Last7Days => (now - ChronoDuration::days(6))
            .date_naive()
            .and_hms_opt(0, 0, 0)?,
        UsageStatisticsRange::Last30Days => (now - ChronoDuration::days(29))
            .date_naive()
            .and_hms_opt(0, 0, 0)?,
    };
    Some(start.and_utc().to_rfc3339())
}

fn aggregate_usage_statistics(
    conn: &Connection,
    range: UsageStatisticsRange,
) -> Result<UsageStatistics, AppError> {
    let start = usage_range_start(range);
    let (input_tokens, output_tokens, counted_messages, counted_sessions): (i64, i64, i64, i64) =
        if let Some(start) = start {
            conn.query_row(
                "SELECT
                    COALESCE(SUM(token_input), 0),
                    COALESCE(SUM(token_output), 0),
                    COUNT(*),
                    COUNT(DISTINCT session_id)
                 FROM messages
                 WHERE role = 'assistant'
                   AND (COALESCE(token_input, 0) > 0 OR COALESCE(token_output, 0) > 0)
                   AND created_at >= ?1",
                params![start],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?
        } else {
            conn.query_row(
                "SELECT
                    COALESCE(SUM(token_input), 0),
                    COALESCE(SUM(token_output), 0),
                    COUNT(*),
                    COUNT(DISTINCT session_id)
                 FROM messages
                 WHERE role = 'assistant'
                   AND (COALESCE(token_input, 0) > 0 OR COALESCE(token_output, 0) > 0)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?
        };

    Ok(UsageStatistics {
        range,
        total_tokens: input_tokens + output_tokens,
        input_tokens,
        output_tokens,
        counted_messages,
        counted_sessions,
        generated_at: current_timestamp(),
    })
}

fn complete_assistant_message(
    conn: &Connection,
    message_id: &str,
    content: &str,
    token_usage: &TokenUsage,
) -> Result<ChatMessage, AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages
         SET status = 'completed', content = ?1, token_input = ?2, token_output = ?3, updated_at = ?4
         WHERE id = ?5",
        params![content, token_usage.input, token_usage.output, now, message_id],
    )?;
    load_chat_message(conn, message_id)
}

fn append_assistant_content(
    conn: &Connection,
    message_id: &str,
    content_delta: &str,
) -> Result<ChatMessage, AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages
         SET content = content || ?1, updated_at = ?2
         WHERE id = ?3",
        params![content_delta, now, message_id],
    )?;
    load_chat_message(conn, message_id)
}

fn append_assistant_thinking(
    conn: &Connection,
    message_id: &str,
    content_delta: &str,
) -> Result<ChatMessage, AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages
         SET thinking_content = COALESCE(thinking_content, '') || ?1, updated_at = ?2
         WHERE id = ?3",
        params![content_delta, now, message_id],
    )?;
    load_chat_message(conn, message_id)
}

fn append_assistant_tool_use(
    conn: &Connection,
    message_id: &str,
    tool_use: ToolUseBlock,
) -> Result<ChatMessage, AppError> {
    let mut message = load_chat_message(conn, message_id)?;
    let mut tools = message.tool_use.take().unwrap_or_default();
    tools.push(tool_use);
    let tools_json =
        serde_json::to_string(&tools).map_err(|error| AppError::Storage(error.to_string()))?;
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages SET tool_use = ?1, updated_at = ?2 WHERE id = ?3",
        params![tools_json, now, message_id],
    )?;
    load_chat_message(conn, message_id)
}

fn fail_assistant_message(
    conn: &Connection,
    message_id: &str,
    content: &str,
    error: &str,
) -> Result<ChatMessage, AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages
         SET status = 'failed', content = ?1, metadata = ?2, updated_at = ?3
         WHERE id = ?4",
        params![content, error, now, message_id],
    )?;
    load_chat_message(conn, message_id)
}

fn update_session_runtime_session_id(
    conn: &Connection,
    session_id: &str,
    runtime_session_id: &str,
) -> Result<Session, AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE sessions SET runtime_session_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![runtime_session_id, now, session_id],
    )?;
    load_session(conn, session_id)
}

fn cancel_streaming_messages(conn: &Connection, session_id: &str) -> Result<(), AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages SET status = 'cancelled', updated_at = ?1 WHERE session_id = ?2 AND status = 'streaming'",
        params![now, session_id],
    )?;
    Ok(())
}

fn update_session_lifecycle(
    conn: &Connection,
    session_id: &str,
    lifecycle: SessionLifecycleState,
) -> Result<Session, AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE sessions SET lifecycle_state = ?1, updated_at = ?2 WHERE id = ?3",
        params![lifecycle.as_str(), now, session_id],
    )?;
    conn.execute(
        "UPDATE workflow_state SET lifecycle_state = ?1 WHERE active_session_id = ?2",
        params![lifecycle.as_str(), session_id],
    )?;
    load_session(conn, session_id)
}

fn update_active_workflow_for_session(
    conn: &Connection,
    session: &Session,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE workflow_state
         SET active_session_id = ?1,
             active_agent_id = ?2,
             active_interaction_mode = ?3,
             lifecycle_state = ?4
         WHERE id = 1",
        params![
            session.id,
            session.agent_id,
            session.interaction_mode.as_str(),
            session.lifecycle_state.as_str()
        ],
    )?;
    Ok(())
}

fn clear_active_session_if_matches(conn: &Connection, session_id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE workflow_state SET active_session_id = NULL WHERE id = 1 AND active_session_id = ?1",
        params![session_id],
    )?;
    Ok(())
}

fn update_session_flag(
    conn: &Connection,
    session_id: &str,
    column: &str,
    value: bool,
) -> Result<Session, AppError> {
    let now = current_timestamp();
    conn.execute(
        &format!("UPDATE sessions SET {column} = ?1, updated_at = ?2 WHERE id = ?3"),
        params![if value { 1 } else { 0 }, now, session_id],
    )?;
    load_session(conn, session_id)
}

fn load_agent(conn: &Connection, agent_id: &str) -> Result<AgentRegistryEntry, AppError> {
    let row = conn
        .query_row(
            "SELECT id, display_name, provider, launch_kind, launch_command, launch_url, executable_name, managed_sdk_dependency_id
             FROM agents WHERE id = ?1",
            params![agent_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::AgentNotFound(agent_id.to_string()))?;

    let modes = load_modes(conn, &row.0)?;
    let tags = load_tags(conn, &row.0)?;
    let (availability_state, unavailable_reason) =
        availability_for(row.6.as_deref(), row.7.as_deref());

    Ok(AgentRegistryEntry {
        id: row.0,
        display_name: row.1,
        provider: row.2,
        managed_sdk_dependency_id: row.7,
        launch: LaunchMetadata {
            kind: row.3,
            command: row.4,
            url: row.5,
            executable_name: row.6,
        },
        supported_interaction_modes: modes,
        availability_state,
        unavailable_reason,
        capability_tags: tags,
    })
}

fn load_modes(conn: &Connection, agent_id: &str) -> Result<Vec<InteractionMode>, AppError> {
    let mut stmt =
        conn.prepare("SELECT mode FROM agent_modes WHERE agent_id = ?1 ORDER BY mode")?;
    let rows = stmt.query_map(params![agent_id], |row| row.get::<_, String>(0))?;
    let mut modes = Vec::new();
    for row in rows {
        modes.push(parse_mode(&row?)?);
    }
    Ok(modes)
}

fn load_tags(conn: &Connection, agent_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt =
        conn.prepare("SELECT tag FROM agent_capability_tags WHERE agent_id = ?1 ORDER BY tag")?;
    let rows = stmt.query_map(params![agent_id], |row| row.get::<_, String>(0))?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}

fn availability_for(
    executable_name: Option<&str>,
    managed_sdk_dependency_id: Option<&str>,
) -> (AvailabilityState, Option<String>) {
    if let Some(sdk_id) = managed_sdk_dependency_id {
        let Some(parsed_sdk_id) = sdk::models::SdkId::parse(sdk_id) else {
            return (
                AvailabilityState::Unavailable,
                Some(format!(
                    "Managed SDK dependency '{sdk_id}' is not recognized."
                )),
            );
        };
        if !sdk::service::is_installed(parsed_sdk_id) {
            return (
                AvailabilityState::Unavailable,
                Some(format!(
                    "Managed SDK dependency '{sdk_id}' is not installed."
                )),
            );
        }
    }

    match executable_name {
        Some(name) if command_exists(name) => (AvailabilityState::Available, None),
        Some(name) => (
            AvailabilityState::Unavailable,
            Some(format!("Command '{name}' was not found on PATH.")),
        ),
        None => (AvailabilityState::Unknown, None),
    }
}

#[tauri::command]
fn list_cli_tools(state: State<'_, Mutex<RegistryStore>>) -> Result<Vec<CliToolStatus>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_cli_tool_statuses(&conn)
}

#[tauri::command]
fn refresh_cli_detections(
    app: AppHandle,
    registry: State<'_, tasks::registry::TaskRegistry>,
) -> Result<tasks::models::OperationTask, AppError> {
    start_cli_refresh_operation(app, &registry, "Refreshing CLI detections")
}

fn start_cli_refresh_operation(
    app: AppHandle,
    registry: &tasks::registry::TaskRegistry,
    message: &str,
) -> Result<tasks::models::OperationTask, AppError> {
    let operation = registry.start(
        tasks::models::OperationKind::Agent,
        None,
        Some(message.to_string()),
    )?;
    let operation_id = operation.id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_cli_refresh_operation(app, operation_id);
    });

    Ok(operation)
}

#[tauri::command]
fn install_cli_version(
    app: AppHandle,
    registry: State<'_, tasks::registry::TaskRegistry>,
    agent_id: String,
    target_version: String,
) -> Result<tasks::models::OperationTask, AppError> {
    let definition = cli_tool_definition(&agent_id)
        .ok_or_else(|| AppError::Validation(format!("unsupported CLI agent id: {agent_id}")))?;
    if !is_stable_version(&target_version) {
        return Err(AppError::Validation(format!(
            "target version must be a stable semantic version: {target_version}"
        )));
    }
    let operation = registry.start(
        tasks::models::OperationKind::Agent,
        Some(agent_id.clone()),
        Some(format!(
            "Installing {} version {}",
            definition.display_name, target_version
        )),
    )?;
    let operation_id = operation.id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_cli_package_operation(app, operation_id, definition, target_version);
    });

    Ok(operation)
}

fn run_cli_refresh_operation(app: AppHandle, operation_id: String) {
    let registry = app.state::<tasks::registry::TaskRegistry>();
    append_cli_log(
        &app,
        &registry,
        &operation_id,
        None,
        "Starting CLI detection refresh.",
        logging::LogLevel::Info,
    );
    let mut statuses = Vec::new();
    for definition in CLI_TOOL_DEFINITIONS {
        append_cli_log(
            &app,
            &registry,
            &operation_id,
            Some(definition.agent_id),
            &format!(
                "Checking {} ({})",
                definition.display_name, definition.executable_name
            ),
            logging::LogLevel::Info,
        );
        let status = detect_cli_tool(Some(&app), Some(&registry), definition, &operation_id);
        if let Some(error) = status.last_error.as_deref() {
            append_cli_log(
                &app,
                &registry,
                &operation_id,
                Some(definition.agent_id),
                &format!(
                    "{} completed with warnings: {error}",
                    definition.display_name
                ),
                logging::LogLevel::Warn,
            );
        } else {
            append_cli_log(
                &app,
                &registry,
                &operation_id,
                Some(definition.agent_id),
                &format!("{} detection succeeded.", definition.display_name),
                logging::LogLevel::Info,
            );
        }
        statuses.push(status);
    }

    let persist_result = (|| -> Result<(), AppError> {
        let store = app.state::<Mutex<RegistryStore>>();
        let store = store
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let conn = store.connection()?;
        for status in &statuses {
            save_cli_tool_status(&conn, status)?;
        }
        Ok(())
    })();

    match persist_result {
        Ok(()) => {
            append_cli_log(
                &app,
                &registry,
                &operation_id,
                None,
                "CLI detection refresh finished.",
                logging::LogLevel::Info,
            );
            let result = serde_json::json!({
                "agentIds": statuses.iter().map(|status| status.agent_id.clone()).collect::<Vec<_>>()
            });
            let _ = registry.complete(&operation_id, Some(result));
        }
        Err(error) => {
            append_cli_log(
                &app,
                &registry,
                &operation_id,
                None,
                &format!("Failed to persist CLI detection results: {error}"),
                logging::LogLevel::Error,
            );
            let _ = registry.fail(&operation_id, error.to_string());
        }
    }
}

fn run_cli_package_operation(
    app: AppHandle,
    operation_id: String,
    definition: CliToolDefinition,
    target_version: String,
) {
    let registry = app.state::<tasks::registry::TaskRegistry>();
    let npm = npm_executable();
    let args = cli_package_install_args(definition, &target_version);
    append_cli_log(
        &app,
        &registry,
        &operation_id,
        Some(definition.agent_id),
        &format!(
            "Running npm install for {} version {}.",
            definition.display_name, target_version
        ),
        logging::LogLevel::Info,
    );
    append_cli_log(
        &app,
        &registry,
        &operation_id,
        Some(definition.agent_id),
        &format!("npm executable: {npm}; args: {}", args.join(" ")),
        logging::LogLevel::Info,
    );

    let mut command = match command_safety::std_command(npm) {
        Ok(command) => command,
        Err(error) => {
            let error = error.to_string();
            let context = cli_package_failure_context(
                definition,
                &operation_id,
                CliPackageFailureDetails {
                    target_version: &target_version,
                    npm_executable: npm,
                    args: &args,
                    stdout: None,
                    stderr: None,
                    exit_status: None,
                    timeout_reason: None,
                    error: &error,
                },
            );
            write_cli_diagnostic_log(
                &app,
                logging::LogLevel::Error,
                "CLI package operation failed before npm launch.",
                context,
            );
            persist_cli_operation_error(&app, definition, &operation_id, &error);
            let _ = registry.fail(&operation_id, error);
            return;
        }
    };
    command_safety::audit_command("cli.npm.install", npm, &args);
    command.args(&args);

    match command_output_with_timeout(&mut command, Duration::from_secs(300)) {
        Ok(output) if output.success => {
            append_command_logs(
                &app,
                &registry,
                &operation_id,
                Some(definition.agent_id),
                &output,
            );
            append_cli_log(
                &app,
                &registry,
                &operation_id,
                Some(definition.agent_id),
                &format!("npm install completed for {}.", definition.display_name),
                logging::LogLevel::Info,
            );
            let status = detect_cli_tool(Some(&app), Some(&registry), definition, &operation_id);
            let persist_result = (|| -> Result<(), AppError> {
                let store = app.state::<Mutex<RegistryStore>>();
                let store = store
                    .lock()
                    .map_err(|err| AppError::Storage(err.to_string()))?;
                let conn = store.connection()?;
                save_cli_tool_status(&conn, &status)
            })();
            match persist_result {
                Ok(()) => {
                    let result = serde_json::json!({
                        "agentId": definition.agent_id,
                        "targetVersion": target_version,
                    });
                    let _ = registry.complete(&operation_id, Some(result));
                }
                Err(error) => {
                    append_cli_log(
                        &app,
                        &registry,
                        &operation_id,
                        Some(definition.agent_id),
                        &format!("Failed to persist CLI package result: {error}"),
                        logging::LogLevel::Error,
                    );
                    let _ = registry.fail(&operation_id, error.to_string());
                }
            }
        }
        Ok(output) => {
            append_command_logs(
                &app,
                &registry,
                &operation_id,
                Some(definition.agent_id),
                &output,
            );
            let error =
                first_output_line(&output).unwrap_or_else(|| "npm install failed".to_string());
            let context = cli_package_failure_context(
                definition,
                &operation_id,
                CliPackageFailureDetails {
                    target_version: &target_version,
                    npm_executable: npm,
                    args: &args,
                    stdout: Some(&output.stdout),
                    stderr: Some(&output.stderr),
                    exit_status: Some(&output.status),
                    timeout_reason: None,
                    error: &error,
                },
            );
            write_cli_diagnostic_log(
                &app,
                logging::LogLevel::Error,
                "CLI package operation failed.",
                context,
            );
            persist_cli_operation_error(&app, definition, &operation_id, &error);
            let _ = registry.fail(&operation_id, error);
        }
        Err(error) => {
            let timeout_reason = error
                .to_ascii_lowercase()
                .contains("timed out")
                .then_some(error.as_str());
            let context = cli_package_failure_context(
                definition,
                &operation_id,
                CliPackageFailureDetails {
                    target_version: &target_version,
                    npm_executable: npm,
                    args: &args,
                    stdout: None,
                    stderr: None,
                    exit_status: None,
                    timeout_reason,
                    error: &error,
                },
            );
            write_cli_diagnostic_log(
                &app,
                logging::LogLevel::Error,
                "CLI package operation failed.",
                context,
            );
            persist_cli_operation_error(&app, definition, &operation_id, &error);
            let _ = registry.fail(&operation_id, error);
        }
    }
}

fn append_command_logs(
    app: &AppHandle,
    registry: &tasks::registry::TaskRegistry,
    operation_id: &str,
    agent_id: Option<&str>,
    output: &CapturedCommandOutput,
) {
    for line in output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        append_cli_log(
            app,
            registry,
            operation_id,
            agent_id,
            line,
            logging::LogLevel::Info,
        );
    }
    for line in output
        .stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        append_cli_log(
            app,
            registry,
            operation_id,
            agent_id,
            line,
            logging::LogLevel::Warn,
        );
    }
}

fn append_cli_log(
    app: &AppHandle,
    registry: &tasks::registry::TaskRegistry,
    operation_id: &str,
    agent_id: Option<&str>,
    line: &str,
    level: logging::LogLevel,
) {
    let _ = registry.append_log(operation_id, line.to_string());
    let result = (|| -> Result<(), AppError> {
        let store = app.state::<Mutex<RegistryStore>>();
        let store = store
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let conn = store.connection()?;
        let mut context = BTreeMap::new();
        context.insert("operationId".to_string(), operation_id.to_string());
        if let Some(agent_id) = agent_id {
            context.insert("agentId".to_string(), agent_id.to_string());
        }
        logging::write_message(
            &active_log_dir_from_conn(&conn)?,
            level,
            "cli.operation",
            line,
            context,
        )
    })();
    if let Err(error) = result {
        record_native_error("cli.log", &error);
    }
}

fn persist_cli_operation_error(
    app: &AppHandle,
    definition: CliToolDefinition,
    operation_id: &str,
    error: &str,
) {
    let result = (|| -> Result<(), AppError> {
        let store = app.state::<Mutex<RegistryStore>>();
        let store = store
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let conn = store.connection()?;
        let mut status = load_cli_tool_status(&conn, definition)?;
        status.last_operation_id = Some(operation_id.to_string());
        status.last_error = Some(error.to_string());
        status.version_check_status = CliVersionCheckStatus::Failed;
        save_cli_tool_status(&conn, &status)
    })();
    if let Err(error) = result {
        record_native_error("cli.persist", &error);
    }
}

fn native_desktop_supported() -> bool {
    cfg!(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    ))
}

fn set_lifecycle(conn: &Connection, lifecycle: SessionLifecycleState) -> Result<(), AppError> {
    conn.execute(
        "UPDATE workflow_state SET lifecycle_state = ?1 WHERE id = 1",
        params![lifecycle.as_str()],
    )?;
    Ok(())
}

fn set_session_message(conn: &Connection, adapter: &str, message: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE session_details SET adapter = ?1, message = ?2 WHERE id = 1",
        params![adapter, message],
    )?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, Mutex<RegistryStore>>) -> Result<AppSettings, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    get_settings_from_conn(&conn)
}

#[tauri::command]
fn save_setting(
    state: State<'_, Mutex<RegistryStore>>,
    input: SaveSettingInput,
) -> Result<AppSettings, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    save_setting_to_conn(&conn, &input.key, &input.value)
}

#[tauri::command]
async fn test_network_proxy(
    input: network_proxy::TestNetworkProxyInput,
) -> Result<network_proxy::NetworkProxyTestResult, AppError> {
    network_proxy::test_proxy(input).await
}

#[tauri::command]
async fn scan_network_proxies() -> Vec<network_proxy::DetectedNetworkProxy> {
    network_proxy::scan_local().await
}

fn active_log_dir_from_conn(conn: &Connection) -> Result<PathBuf, AppError> {
    Ok(PathBuf::from(get_settings_from_conn(conn)?.log_directory))
}

fn write_session_runtime_log(
    conn: &Connection,
    level: logging::LogLevel,
    session_id: &str,
    agent_id: &str,
    message: &str,
) -> Result<(), AppError> {
    let mut context = BTreeMap::new();
    context.insert("sessionId".to_string(), session_id.to_string());
    context.insert("agentId".to_string(), agent_id.to_string());
    logging::write_message(
        &active_log_dir_from_conn(conn)?,
        level,
        "session.runtime",
        message,
        context,
    )
}

fn concise_cli_unavailable_error(agent: &AgentRegistryEntry) -> String {
    format!("{} CLI unavailable", agent.display_name)
}

fn concise_cli_failed_error(agent: &AgentRegistryEntry) -> String {
    format!("{} command failed", agent.display_name)
}

fn concise_cli_error(agent: &AgentRegistryEntry, detail: &str) -> String {
    if detail.contains("could not be resolved") || detail.contains("unsupported") {
        concise_cli_unavailable_error(agent)
    } else {
        concise_cli_failed_error(agent)
    }
}

fn resolve_agent_cli_executable(
    conn: &Connection,
    agent: &AgentRegistryEntry,
) -> Result<String, String> {
    let Some(definition) = cli_tool_definition(&agent.id) else {
        return Err(format!(
            "{} is not supported by the generic CLI adapter.",
            agent.display_name
        ));
    };
    let cached_status =
        load_cli_tool_status(conn, definition).map_err(|error| error.to_string())?;
    if let Some(path) = cached_status
        .detected_path
        .filter(|path| !path.trim().is_empty())
    {
        return Ok(path);
    }
    resolve_command_path(definition.executable_name).ok_or_else(|| {
        format!(
            "{} executable '{}' could not be resolved.",
            agent.display_name, definition.executable_name
        )
    })
}

#[cfg(test)]
fn execute_generic_cli_agent(
    conn: &Connection,
    session_id: &str,
    agent: &AgentRegistryEntry,
    prompt: &str,
) -> Result<CapturedCommandOutput, String> {
    if agent.launch.kind != "cli" {
        return Err(format!(
            "{} launch kind '{}' is unsupported for chat runtime.",
            agent.display_name, agent.launch.kind
        ));
    }
    let executable = resolve_agent_cli_executable(conn, agent)?;
    let mut command =
        command_safety::std_command(&executable).map_err(|error| error.to_string())?;
    command.arg(prompt);
    command_safety::audit_command(
        "session.runtime.cli",
        &executable,
        &["[prompt redacted]".to_string()],
    );
    let _ = write_session_runtime_log(
        conn,
        logging::LogLevel::Info,
        session_id,
        &agent.id,
        &format!("executing {}", agent.display_name),
    );
    command_output_with_timeout(&mut command, Duration::from_secs(60))
}

struct SpawnedCliGeneration {
    child: Child,
    stdout: ChildStdout,
    stderr: Option<ChildStderr>,
}

fn spawn_cli_generation(
    conn: &Connection,
    session: &Session,
    agent: &AgentRegistryEntry,
    prompt: &str,
    config: &ChatConfig,
) -> Result<SpawnedCliGeneration, String> {
    if agent.launch.kind != "cli" {
        return Err(format!(
            "{} launch kind '{}' is unsupported for chat runtime.",
            agent.display_name, agent.launch.kind
        ));
    }
    let executable = resolve_agent_cli_executable(conn, agent)?;
    let selections =
        effective_cli_selections(conn, &agent.id, config).map_err(|error| error.to_string())?;
    let managed_args = cli_parameters::preview_args(
        &agent.id,
        &selections,
        cli_parameters::CliParameterLaunchScope::Chat,
    )
    .map_err(|error| error.to_string())?;
    let spec = build_cli_invocation_spec(
        agent,
        executable,
        prompt,
        session.runtime_session_id.as_deref(),
        &managed_args,
    )?;
    let mut command =
        command_safety::std_command(&spec.executable).map_err(|error| error.to_string())?;
    command.args(&spec.args);
    if let Some(folder) = session
        .folder
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        command.current_dir(folder);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if matches!(spec.prompt_delivery, PromptDelivery::Stdin) {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    command_safety::audit_command(
        "session.runtime.cli",
        &spec.executable,
        &spec
            .args
            .iter()
            .map(|arg| {
                if arg == prompt {
                    "[prompt redacted]".to_string()
                } else {
                    arg.clone()
                }
            })
            .collect::<Vec<_>>(),
    );
    let _ = write_session_runtime_log(
        conn,
        logging::LogLevel::Info,
        &session.id,
        &agent.id,
        &format!(
            "executing {} through provider CLI runtime",
            agent.display_name
        ),
    );
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    if matches!(spec.prompt_delivery, PromptDelivery::Stdin) {
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(error) = stdin
                .write_all(prompt.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
            {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "CLI process stdout unavailable.".to_string())?;
    let stderr = child.stderr.take();
    Ok(SpawnedCliGeneration {
        child,
        stdout,
        stderr,
    })
}

fn read_stderr_to_string(stderr: Option<ChildStderr>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let Some(stderr) = stderr else {
            return String::new();
        };
        let mut content = String::new();
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(&line);
        }
        content
    })
}

fn run_cli_generation_stream(
    app: AppHandle,
    db_path: PathBuf,
    session_id: String,
    agent: AgentRegistryEntry,
    assistant_message_id: String,
    stdout: ChildStdout,
    stderr: Option<ChildStderr>,
    prompt_len: usize,
) {
    let stderr_handle = read_stderr_to_string(stderr);
    let parser = parser_for_agent(&agent.id);
    let store = RegistryStore { db_path };
    let conn = match store.connection() {
        Ok(conn) => conn,
        Err(error) => {
            record_native_error("session.runtime.cli", &error);
            return;
        }
    };
    let reader = BufReader::new(stdout);
    let mut response = String::new();
    let mut terminal_error: Option<String> = None;

    for line in reader.lines() {
        let Ok(line) = line else {
            terminal_error = Some("Failed to read Agent CLI output.".to_string());
            break;
        };
        match parser.parse_line(&line) {
            ParsedAgentEvent::Token(delta) => {
                let content_delta = if response.is_empty() {
                    delta
                } else {
                    format!("\n{delta}")
                };
                response.push_str(&content_delta);
                let _ = append_assistant_content(&conn, &assistant_message_id, &content_delta);
                let _ = app.emit(
                    "chat:event",
                    ChatStreamEvent::Token {
                        session_id: session_id.clone(),
                        message_id: assistant_message_id.clone(),
                        content_delta,
                    },
                );
            }
            ParsedAgentEvent::Thinking(content_delta) => {
                let _ = append_assistant_thinking(&conn, &assistant_message_id, &content_delta);
                let _ = app.emit(
                    "chat:event",
                    ChatStreamEvent::Thinking {
                        session_id: session_id.clone(),
                        message_id: assistant_message_id.clone(),
                        content_delta,
                    },
                );
            }
            ParsedAgentEvent::ToolUse(tool_use) => {
                let _ = append_assistant_tool_use(&conn, &assistant_message_id, tool_use.clone());
                let _ = app.emit(
                    "chat:event",
                    ChatStreamEvent::ToolUse {
                        session_id: session_id.clone(),
                        message_id: assistant_message_id.clone(),
                        tool_use,
                    },
                );
            }
            ParsedAgentEvent::SessionId(runtime_session_id) => {
                let _ = update_session_runtime_session_id(&conn, &session_id, &runtime_session_id);
            }
            ParsedAgentEvent::Failed(error) => {
                terminal_error = Some(error);
            }
            ParsedAgentEvent::Completed | ParsedAgentEvent::Empty => {}
        }
    }

    let runtime = app.state::<ChatRuntimeManager>();
    let exit_status = match runtime.take_process(&session_id) {
        Ok(Some(mut child)) => child.wait().ok(),
        Ok(None) => None,
        Err(error) => {
            record_native_error("session.runtime.cli", &error);
            None
        }
    };
    let stderr_output = stderr_handle.join().unwrap_or_default();
    if !stderr_output.trim().is_empty() {
        let _ = write_session_runtime_log(
            &conn,
            logging::LogLevel::Warn,
            &session_id,
            &agent.id,
            stderr_output.trim(),
        );
    }

    let current_message = match load_chat_message(&conn, &assistant_message_id) {
        Ok(message) => message,
        Err(error) => {
            record_native_error("session.runtime.cli", &error);
            return;
        }
    };
    if current_message.status == "cancelled" {
        return;
    }

    if let Some(error) = terminal_error {
        let concise_error = concise_cli_failed_error(&agent);
        let _ = write_session_runtime_log(
            &conn,
            logging::LogLevel::Error,
            &session_id,
            &agent.id,
            &error,
        );
        let _ = fail_assistant_message(&conn, &assistant_message_id, &response, &concise_error);
        let _ = update_session_lifecycle(&conn, &session_id, SessionLifecycleState::Failed);
        let _ = app.emit(
            "chat:event",
            ChatStreamEvent::Failed {
                session_id,
                message_id: assistant_message_id,
                error: concise_error,
            },
        );
        return;
    }

    if !exit_status.map(|status| status.success()).unwrap_or(false) {
        let concise_error = concise_cli_failed_error(&agent);
        let detail = if stderr_output.trim().is_empty() {
            concise_error.clone()
        } else {
            stderr_output.trim().to_string()
        };
        let _ = write_session_runtime_log(
            &conn,
            logging::LogLevel::Error,
            &session_id,
            &agent.id,
            &detail,
        );
        let _ = fail_assistant_message(&conn, &assistant_message_id, &response, &concise_error);
        let _ = update_session_lifecycle(&conn, &session_id, SessionLifecycleState::Failed);
        let _ = app.emit(
            "chat:event",
            ChatStreamEvent::Failed {
                session_id,
                message_id: assistant_message_id,
                error: concise_error,
            },
        );
        return;
    }

    let token_usage = TokenUsage {
        input: prompt_len as i64,
        output: response.chars().count() as i64,
    };
    let _ = complete_assistant_message(&conn, &assistant_message_id, &response, &token_usage);
    let _ = update_session_lifecycle(&conn, &session_id, SessionLifecycleState::Idle);
    let _ = app.emit(
        "chat:event",
        ChatStreamEvent::Completed {
            session_id,
            message_id: assistant_message_id,
            token_usage: Some(token_usage),
        },
    );
}

#[tauri::command]
fn open_log_directory(state: State<'_, Mutex<RegistryStore>>) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    logging::open_directory(&active_log_dir_from_conn(&conn)?)
}

#[tauri::command]
fn report_client_log_event(
    state: State<'_, Mutex<RegistryStore>>,
    event: logging::ClientLogEvent,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    logging::write_client_event(&active_log_dir_from_conn(&conn)?, event)
}

#[tauri::command]
fn get_node_info() -> NodeInfo {
    resolve_node_info()
}

#[tauri::command]
fn list_agents(
    state: State<'_, Mutex<RegistryStore>>,
    capability_tag: Option<String>,
) -> Result<Vec<AgentRegistryEntry>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let mut stmt = conn.prepare("SELECT id FROM agents ORDER BY display_name")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut agents = Vec::new();
    for row in rows {
        let agent = load_agent(&conn, &row?)?;
        if let Some(tag) = capability_tag.as_deref() {
            if !agent
                .capability_tags
                .iter()
                .any(|candidate| candidate == tag)
            {
                continue;
            }
        }
        agents.push(agent);
    }
    Ok(agents)
}

#[tauri::command]
fn get_agent_by_id(
    state: State<'_, Mutex<RegistryStore>>,
    agent_id: String,
) -> Result<AgentRegistryEntry, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_agent(&conn, &agent_id)
}

#[tauri::command]
fn get_workflow_state(state: State<'_, Mutex<RegistryStore>>) -> Result<WorkflowState, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    get_workflow_state_from_conn(&conn)
}

#[tauri::command]
fn select_agent(
    state: State<'_, Mutex<RegistryStore>>,
    agent_id: String,
    interaction_mode: InteractionMode,
) -> Result<WorkflowState, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let agent = load_agent(&conn, &agent_id)?;

    if matches!(
        agent.availability_state,
        AvailabilityState::Unavailable | AvailabilityState::NeedsAuth
    ) {
        return Err(AppError::AgentUnavailable(
            agent
                .unavailable_reason
                .unwrap_or_else(|| format!("{} is not available.", agent.display_name)),
        ));
    }

    if !agent
        .supported_interaction_modes
        .iter()
        .any(|mode| mode.as_str() == interaction_mode.as_str())
    {
        return Err(AppError::UnsupportedInteractionMode(
            interaction_mode.as_str().to_string(),
        ));
    }

    let current_intent = conn.query_row(
        "SELECT intent FROM workflow_state WHERE id = 1",
        [],
        |row| row.get::<_, String>(0),
    )?;

    conn.execute(
        "UPDATE workflow_state
         SET active_agent_id = ?1, active_interaction_mode = ?2, lifecycle_state = ?3
         WHERE id = 1",
        params![
            agent_id,
            interaction_mode.as_str(),
            SessionLifecycleState::Idle.as_str()
        ],
    )?;

    Ok(WorkflowState {
        active_agent_id: Some(agent.id),
        active_interaction_mode: Some(interaction_mode),
        lifecycle_state: SessionLifecycleState::Idle,
        intent: current_intent,
    })
}

#[tauri::command]
fn check_browser_readiness(
    state: State<'_, Mutex<RegistryStore>>,
    agent_id: String,
) -> Result<ReadinessStatus, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let agent = load_agent(&conn, &agent_id)?;
    let supports_browser = agent
        .supported_interaction_modes
        .iter()
        .any(|mode| matches!(mode, InteractionMode::Browser));

    if !supports_browser {
        return Ok(ReadinessStatus {
            ready: false,
            reason: Some(format!(
                "{} does not support browser interaction mode.",
                agent.display_name
            )),
            requires_authentication: false,
        });
    }

    Ok(ReadinessStatus {
        ready: true,
        reason: None,
        requires_authentication: true,
    })
}

#[tauri::command]
fn launch_active_workflow(
    state: State<'_, Mutex<RegistryStore>>,
    registry: State<'_, tasks::registry::TaskRegistry>,
) -> Result<LaunchResult, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let workflow = get_workflow_state_from_conn(&conn)?;
    let agent_id = workflow
        .active_agent_id
        .clone()
        .ok_or(AppError::NoActiveAgent)?;
    let mode = workflow
        .active_interaction_mode
        .clone()
        .ok_or(AppError::NoActiveAgent)?;
    let agent = load_agent(&conn, &agent_id)?;
    let task = registry.start(
        tasks::models::OperationKind::Agent,
        Some(agent_id.clone()),
        Some(format!("Launching {}", agent.display_name)),
    )?;

    set_lifecycle(&conn, SessionLifecycleState::Starting)?;

    let message = match mode {
        InteractionMode::Browser => {
            let readiness = check_browser_readiness_inner(&agent);
            if !readiness.ready {
                set_lifecycle(&conn, SessionLifecycleState::Failed)?;
                let error = AppError::LaunchFailed(
                    readiness
                        .reason
                        .unwrap_or_else(|| "Browser mode is not ready.".to_string()),
                );
                let _ = registry.fail(&task.id, error.to_string());
                return Err(error);
            }
            set_session_message(
                &conn,
                "browser",
                "Browser workflow routed to Playwright adapter.",
            )?;
            "Browser workflow routed to Playwright adapter.".to_string()
        }
        InteractionMode::NativeDesktop => {
            if !native_desktop_supported() {
                set_lifecycle(&conn, SessionLifecycleState::Failed)?;
                let error = AppError::UnsupportedInteractionMode(
                    "native-desktop is not supported on this platform".to_string(),
                );
                let _ = registry.fail(&task.id, error.to_string());
                return Err(error);
            }
            if let Err(error) = launch_command_if_present(&conn, &agent) {
                set_lifecycle(&conn, SessionLifecycleState::Failed)?;
                let _ = registry.fail(&task.id, error.to_string());
                return Err(error);
            }
            set_session_message(
                &conn,
                "native-desktop",
                "Native desktop workflow launch routed through Tauri adapter.",
            )?;
            "Native desktop workflow launch routed through Tauri adapter.".to_string()
        }
        InteractionMode::Cli => {
            if let Err(error) = launch_command_if_present(&conn, &agent) {
                set_lifecycle(&conn, SessionLifecycleState::Failed)?;
                let _ = registry.fail(&task.id, error.to_string());
                return Err(error);
            }
            set_session_message(
                &conn,
                "cli",
                "CLI workflow launch routed through Tauri adapter.",
            )?;
            "CLI workflow launch routed through Tauri adapter.".to_string()
        }
    };

    set_lifecycle(&conn, SessionLifecycleState::Running)?;
    let _ = registry.append_log(&task.id, message.clone());
    let mut context = BTreeMap::new();
    context.insert("operationId".to_string(), task.id.clone());
    context.insert("agentId".to_string(), agent_id);
    context.insert("interactionMode".to_string(), mode.as_str().to_string());
    let _ = logging::write_message(
        &active_log_dir_from_conn(&conn)?,
        logging::LogLevel::Info,
        "agent.launch",
        &message,
        context,
    );
    let _ = registry.complete(&task.id, None);

    Ok(LaunchResult {
        operation_id: Some(task.id),
        workflow: get_workflow_state_from_conn(&conn)?,
        message,
    })
}

fn check_browser_readiness_inner(agent: &AgentRegistryEntry) -> ReadinessStatus {
    let supports_browser = agent
        .supported_interaction_modes
        .iter()
        .any(|mode| matches!(mode, InteractionMode::Browser));

    ReadinessStatus {
        ready: supports_browser,
        reason: if supports_browser {
            None
        } else {
            Some(format!(
                "{} does not support browser interaction mode.",
                agent.display_name
            ))
        },
        requires_authentication: supports_browser,
    }
}

fn launch_command_if_present(
    conn: &Connection,
    agent: &AgentRegistryEntry,
) -> Result<(), AppError> {
    let Some(command) = agent.launch.command.as_deref() else {
        return Ok(());
    };

    if !command_exists(command) {
        return Err(AppError::LaunchFailed(format!(
            "Command '{command}' was not found on PATH."
        )));
    }

    let managed_args = if cli_parameters::MANAGED_CLI_AGENT_IDS.contains(&agent.id.as_str()) {
        let selections = cli_parameters::load_selections(conn, &agent.id)?;
        cli_parameters::preview_args(
            &agent.id,
            &selections,
            cli_parameters::CliParameterLaunchScope::Interactive,
        )?
    } else {
        Vec::new()
    };
    command_safety::audit_command("command.launch", command, &managed_args);
    let mut process = command_safety::std_command(command)?;
    process
        .args(&managed_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| {
            let error = AppError::LaunchFailed(err.to_string());
            record_native_error("command.launch", &error);
            error
        })
}

#[tauri::command]
fn list_known_projects(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<Vec<KnownProject>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let mut stmt = conn.prepare(
        "SELECT path, display_name, is_git, last_opened_at FROM known_projects ORDER BY last_opened_at DESC",
    )?;
    let rows = stmt.query_map([], load_known_project_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

#[tauri::command]
fn inspect_project(path: String) -> Result<ProjectInspection, AppError> {
    inspect_project_inner(&path)
}

#[tauri::command]
fn select_project_directory() -> Result<Option<String>, AppError> {
    let cwd = std::env::current_dir().map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(Some(cwd.to_string_lossy().to_string()))
}

#[tauri::command]
fn create_session(
    app: AppHandle,
    registry: State<'_, tasks::registry::TaskRegistry>,
    input: CreateSessionInput,
) -> Result<tasks::models::OperationTask, AppError> {
    let operation = registry.start(
        tasks::models::OperationKind::Workspace,
        input.project_path.clone().or(input.folder.clone()),
        Some("Creating session".to_string()),
    )?;
    let operation_id = operation.id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let registry = app.state::<tasks::registry::TaskRegistry>();
        let result = (|| -> Result<Session, AppError> {
            let state = app.state::<Mutex<RegistryStore>>();
            let store = state
                .lock()
                .map_err(|err| AppError::Storage(err.to_string()))?;
            let conn = store.connection()?;
            create_session_inner(&conn, input)
        })();

        match result {
            Ok(session) => {
                let _ =
                    registry.append_log(&operation_id, format!("Created session {}", session.id));
                let _ = registry.complete(&operation_id, serde_json::to_value(&session).ok());
            }
            Err(error) => {
                record_native_error("session.create", &error);
                let _ = registry.append_log(&operation_id, error.to_string());
                let _ = registry.fail(&operation_id, error.to_string());
            }
        }
    });

    Ok(operation)
}

fn create_session_inner(conn: &Connection, input: CreateSessionInput) -> Result<Session, AppError> {
    let agent = load_agent(conn, &input.agent_id)?;
    if !agent
        .supported_interaction_modes
        .iter()
        .any(|mode| mode.as_str() == input.interaction_mode.as_str())
    {
        return Err(AppError::UnsupportedInteractionMode(
            input.interaction_mode.as_str().to_string(),
        ));
    }

    let selected_project = input
        .project_path
        .as_deref()
        .or(input.folder.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let inspection = selected_project.map(inspect_project_inner).transpose()?;
    if let Some(inspection) = &inspection {
        upsert_known_project(conn, inspection)?;
    }

    let mut effective_folder = inspection
        .as_ref()
        .map(|project| project.path.clone())
        .or(input.folder.clone());
    let mut worktree_path = None;
    let mut worktree_name = None;
    let mut worktree_branch = None;
    if let Some(request) = &input.worktree {
        if request.enabled {
            let inspection = inspection
                .as_ref()
                .ok_or_else(|| AppError::Validation("Project unavailable".to_string()))?;
            if !inspection.is_git {
                return Err(AppError::Validation("Git worktree unavailable".to_string()));
            }
            let name = validate_worktree_name(request.name.as_deref().unwrap_or(""))?;
            let (created_path, branch) = create_git_worktree(Path::new(&inspection.path), &name)?;
            effective_folder = Some(created_path.clone());
            worktree_path = Some(created_path);
            worktree_name = Some(name);
            worktree_branch = Some(branch);
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = current_timestamp();
    let session_title = input
        .title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "新会话".to_string());
    conn.execute(
        "INSERT INTO sessions
         (id, title, agent_id, interaction_mode, lifecycle_state, folder, project_path, worktree_path, worktree_name, worktree_branch, pinned, archived, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, ?11, ?12)",
        params![
            id,
            session_title,
            input.agent_id,
            input.interaction_mode.as_str(),
            SessionLifecycleState::Idle.as_str(),
            effective_folder,
            inspection.as_ref().map(|project| project.path.clone()),
            worktree_path,
            worktree_name,
            worktree_branch,
            now,
            now
        ],
    )?;

    let session = load_session(conn, &id)?;
    update_active_workflow_for_session(conn, &session)?;
    Ok(session)
}

#[tauri::command]
fn list_sessions(state: State<'_, Mutex<RegistryStore>>) -> Result<Vec<Session>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let mut stmt = conn.prepare(&format!(
        "{} WHERE archived = 0 ORDER BY pinned DESC, updated_at DESC",
        session_select_sql()
    ))?;
    let rows = stmt.query_map([], load_session_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

#[tauri::command]
fn list_archived_sessions(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<Vec<Session>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let mut stmt = conn.prepare(&format!(
        "{} WHERE archived = 1 ORDER BY pinned DESC, updated_at DESC",
        session_select_sql()
    ))?;
    let rows = stmt.query_map([], load_session_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

#[tauri::command]
fn get_active_session(state: State<'_, Mutex<RegistryStore>>) -> Result<Option<Session>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let active_session_id = conn.query_row(
        "SELECT active_session_id FROM workflow_state WHERE id = 1",
        [],
        |row| row.get::<_, Option<String>>(0),
    )?;
    let Some(session_id) = active_session_id else {
        return Ok(None);
    };
    let session = conn
        .query_row(
            &format!("{} WHERE id = ?1 AND archived = 0", session_select_sql()),
            params![session_id],
            load_session_from_row,
        )
        .optional()?;
    if session.is_none() {
        clear_active_session_if_matches(&conn, &session_id)?;
    }
    Ok(session)
}

#[tauri::command]
fn switch_session(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let session = load_session(&conn, &session_id)?;
    if session.archived {
        return Err(AppError::Validation(format!(
            "Cannot switch to archived session: {session_id}"
        )));
    }
    update_active_workflow_for_session(&conn, &session)?;
    Ok(session)
}

#[tauri::command]
fn rename_session(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    title: String,
) -> Result<Session, AppError> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::Validation(
            "Session title cannot be empty.".to_string(),
        ));
    }
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let now = current_timestamp();
    conn.execute(
        "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now, session_id],
    )?;
    load_session(&conn, &session_id)
}

#[tauri::command]
fn pin_session(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    update_session_flag(&conn, &session_id, "pinned", true)
}

#[tauri::command]
fn unpin_session(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    update_session_flag(&conn, &session_id, "pinned", false)
}

#[tauri::command]
fn archive_session(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ChatRuntimeManager>,
    session_id: String,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    stop_generation_for_session(&app, &conn, &runtime, &session_id)?;
    let session = update_session_flag(&conn, &session_id, "archived", true)?;
    clear_active_session_if_matches(&conn, &session_id)?;
    Ok(session)
}

#[tauri::command]
fn unarchive_session(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    update_session_flag(&conn, &session_id, "archived", false)
}

#[tauri::command]
fn delete_session(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ChatRuntimeManager>,
    session_id: String,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_session(&conn, &session_id)?;
    stop_generation_for_session(&app, &conn, &runtime, &session_id)?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    clear_active_session_if_matches(&conn, &session_id)?;
    Ok(())
}

#[tauri::command]
fn send_message(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ChatRuntimeManager>,
    session_id: String,
    content: String,
    config: ChatConfig,
) -> Result<ChatMessage, AppError> {
    let trimmed_content = content.trim().to_string();
    if trimmed_content.is_empty() {
        return Err(AppError::Validation(
            "Message content cannot be empty.".to_string(),
        ));
    }
    let (conn, db_path) = {
        let store = state
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?;
        (store.connection()?, store.db_path.clone())
    };
    let session = load_session(&conn, &session_id)?;
    if session.archived {
        return Err(AppError::Validation(format!(
            "Cannot send a message to archived session: {session_id}"
        )));
    }
    let selected_agent = load_agent(&conn, &session.agent_id)?;

    insert_chat_message(&conn, &session_id, "user", "completed", &trimmed_content)?;
    let assistant_message = insert_chat_message(&conn, &session_id, "assistant", "streaming", "")?;
    update_session_lifecycle(&conn, &session_id, SessionLifecycleState::Starting)?;
    let _ = app.emit(
        "chat:event",
        ChatStreamEvent::Started {
            session_id: session_id.clone(),
            message_id: assistant_message.id.clone(),
        },
    );
    update_session_lifecycle(&conn, &session_id, SessionLifecycleState::Running)?;

    let spawned =
        match spawn_cli_generation(&conn, &session, &selected_agent, &trimmed_content, &config) {
            Ok(spawned) => spawned,
            Err(error) => {
                let concise_error = concise_cli_error(&selected_agent, &error);
                let _ = write_session_runtime_log(
                    &conn,
                    logging::LogLevel::Error,
                    &session_id,
                    &selected_agent.id,
                    &error,
                );
                let failed =
                    fail_assistant_message(&conn, &assistant_message.id, "", &concise_error)?;
                update_session_lifecycle(&conn, &session_id, SessionLifecycleState::Failed)?;
                runtime.complete(&session_id)?;
                let _ = app.emit(
                    "chat:event",
                    ChatStreamEvent::Failed {
                        session_id,
                        message_id: assistant_message.id,
                        error: concise_error,
                    },
                );
                return Ok(failed);
            }
        };

    let stdout = spawned.stdout;
    let stderr = spawned.stderr;
    runtime.start(
        session_id.clone(),
        assistant_message.id.clone(),
        Some(spawned.child),
    )?;
    let background_app = app.clone();
    let background_session_id = session_id.clone();
    let background_agent = selected_agent.clone();
    let background_message_id = assistant_message.id.clone();
    let prompt_len = trimmed_content.chars().count();
    thread::spawn(move || {
        run_cli_generation_stream(
            background_app,
            db_path,
            background_session_id,
            background_agent,
            background_message_id,
            stdout,
            stderr,
            prompt_len,
        );
    });

    Ok(assistant_message)
}

#[tauri::command]
fn list_messages(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    limit: Option<i64>,
    before_id: Option<String>,
) -> Result<Vec<ChatMessage>, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    list_chat_messages(&conn, &session_id, limit, before_id.as_deref())
}

#[tauri::command]
fn get_usage_statistics(
    state: State<'_, Mutex<RegistryStore>>,
    range: UsageStatisticsRange,
) -> Result<UsageStatistics, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    aggregate_usage_statistics(&conn, range)
}

fn stop_generation_for_session(
    app: &AppHandle,
    conn: &Connection,
    runtime: &ChatRuntimeManager,
    session_id: &str,
) -> Result<StopGenerationOutcome, AppError> {
    load_session(conn, session_id)?;
    let stop_outcome = runtime.stop(session_id)?;
    let streaming_ids = {
        let mut stmt =
            conn.prepare("SELECT id FROM messages WHERE session_id = ?1 AND status = 'streaming'")?;
        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    if matches!(stop_outcome, StopGenerationOutcome::NoActiveGeneration) && streaming_ids.is_empty()
    {
        return Ok(stop_outcome);
    }
    cancel_streaming_messages(conn, session_id)?;
    update_session_lifecycle(conn, session_id, SessionLifecycleState::Stopped)?;
    let _ = write_session_runtime_log(
        conn,
        logging::LogLevel::Warn,
        session_id,
        &load_session(conn, session_id)?.agent_id,
        "session generation cancelled",
    );
    if let StopGenerationOutcome::SoftCancelled { message_id }
    | StopGenerationOutcome::ProcessKilled { message_id } = &stop_outcome
    {
        if !streaming_ids.iter().any(|id| id == message_id) {
            let _ = app.emit(
                "chat:event",
                ChatStreamEvent::Cancelled {
                    session_id: session_id.to_string(),
                    message_id: message_id.clone(),
                },
            );
        }
    }
    for message_id in streaming_ids {
        let _ = app.emit(
            "chat:event",
            ChatStreamEvent::Cancelled {
                session_id: session_id.to_string(),
                message_id,
            },
        );
    }
    Ok(stop_outcome)
}

#[tauri::command]
fn stop_generation(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ChatRuntimeManager>,
    session_id: String,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    stop_generation_for_session(&app, &conn, &runtime, &session_id)?;
    Ok(())
}

#[tauri::command]
fn get_session_details(state: State<'_, Mutex<RegistryStore>>) -> Result<SessionDetails, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let workflow = get_workflow_state_from_conn(&conn)?;
    let (adapter, message) = conn.query_row(
        "SELECT adapter, message FROM session_details WHERE id = 1",
        [],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    let mut details = std::collections::BTreeMap::new();
    details.insert("runtime".to_string(), "tauri".to_string());
    details.insert("message".to_string(), message);
    details.insert(
        "nativeDesktopSupported".to_string(),
        native_desktop_supported().to_string(),
    );

    Ok(SessionDetails {
        agent_id: workflow.active_agent_id,
        interaction_mode: workflow.active_interaction_mode,
        lifecycle_state: workflow.lifecycle_state,
        adapter,
        details,
    })
}

fn get_workflow_state_from_conn(conn: &Connection) -> Result<WorkflowState, AppError> {
    let row = conn.query_row(
        "SELECT active_agent_id, active_interaction_mode, lifecycle_state, intent FROM workflow_state WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;

    Ok(WorkflowState {
        active_agent_id: row.0,
        active_interaction_mode: row.1.as_deref().map(parse_mode).transpose()?,
        lifecycle_state: parse_lifecycle_state(&row.2),
        intent: row.3,
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            std::env::set_var("VANEHUB_APP_DATA_DIR", &data_dir);
            let store = RegistryStore::new(data_dir)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            app.manage(Mutex::new(store));
            app.manage(ChatRuntimeManager::default());
            app.manage(tasks::registry::TaskRegistry::default());
            let should_refresh = {
                let store = app.state::<Mutex<RegistryStore>>();
                let store = store.lock().map_err(|err| {
                    Box::new(AppError::Storage(err.to_string())) as Box<dyn std::error::Error>
                })?;
                let conn = store
                    .connection()
                    .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
                should_start_initial_cli_refresh(&conn)
                    .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?
            };
            if should_refresh {
                let registry = app.state::<tasks::registry::TaskRegistry>();
                start_cli_refresh_operation(
                    app.handle().clone(),
                    &registry,
                    "Initial CLI detection refresh",
                )
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_agents,
            list_cli_tools,
            refresh_cli_detections,
            install_cli_version,
            cli_parameters::list_cli_parameter_profiles,
            cli_parameters::save_cli_parameter_profile,
            cli_parameters::reset_cli_parameter_profile,
            get_agent_by_id,
            get_workflow_state,
            select_agent,
            check_browser_readiness,
            launch_active_workflow,
            get_session_details,
            list_known_projects,
            inspect_project,
            select_project_directory,
            create_session,
            list_sessions,
            list_archived_sessions,
            get_active_session,
            switch_session,
            rename_session,
            pin_session,
            unpin_session,
            archive_session,
            unarchive_session,
            delete_session,
            send_message,
            list_messages,
            get_usage_statistics,
            stop_generation,
            get_settings,
            save_setting,
            test_network_proxy,
            scan_network_proxies,
            open_log_directory,
            report_client_log_event,
            get_node_info,
            mcp::commands::list_mcp_servers,
            mcp::commands::add_mcp_server,
            mcp::commands::update_mcp_server,
            mcp::commands::remove_mcp_server,
            mcp::commands::toggle_mcp_server,
            mcp::commands::test_mcp_connection,
            mcp::commands::get_mcp_server_status,
            mcp::commands::import_mcp_servers,
            mcp::commands::export_mcp_servers,
            sdk::commands::list_sdk_definitions,
            sdk::commands::list_sdk_statuses,
            sdk::commands::check_sdk_environment,
            sdk::commands::get_sdk_versions,
            sdk::commands::check_sdk_updates,
            sdk::commands::install_sdk_dependency,
            sdk::commands::update_sdk_dependency,
            sdk::commands::rollback_sdk_dependency,
            sdk::commands::uninstall_sdk_dependency,
            sdk::commands::get_sdk_operation_logs,
            skills::commands::list_skills,
            skills::commands::list_skill_mount_paths,
            skills::commands::update_skill_mount_path,
            skills::commands::create_skill,
            skills::commands::update_skill,
            skills::commands::delete_skill,
            skills::commands::restore_builtin_skill,
            skills::commands::set_skill_enabled,
            skills::commands::set_skill_agent_bindings,
            skills::commands::preview_skill,
            skills::commands::import_skill,
            skills::commands::detect_skill_drift,
            skills::commands::sync_skill_drift,
            skills::commands::select_workspace_directory,
            tasks::commands::list_operations,
            tasks::commands::get_operation_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running VaneHub AI");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        migrate(&conn).expect("migrate");
        seed_agents(&conn).expect("seed");
        conn
    }

    fn test_chat_config(agent_id: &str) -> ChatConfig {
        ChatConfig {
            agent_id: agent_id.to_string(),
            interaction_mode: InteractionMode::Cli,
            permission_mode: "default".to_string(),
            provider_id: None,
            model_id: None,
            reasoning_depth: None,
            streaming: true,
            thinking: false,
            long_context: false,
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("vanehub-ai-{name}-{unique}"))
    }

    #[test]
    fn registry_store_uses_supplied_app_data_directory() {
        let root = unique_temp_dir("store");
        let store = RegistryStore::new(root.clone()).expect("store");

        assert_eq!(store.db_path, root.join("vanehub.sqlite"));
        assert!(root.exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migration_records_applied_versions() {
        let conn = Connection::open_in_memory().expect("sqlite");
        migrate(&conn).expect("migrate");

        let versions = conn
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .expect("prepare")
            .query_map([], |row| row.get::<_, i64>(0))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("versions");

        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn project_worktree_migration_adds_tables_and_columns() {
        let conn = test_conn();

        assert!(table_has_column(&conn, "sessions", "project_path").expect("project column"));
        assert!(table_has_column(&conn, "sessions", "worktree_path").expect("worktree column"));
        conn.execute(
            "INSERT INTO known_projects (path, display_name, is_git, last_opened_at) VALUES (?1, ?2, ?3, ?4)",
            params!["D:\\code\\app", "app", 1, current_timestamp()],
        )
        .expect("insert project");
    }

    #[test]
    fn known_project_upsert_orders_by_last_opened() {
        let conn = test_conn();
        let first = ProjectInspection {
            path: "D:\\code\\first".to_string(),
            display_name: "first".to_string(),
            is_git: true,
            git_root: Some("D:\\code\\first".to_string()),
        };
        let second = ProjectInspection {
            path: "D:\\code\\second".to_string(),
            display_name: "second".to_string(),
            is_git: false,
            git_root: None,
        };

        upsert_known_project(&conn, &first).expect("first");
        upsert_known_project(&conn, &second).expect("second");
        let projects = {
            let mut stmt = conn
                .prepare("SELECT path, display_name, is_git, last_opened_at FROM known_projects ORDER BY last_opened_at DESC, path DESC")
                .expect("prepare");
            stmt.query_map([], load_known_project_from_row)
                .expect("query")
                .collect::<Result<Vec<_>, _>>()
                .expect("projects")
        };

        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|project| project.path == first.path));
        assert!(projects.iter().any(|project| project.path == second.path));
    }

    #[test]
    fn inspect_project_reports_non_git_for_plain_temp_dir() {
        let root = unique_temp_dir("plain-project");
        std::fs::create_dir_all(&root).expect("create temp");

        let inspection = inspect_project_inner(root.to_str().expect("utf8 path")).expect("inspect");

        assert_eq!(
            inspection.path,
            std::fs::canonicalize(&root)
                .expect("canonical")
                .to_string_lossy()
        );
        assert!(!inspection.is_git);
        assert_eq!(inspection.git_root, None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn worktree_name_validation_rejects_unsafe_values() {
        assert_eq!(
            validate_worktree_name("feature-a").expect("valid"),
            "feature-a"
        );
        assert!(validate_worktree_name("").is_err());
        assert!(validate_worktree_name("../bad").is_err());
        assert!(validate_worktree_name("bad\\name").is_err());
    }

    #[test]
    fn worktree_target_uses_sibling_and_rejects_existing_path() {
        let parent = unique_temp_dir("worktree-parent");
        let project = parent.join("app");
        let existing = parent.join("app-feature-a");
        std::fs::create_dir_all(&project).expect("project");

        let target = resolve_worktree_target(&project, "feature-a").expect("target");
        assert_eq!(target, existing);

        std::fs::create_dir_all(&existing).expect("existing");
        assert!(resolve_worktree_target(&project, "feature-a").is_err());
        let _ = std::fs::remove_dir_all(parent);
    }

    #[test]
    fn session_metadata_persists_project_and_worktree_fields() {
        let conn = test_conn();
        let now = current_timestamp();
        conn.execute(
            "INSERT INTO sessions
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, project_path, worktree_path, worktree_name, worktree_branch, pinned, archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, ?11, ?12)",
            params![
                "session-worktree",
                "Worktree",
                "codex-cli",
                "cli",
                "idle",
                "D:\\code\\app-feature-a",
                "D:\\code\\app",
                "D:\\code\\app-feature-a",
                "feature-a",
                "vanehub/feature-a",
                now,
                now
            ],
        )
        .expect("insert session");

        let session = load_session(&conn, "session-worktree").expect("session");

        assert_eq!(session.folder.as_deref(), Some("D:\\code\\app-feature-a"));
        assert_eq!(session.project_path.as_deref(), Some("D:\\code\\app"));
        assert_eq!(
            session.worktree_path.as_deref(),
            Some("D:\\code\\app-feature-a")
        );
        assert_eq!(session.worktree_name.as_deref(), Some("feature-a"));
        assert_eq!(
            session.worktree_branch.as_deref(),
            Some("vanehub/feature-a")
        );
    }

    #[test]
    fn migration_upgrades_existing_agents_table() {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            r#"
            CREATE TABLE agents (
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                provider TEXT NOT NULL,
                launch_kind TEXT NOT NULL,
                launch_command TEXT,
                launch_url TEXT,
                executable_name TEXT
            );
            "#,
        )
        .expect("legacy agents table");

        migrate(&conn).expect("migrate");

        assert!(
            table_has_column(&conn, "agents", "managed_sdk_dependency_id").expect("column check")
        );
    }

    #[test]
    fn migration_adds_session_storage() {
        let conn = test_conn();

        assert!(
            table_has_column(&conn, "workflow_state", "active_session_id").expect("column check")
        );
        assert!(table_has_column(&conn, "sessions", "updated_at").expect("column check"));
        assert!(table_has_column(&conn, "messages", "status").expect("column check"));
        assert!(table_has_column(&conn, "messages", "session_id").expect("column check"));
        assert!(table_has_column(&conn, "settings", "value").expect("column check"));
        assert!(table_has_column(&conn, "sessions", "runtime_session_id").expect("column check"));
    }

    #[test]
    fn settings_repository_merges_defaults_and_saved_values() {
        let conn = test_conn();

        let defaults = get_settings_from_conn(&conn).expect("default settings");
        assert_eq!(defaults.application_language, "zh-CN");
        assert_eq!(defaults.font_size, "14px");
        assert_eq!(defaults.theme, "futuristic");
        assert_eq!(defaults.network_proxy_url, "");
        assert_eq!(defaults.network_proxy_bypass, network_proxy::DEFAULT_BYPASS);

        let saved = save_setting_to_conn(&conn, "fontSize", "18px").expect("save setting");

        assert_eq!(saved.font_size, "18px");
        assert_eq!(saved.application_language, "zh-CN");
    }

    #[test]
    fn settings_repository_rejects_invalid_values() {
        let conn = test_conn();

        let result = save_setting_to_conn(&conn, "fontSize", "20px");

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn settings_repository_normalizes_network_proxy_values() {
        let conn = test_conn();

        let saved =
            save_setting_to_conn(&conn, "networkProxyUrl", "http://user:pass@127.0.0.1:7890")
                .expect("save proxy url");
        assert_eq!(saved.network_proxy_url, "http://user:pass@127.0.0.1:7890");

        let saved = save_setting_to_conn(&conn, "networkProxyBypass", " localhost, 127.0.0.1 ::1 ")
            .expect("save bypass");
        assert_eq!(saved.network_proxy_bypass, "localhost,127.0.0.1,::1");
    }

    #[test]
    fn settings_repository_rejects_invalid_network_proxy_values() {
        let conn = test_conn();

        assert!(save_setting_to_conn(&conn, "networkProxyUrl", "ftp://127.0.0.1:7890").is_err());
        assert!(save_setting_to_conn(&conn, "networkProxyBypass", "localhost\nbad").is_err());
    }

    fn insert_test_session(conn: &Connection, session_id: &str) {
        let now = current_timestamp();
        conn.execute(
            "INSERT INTO sessions
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0, 0, ?6, ?7)",
            params![
                session_id,
                "新会话",
                "gemini-cli",
                "browser",
                "idle",
                now,
                now
            ],
        )
        .expect("insert session");
    }

    #[test]
    fn message_repository_lists_messages_by_session() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");
        insert_test_session(&conn, "session-2");

        let first = insert_chat_message(&conn, "session-1", "user", "completed", "hello")
            .expect("first message");
        let second = insert_chat_message(&conn, "session-1", "assistant", "streaming", "")
            .expect("second message");
        insert_chat_message(&conn, "session-2", "user", "completed", "other")
            .expect("other message");

        let usage = TokenUsage {
            input: 5,
            output: 7,
        };
        complete_assistant_message(&conn, &second.id, "response", &usage).expect("complete");

        let messages = list_chat_messages(&conn, "session-1", Some(50), None).expect("messages");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, first.id);
        assert_eq!(messages[1].status, "completed");
        assert_eq!(
            messages[1].token_usage.as_ref().map(|usage| usage.output),
            Some(7)
        );
    }

    #[test]
    fn message_repository_pages_older_messages() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");

        let first = insert_chat_message(&conn, "session-1", "user", "completed", "first")
            .expect("first message");
        let second = insert_chat_message(&conn, "session-1", "assistant", "completed", "second")
            .expect("second message");
        let third = insert_chat_message(&conn, "session-1", "user", "completed", "third")
            .expect("third message");
        conn.execute(
            "UPDATE messages SET created_at = CASE id WHEN ?1 THEN ?2 WHEN ?3 THEN ?4 WHEN ?5 THEN ?6 ELSE created_at END",
            params![
                first.id.as_str(),
                "2026-07-15T00:00:01Z",
                second.id.as_str(),
                "2026-07-15T00:00:02Z",
                third.id.as_str(),
                "2026-07-15T00:00:03Z"
            ],
        )
        .expect("stable message timestamps");

        let newest = list_chat_messages(&conn, "session-1", Some(2), None).expect("newest page");
        assert_eq!(
            newest
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec![second.id.as_str(), third.id.as_str()]
        );

        let older =
            list_chat_messages(&conn, "session-1", Some(2), Some(&second.id)).expect("older page");
        assert_eq!(
            older
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec![first.id.as_str()]
        );
    }

    #[test]
    fn usage_statistics_aggregates_all_time_message_usage() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");
        insert_test_session(&conn, "session-2");

        let first = insert_chat_message(&conn, "session-1", "assistant", "streaming", "")
            .expect("first assistant");
        let second = insert_chat_message(&conn, "session-1", "assistant", "streaming", "")
            .expect("second assistant");
        let third = insert_chat_message(&conn, "session-2", "assistant", "streaming", "")
            .expect("third assistant");
        insert_chat_message(&conn, "session-2", "user", "completed", "not counted")
            .expect("user message");

        complete_assistant_message(
            &conn,
            &first.id,
            "first",
            &TokenUsage {
                input: 10,
                output: 20,
            },
        )
        .expect("complete first");
        complete_assistant_message(
            &conn,
            &second.id,
            "second",
            &TokenUsage {
                input: 5,
                output: 7,
            },
        )
        .expect("complete second");
        complete_assistant_message(
            &conn,
            &third.id,
            "third",
            &TokenUsage {
                input: 0,
                output: 0,
            },
        )
        .expect("complete third");

        let stats =
            aggregate_usage_statistics(&conn, UsageStatisticsRange::All).expect("usage stats");

        assert_eq!(stats.input_tokens, 15);
        assert_eq!(stats.output_tokens, 27);
        assert_eq!(stats.total_tokens, 42);
        assert_eq!(stats.counted_messages, 2);
        assert_eq!(stats.counted_sessions, 1);
    }

    #[test]
    fn usage_statistics_filters_by_range_start() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");

        let old_message = insert_chat_message(&conn, "session-1", "assistant", "streaming", "")
            .expect("old assistant");
        let recent_message = insert_chat_message(&conn, "session-1", "assistant", "streaming", "")
            .expect("recent assistant");
        complete_assistant_message(
            &conn,
            &old_message.id,
            "old",
            &TokenUsage {
                input: 100,
                output: 100,
            },
        )
        .expect("complete old");
        complete_assistant_message(
            &conn,
            &recent_message.id,
            "recent",
            &TokenUsage {
                input: 8,
                output: 13,
            },
        )
        .expect("complete recent");

        let old_timestamp = (Utc::now() - ChronoDuration::days(40)).to_rfc3339();
        conn.execute(
            "UPDATE messages SET created_at = ?1 WHERE id = ?2",
            params![old_timestamp, old_message.id],
        )
        .expect("age old message");

        let stats = aggregate_usage_statistics(&conn, UsageStatisticsRange::Last30Days)
            .expect("range stats");

        assert_eq!(stats.input_tokens, 8);
        assert_eq!(stats.output_tokens, 13);
        assert_eq!(stats.counted_messages, 1);
    }

    #[test]
    fn deleting_session_cascades_messages() {
        let conn = test_conn();
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys");
        insert_test_session(&conn, "session-1");
        insert_chat_message(&conn, "session-1", "user", "completed", "hello").expect("message");

        conn.execute("DELETE FROM sessions WHERE id = ?1", params!["session-1"])
            .expect("delete session");

        let count = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
                params!["session-1"],
                |row| row.get::<_, i64>(0),
            )
            .expect("message count");
        assert_eq!(count, 0);
    }

    #[test]
    fn generic_parser_emits_non_empty_lines_as_tokens() {
        assert_eq!(
            GenericLineParser.parse_line("hello"),
            ParsedAgentEvent::Token("hello".to_string())
        );
        assert_eq!(GenericLineParser.parse_line("  "), ParsedAgentEvent::Empty);
    }

    #[test]
    fn claude_parser_reads_text_deltas_and_errors() {
        assert_eq!(
            ClaudeCodeParser
                .parse_line(r#"{"type":"content_block_delta","delta":{"text":"hello"}}"#),
            ParsedAgentEvent::Token("hello".to_string())
        );
        assert_eq!(
            ClaudeCodeParser.parse_line(r#"{"type":"thinking_delta","delta":{"thinking":"plan"}}"#),
            ParsedAgentEvent::Thinking("plan".to_string())
        );
        assert_eq!(
            ClaudeCodeParser.parse_line(r#"{"type":"error","message":"boom"}"#),
            ParsedAgentEvent::Failed("boom".to_string())
        );
    }

    #[test]
    fn structured_parser_reads_session_text_and_tools() {
        assert_eq!(
            StructuredJsonParser
                .parse_line(r#"{"type":"session_init","session_id":"provider-session-1"}"#),
            ParsedAgentEvent::SessionId("provider-session-1".to_string())
        );
        assert_eq!(
            StructuredJsonParser.parse_line(r#"{"type":"message","text":"hello"}"#),
            ParsedAgentEvent::Token("hello".to_string())
        );
        assert_eq!(
            StructuredJsonParser.parse_line(r#"{"type":"reasoning","reasoning":"checking"}"#),
            ParsedAgentEvent::Thinking("checking".to_string())
        );
        assert_eq!(
            StructuredJsonParser.parse_line(r#"{"type":"tool_call","id":"t1","name":"read"}"#),
            ParsedAgentEvent::ToolUse(ToolUseBlock {
                id: "t1".to_string(),
                name: "read".to_string(),
                input: None,
                output: None,
                status: "running".to_string(),
            })
        );
    }

    #[test]
    fn provider_command_builders_use_expected_shapes() {
        let conn = test_conn();
        let claude = load_agent(&conn, "claude-code").expect("claude");
        let codex = load_agent(&conn, "codex-cli").expect("codex");
        let gemini = load_agent(&conn, "gemini-cli").expect("gemini");
        let opencode = load_agent(&conn, "opencode").expect("opencode");

        let claude_spec = build_cli_invocation_spec(
            &claude,
            "claude".to_string(),
            "hello",
            Some("claude-session"),
            &[],
        )
        .expect("claude spec");
        assert_eq!(claude_spec.prompt_delivery, PromptDelivery::Stdin);
        assert!(claude_spec.args.iter().any(|arg| arg == "--resume"));
        assert!(!claude_spec.args.iter().any(|arg| arg == "hello"));

        let codex_spec = build_cli_invocation_spec(
            &codex,
            "codex".to_string(),
            "hello",
            Some("codex-session"),
            &[],
        )
        .expect("codex spec");
        assert_eq!(codex_spec.prompt_delivery, PromptDelivery::Stdin);
        assert_eq!(
            codex_spec.args,
            vec!["exec", "resume", "codex-session", "--json", "--", "-"]
        );

        let gemini_spec =
            build_cli_invocation_spec(&gemini, "gemini".to_string(), "hello", None, &[])
                .expect("gemini spec");
        assert_eq!(gemini_spec.prompt_delivery, PromptDelivery::Argument);
        assert!(gemini_spec
            .args
            .windows(2)
            .any(|pair| pair == ["-p", "hello"]));

        let opencode_spec = build_cli_invocation_spec(
            &opencode,
            "opencode".to_string(),
            "hello",
            Some("open-1"),
            &[],
        )
        .expect("opencode spec");
        assert_eq!(opencode_spec.prompt_delivery, PromptDelivery::Argument);
        assert!(opencode_spec
            .args
            .windows(2)
            .any(|pair| pair == ["--session", "open-1"]));
        assert_eq!(opencode_spec.args.last().map(String::as_str), Some("hello"));
    }

    #[test]
    fn provider_command_builders_preserve_fresh_and_resume_shapes_with_default_profiles() {
        let conn = test_conn();
        for agent_id in cli_parameters::MANAGED_CLI_AGENT_IDS {
            let agent = load_agent(&conn, agent_id).expect("agent");
            let selections = cli_parameters::load_selections(&conn, agent_id).expect("profile");
            let args = cli_parameters::preview_args(
                agent_id,
                &selections,
                cli_parameters::CliParameterLaunchScope::Chat,
            )
            .expect("args");
            let fresh =
                build_cli_invocation_spec(&agent, agent_id.to_string(), "prompt", None, &args)
                    .expect("fresh");
            let resumed = build_cli_invocation_spec(
                &agent,
                agent_id.to_string(),
                "prompt",
                Some("provider-session"),
                &args,
            )
            .expect("resumed");
            assert!(!fresh
                .args
                .iter()
                .any(|argument| argument == "provider-session"));
            assert!(resumed
                .args
                .iter()
                .any(|argument| argument == "provider-session"));
            match agent_id {
                "claude-code" => assert!(fresh.args.contains(&"stream-json".to_string())),
                "codex-cli" => assert!(fresh.args.contains(&"exec".to_string())),
                "gemini-cli" => {
                    assert!(fresh.args.contains(&"--approval-mode".to_string()));
                    assert!(fresh.args.contains(&"stream-json".to_string()));
                }
                "opencode" => assert!(fresh.args.contains(&"run".to_string())),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn provider_command_builders_place_managed_arguments_without_overriding_runtime_tokens() {
        let conn = test_conn();
        let codex = load_agent(&conn, "codex-cli").expect("codex");
        let spec = build_cli_invocation_spec(
            &codex,
            "codex".to_string(),
            "secret prompt",
            Some("session-1"),
            &[
                "--model".to_string(),
                "gpt-5.4".to_string(),
                "--ephemeral".to_string(),
            ],
        )
        .expect("spec");
        assert_eq!(
            spec.args,
            vec![
                "--model",
                "gpt-5.4",
                "exec",
                "resume",
                "session-1",
                "--ephemeral",
                "--json",
                "--",
                "-",
            ]
        );
        assert!(!spec.args.iter().any(|argument| argument == "secret prompt"));
    }

    #[test]
    fn chat_config_overrides_persisted_profile_without_mutating_it() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at) VALUES (?1, ?2, 1, ?3, ?4)",
            params!["codex-cli", "sandbox", "\"read-only\"", current_timestamp()],
        )
        .expect("persist profile");
        let config = ChatConfig {
            permission_mode: "agent".to_string(),
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-5-4".to_string()),
            reasoning_depth: Some("max".to_string()),
            ..test_chat_config("codex-cli")
        };
        let effective = effective_cli_selections(&conn, "codex-cli", &config).expect("effective");
        assert_eq!(effective["model"], "gpt-5.4");
        assert_eq!(effective["reasoningEffort"], "xhigh");
        assert_eq!(effective["sandbox"], "workspace-write");
        assert_eq!(effective["approvalPolicy"], "on-request");
        assert_eq!(
            cli_parameters::load_selections(&conn, "codex-cli").expect("persisted")["sandbox"],
            "read-only"
        );
    }

    #[test]
    fn chat_config_maps_supported_values_and_omits_unknown_models() {
        let conn = test_conn();
        let claude = effective_cli_selections(
            &conn,
            "claude-code",
            &ChatConfig {
                permission_mode: "plan".to_string(),
                model_id: Some("claude-opus-4-8".to_string()),
                reasoning_depth: Some("high".to_string()),
                ..test_chat_config("claude-code")
            },
        )
        .expect("claude");
        assert_eq!(claude["model"], "opus");
        assert_eq!(claude["effort"], "high");
        assert_eq!(claude["permissionMode"], "plan");

        let gemini = effective_cli_selections(
            &conn,
            "gemini-cli",
            &ChatConfig {
                permission_mode: "plan".to_string(),
                model_id: Some("gemini-2-5-pro".to_string()),
                ..test_chat_config("gemini-cli")
            },
        )
        .expect("gemini");
        assert_eq!(gemini["model"], "gemini-2.5-pro");
        assert_eq!(gemini["approvalMode"], "plan");

        let opencode = effective_cli_selections(
            &conn,
            "opencode",
            &ChatConfig {
                permission_mode: "auto".to_string(),
                thinking: true,
                ..test_chat_config("opencode")
            },
        )
        .expect("opencode");
        assert_eq!(opencode["autoApprove"], true);
        assert_eq!(opencode["thinking"], true);

        let unknown = effective_cli_selections(
            &conn,
            "claude-code",
            &ChatConfig {
                model_id: Some("unrecognized-model".to_string()),
                ..test_chat_config("claude-code")
            },
        )
        .expect("unknown model");
        assert_eq!(unknown["model"], "default");
    }

    #[test]
    fn selection_snapshots_change_only_for_the_next_process() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at) VALUES ('claude-code', 'effort', 1, '\"low\"', ?1)",
            params![current_timestamp()],
        )
        .expect("initial profile");
        let first =
            effective_cli_selections(&conn, "claude-code", &test_chat_config("claude-code"))
                .expect("first process");
        conn.execute(
            "UPDATE cli_parameter_settings SET value_json = '\"high\"', updated_at = ?1 WHERE agent_id = 'claude-code' AND parameter_id = 'effort'",
            params![current_timestamp()],
        )
        .expect("updated profile");
        let next = effective_cli_selections(&conn, "claude-code", &test_chat_config("claude-code"))
            .expect("next process");
        assert_eq!(first["effort"], "low");
        assert_eq!(next["effort"], "high");
    }

    #[test]
    fn session_runtime_session_id_round_trips() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");

        let updated = update_session_runtime_session_id(&conn, "session-1", "provider-123")
            .expect("update runtime session id");

        assert_eq!(updated.runtime_session_id.as_deref(), Some("provider-123"));
        let loaded = load_session(&conn, "session-1").expect("load session");
        assert_eq!(loaded.runtime_session_id.as_deref(), Some("provider-123"));
    }

    #[test]
    fn runtime_manager_tracks_and_soft_cancels_generation() {
        let manager = ChatRuntimeManager::default();
        manager
            .start("session-1".to_string(), "message-1".to_string(), None)
            .expect("start");

        let outcome = manager.stop("session-1").expect("stop");

        assert_eq!(
            outcome,
            StopGenerationOutcome::SoftCancelled {
                message_id: "message-1".to_string()
            }
        );
        assert_eq!(
            manager.stop("session-1").expect("second stop"),
            StopGenerationOutcome::NoActiveGeneration
        );
    }

    #[test]
    fn session_lifecycle_update_syncs_active_workflow() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");
        let session = load_session(&conn, "session-1").expect("session");
        update_active_workflow_for_session(&conn, &session).expect("active session");

        let updated = update_session_lifecycle(&conn, "session-1", SessionLifecycleState::Running)
            .expect("update lifecycle");
        let workflow = get_workflow_state_from_conn(&conn).expect("workflow");

        assert!(matches!(
            updated.lifecycle_state,
            SessionLifecycleState::Running
        ));
        assert!(matches!(
            workflow.lifecycle_state,
            SessionLifecycleState::Running
        ));
    }

    #[test]
    fn assistant_failure_message_keeps_concise_error() {
        let conn = test_conn();
        insert_test_session(&conn, "session-1");
        let assistant = insert_chat_message(&conn, "session-1", "assistant", "streaming", "")
            .expect("assistant");

        let failed = fail_assistant_message(&conn, &assistant.id, "", "Codex CLI unavailable")
            .expect("fail assistant");

        assert_eq!(failed.status, "failed");
        assert_eq!(failed.error.as_deref(), Some("Codex CLI unavailable"));
        assert!(failed.content.is_empty());
    }

    #[test]
    fn generic_cli_adapter_rejects_unsupported_agent() {
        let conn = test_conn();
        let mut agent = load_agent(&conn, "codex-cli").expect("agent");
        agent.id = "unknown-agent".to_string();

        let error = execute_generic_cli_agent(&conn, "session-1", &agent, "hello")
            .expect_err("unsupported agent");

        assert!(error.contains("not supported"));
    }

    #[test]
    fn archive_and_delete_clear_active_session() {
        let conn = test_conn();
        let now = current_timestamp();
        conn.execute(
            "INSERT INTO sessions
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0, 0, ?6, ?7)",
            params![
                "session-1",
                "新会话",
                "gemini-cli",
                "browser",
                "idle",
                now,
                now
            ],
        )
        .expect("insert session");
        let session = load_session(&conn, "session-1").expect("session");
        update_active_workflow_for_session(&conn, &session).expect("active session");

        clear_active_session_if_matches(&conn, "session-1").expect("clear active session");

        let active_session_id = conn
            .query_row(
                "SELECT active_session_id FROM workflow_state WHERE id = 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("active session id");
        assert_eq!(active_session_id, None);
    }

    #[test]
    fn seed_agents_are_queryable_by_stable_id() {
        let conn = test_conn();
        let agent = load_agent(&conn, "codex-cli").expect("codex agent");

        assert_eq!(agent.id, "codex-cli");
        assert_eq!(agent.display_name, "Codex CLI");
        assert!(agent.capability_tags.iter().any(|tag| tag == "cli"));
    }

    #[test]
    fn capability_tags_filter_expected_agents() {
        let conn = test_conn();
        let ids: Vec<String> = ["claude-code", "opencode", "codex-cli", "gemini-cli"]
            .iter()
            .map(|id| load_agent(&conn, id).expect("agent"))
            .filter(|agent| agent.capability_tags.iter().any(|tag| tag == "browser"))
            .map(|agent| agent.id)
            .collect();

        assert_eq!(ids, vec!["gemini-cli"]);
    }

    #[test]
    fn workflow_state_preserves_stable_values() {
        let conn = test_conn();
        conn.execute(
            "UPDATE workflow_state SET active_agent_id = ?1, active_interaction_mode = ?2, lifecycle_state = ?3 WHERE id = 1",
            params!["gemini-cli", "browser", "running"],
        )
        .expect("update workflow");

        let workflow = get_workflow_state_from_conn(&conn).expect("workflow");

        assert_eq!(workflow.active_agent_id.as_deref(), Some("gemini-cli"));
        assert!(matches!(
            workflow.active_interaction_mode,
            Some(InteractionMode::Browser)
        ));
        assert!(matches!(
            workflow.lifecycle_state,
            SessionLifecycleState::Running
        ));
    }

    #[test]
    fn browser_readiness_requires_browser_support() {
        let conn = test_conn();
        let gemini = load_agent(&conn, "gemini-cli").expect("gemini");
        let opencode = load_agent(&conn, "opencode").expect("opencode");

        assert!(check_browser_readiness_inner(&gemini).ready);
        assert!(!check_browser_readiness_inner(&opencode).ready);
    }

    #[test]
    fn managed_sdk_dependency_marks_agent_unavailable_without_launch() {
        let (state, reason) = availability_for(None, Some("claude-sdk"));

        assert!(matches!(state, AvailabilityState::Unavailable));
        assert_eq!(
            reason.as_deref(),
            Some("Managed SDK dependency 'claude-sdk' is not installed.")
        );
    }

    #[test]
    fn cli_catalog_preserves_fixed_order_and_metadata() {
        let ids = CLI_TOOL_DEFINITIONS
            .iter()
            .map(|definition| definition.agent_id)
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["claude-code", "codex-cli", "gemini-cli", "opencode"]
        );
        assert_eq!(
            cli_tool_definition("codex-cli")
                .expect("codex definition")
                .package_name,
            "@openai/codex"
        );
        assert!(cli_tool_definition("unknown").is_none());
    }

    #[test]
    fn cli_package_args_are_explicit_for_all_managed_clis() {
        let expected = [
            ("claude-code", "@anthropic-ai/claude-code", "claude"),
            ("codex-cli", "@openai/codex", "codex"),
            ("gemini-cli", "@google/gemini-cli", "gemini"),
            ("opencode", "opencode-ai", "opencode"),
        ];

        for (agent_id, package_name, executable_name) in expected {
            let definition = cli_tool_definition(agent_id).expect("managed cli definition");
            assert_eq!(definition.package_name, package_name);
            assert_eq!(definition.executable_name, executable_name);
            assert_eq!(
                cli_package_install_args(definition, "1.2.3"),
                vec![
                    "install".to_string(),
                    "-g".to_string(),
                    format!("{package_name}@1.2.3")
                ]
            );
        }
        assert!(cli_tool_definition("unsupported-cli").is_none());
        assert!(is_stable_version("1.2.3"));
        assert!(!is_stable_version("1.2.3-beta.1"));
    }

    #[test]
    fn cli_diagnostic_logs_persist_through_unified_redaction() {
        let conn = test_conn();
        let dir = unique_temp_dir("cli-diagnostic-logs");
        std::fs::create_dir_all(&dir).expect("log dir");
        save_setting_to_conn(&conn, "logDirectory", dir.to_str().expect("utf8 dir"))
            .expect("save log dir");

        let definition = cli_tool_definition("opencode").expect("opencode definition");
        let package_args = cli_package_install_args(definition, "1.18.2");
        let package_context = cli_package_failure_context(
            definition,
            "op-package",
            CliPackageFailureDetails {
                target_version: "1.18.2",
                npm_executable: "npm",
                args: &package_args,
                stdout: Some("token=abc123"),
                stderr: Some("password:super-secret"),
                exit_status: Some("1"),
                timeout_reason: None,
                error: "npm install failed token=abc123",
            },
        );
        write_cli_diagnostic_log_to_conn(
            &conn,
            logging::LogLevel::Error,
            "CLI package operation failed token=abc123.",
            package_context,
        )
        .expect("write package log");

        let mut detection_context = cli_definition_context(definition, "op-refresh");
        detection_context.insert(
            "attemptedOperation".to_string(),
            "npm-view-version".to_string(),
        );
        detection_context.insert("reason".to_string(), "command timed out".to_string());
        detection_context.insert("timeoutReason".to_string(), "command timed out".to_string());
        write_cli_diagnostic_log_to_conn(
            &conn,
            logging::LogLevel::Warn,
            "CLI detection diagnostic failure.",
            detection_context,
        )
        .expect("write detection log");

        let content = std::fs::read_to_string(dir.join("vanehub.log")).expect("log file");
        assert!(content.contains("\"category\":\"cli.diagnostic\""));
        assert!(content.contains("\"agentId\":\"opencode\""));
        assert!(content.contains("\"npmArguments\":\"install -g opencode-ai@1.18.2\""));
        assert!(content.contains("\"attemptedOperation\":\"npm-view-version\""));
        assert!(content.contains("\"timeoutReason\":\"command timed out\""));
        assert!(!content.contains("abc123"));
        assert!(!content.contains("super-secret"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn stable_versions_filter_excludes_prerelease_and_limits() {
        let raw = serde_json::to_string(&vec![
            "1.0.0",
            "1.1.0-beta.1",
            "1.1.0",
            "2.0.0-rc.1",
            "2.0.0",
        ])
        .expect("json");

        assert_eq!(
            stable_versions_from_npm_json(&raw, 2),
            vec!["2.0.0".to_string(), "1.1.0".to_string()]
        );
    }

    #[test]
    fn cli_cached_status_reads_without_detection_result() {
        let conn = test_conn();
        let statuses = load_cli_tool_statuses(&conn).expect("statuses");

        assert_eq!(statuses.len(), 4);
        assert_eq!(statuses[0].agent_id, "claude-code");
        assert_eq!(statuses[0].installed, None);
        assert!(matches!(
            statuses[0].version_check_status,
            CliVersionCheckStatus::NotDetected
        ));
        assert!(should_start_initial_cli_refresh(&conn).expect("initial refresh needed"));

        conn.execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at) VALUES ('claude-code', 'chrome', 1, 'true', ?1)",
            params![current_timestamp()],
        )
        .expect("interactive parameter");
        let after_profile_change = load_cli_tool_statuses(&conn).expect("statuses after profile");
        assert_eq!(after_profile_change[0].installed, statuses[0].installed);
        assert_eq!(
            cli_version_check_status_str(&after_profile_change[0].version_check_status),
            cli_version_check_status_str(&statuses[0].version_check_status)
        );
    }

    #[test]
    fn cli_cached_status_round_trips() {
        let conn = test_conn();
        let mut status = status_from_row(CLI_TOOL_DEFINITIONS[1], None);
        status.installed = Some(true);
        status.current_version = Some("1.2.3".to_string());
        status.latest_version = Some("1.3.0".to_string());
        status.available_versions = vec!["1.3.0".to_string(), "1.2.3".to_string()];
        status.detected_path = Some("C:\\Users\\dev\\codex.cmd".to_string());
        status.last_checked_at = Some("123".to_string());
        status.version_check_status = CliVersionCheckStatus::Succeeded;

        save_cli_tool_status(&conn, &status).expect("save");
        let loaded = load_cli_tool_status(&conn, CLI_TOOL_DEFINITIONS[1]).expect("load");

        assert!(!should_start_initial_cli_refresh(&conn).expect("initial refresh not needed"));
        assert_eq!(loaded.installed, Some(true));
        assert_eq!(loaded.current_version.as_deref(), Some("1.2.3"));
        assert_eq!(loaded.available_versions, vec!["1.3.0", "1.2.3"]);
        assert_eq!(
            loaded.detected_path.as_deref(),
            Some("C:\\Users\\dev\\codex.cmd")
        );
    }

    #[test]
    fn version_comparison_handles_upgrade_and_downgrade() {
        assert_eq!(
            compare_versions("1.3.0", "1.2.9"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_versions("1.2.0", "1.2"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_versions("1.1.9", "1.2.0"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(compare_versions("1.2.0-beta.1", "1.2.0"), None);
    }
}
