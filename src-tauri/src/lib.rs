use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use thiserror::Error;

mod command_safety;
mod mcp;
mod sdk;
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
    pinned: bool,
    archived: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatConfig {
    agent_id: String,
    interaction_mode: InteractionMode,
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
    } else {
        Box::new(GenericLineParser)
    }
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

fn default_app_settings() -> AppSettings {
    AppSettings {
        application_language: "zh-CN".to_string(),
        font_size: "14px".to_string(),
        theme: "futuristic".to_string(),
        default_folder_path: String::new(),
    }
}

fn validate_setting_value(key: &str, value: &str) -> Result<(), AppError> {
    let valid = match key {
        "applicationLanguage" => matches!(value, "zh-CN" | "en"),
        "fontSize" => matches!(value, "12px" | "14px" | "16px" | "18px"),
        "theme" => matches!(value, "futuristic" | "minimal"),
        "defaultFolderPath" => true,
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
    let defaults = default_app_settings();
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

    Ok(AppSettings {
        application_language,
        font_size,
        theme,
        default_folder_path,
    })
}

fn save_setting_to_conn(conn: &Connection, key: &str, value: &str) -> Result<AppSettings, AppError> {
    validate_setting_value(key, value)?;
    let now = current_timestamp();
    conn.execute(
        r#"
        INSERT INTO settings (key, value, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?3)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
        "#,
        params![key, value, now],
    )?;
    get_settings_from_conn(conn)
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
        pinned: row.get::<_, i64>(6)? != 0,
        archived: row.get::<_, i64>(7)? != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn session_select_sql() -> &'static str {
    "SELECT id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, created_at, updated_at FROM sessions"
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

fn cancel_streaming_messages(conn: &Connection, session_id: &str) -> Result<(), AppError> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE messages SET status = 'cancelled', updated_at = ?1 WHERE session_id = ?2 AND status = 'streaming'",
        params![now, session_id],
    )?;
    Ok(())
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
            if let Err(error) = launch_command_if_present(&agent) {
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
            if let Err(error) = launch_command_if_present(&agent) {
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

fn launch_command_if_present(agent: &AgentRegistryEntry) -> Result<(), AppError> {
    let Some(command) = agent.launch.command.as_deref() else {
        return Ok(());
    };

    if !command_exists(command) {
        return Err(AppError::LaunchFailed(format!(
            "Command '{command}' was not found on PATH."
        )));
    }

    command_safety::audit_command("command.launch", command, &[]);
    let mut process = command_safety::std_command(command)?;
    process
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
fn create_session(
    state: State<'_, Mutex<RegistryStore>>,
    agent_id: String,
    interaction_mode: InteractionMode,
    title: Option<String>,
    folder: Option<String>,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let agent = load_agent(&conn, &agent_id)?;
    if !agent
        .supported_interaction_modes
        .iter()
        .any(|mode| mode.as_str() == interaction_mode.as_str())
    {
        return Err(AppError::UnsupportedInteractionMode(
            interaction_mode.as_str().to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = current_timestamp();
    let session_title = title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "新会话".to_string());
    conn.execute(
        "INSERT INTO sessions
         (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, ?7, ?8)",
        params![
            id,
            session_title,
            agent_id,
            interaction_mode.as_str(),
            SessionLifecycleState::Idle.as_str(),
            folder,
            now,
            now
        ],
    )?;

    let session = load_session(&conn, &id)?;
    update_active_workflow_for_session(&conn, &session)?;
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
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<Session, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
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
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_session(&conn, &session_id)?;
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
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_session(&conn, &session_id)?;
    let selected_agent = load_agent(&conn, &config.agent_id)?;

    let _user_message =
        insert_chat_message(&conn, &session_id, "user", "completed", &trimmed_content)?;
    let assistant_message = insert_chat_message(&conn, &session_id, "assistant", "streaming", "")?;
    runtime.start(session_id.clone(), assistant_message.id.clone(), None)?;
    let _ = app.emit(
        "chat:event",
        ChatStreamEvent::Started {
            session_id: session_id.clone(),
            message_id: assistant_message.id.clone(),
        },
    );

    let response = format!(
        "Desktop preview response from {}: received \"{}\". Real Agent CLI streaming will attach behind the same message contract.",
        selected_agent.display_name, trimmed_content
    );
    let token_usage = TokenUsage {
        input: trimmed_content.chars().count() as i64,
        output: response.chars().count() as i64,
    };
    let parser = parser_for_agent(&selected_agent.id);
    if let ParsedAgentEvent::Token(content_delta) = parser.parse_line(&response) {
        let _ = app.emit(
            "chat:event",
            ChatStreamEvent::Token {
                session_id: session_id.clone(),
                message_id: assistant_message.id.clone(),
                content_delta,
            },
        );
    }
    let completed =
        complete_assistant_message(&conn, &assistant_message.id, &response, &token_usage)?;
    runtime.complete(&session_id)?;
    let _ = app.emit(
        "chat:event",
        ChatStreamEvent::Completed {
            session_id,
            message_id: assistant_message.id,
            token_usage: Some(token_usage),
        },
    );
    Ok(completed)
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
fn stop_generation(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ChatRuntimeManager>,
    session_id: String,
) -> Result<(), AppError> {
    let stop_outcome = runtime.stop(&session_id)?;
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_session(&conn, &session_id)?;
    let streaming_ids = {
        let mut stmt =
            conn.prepare("SELECT id FROM messages WHERE session_id = ?1 AND status = 'streaming'")?;
        let rows = stmt.query_map(params![session_id.as_str()], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    cancel_streaming_messages(&conn, &session_id)?;
    if let StopGenerationOutcome::SoftCancelled { message_id }
    | StopGenerationOutcome::ProcessKilled { message_id } = stop_outcome
    {
        if !streaming_ids.iter().any(|id| id == &message_id) {
            let _ = app.emit(
                "chat:event",
                ChatStreamEvent::Cancelled {
                    session_id: session_id.clone(),
                    message_id,
                },
            );
        }
    }
    for message_id in streaming_ids {
        let _ = app.emit(
            "chat:event",
            ChatStreamEvent::Cancelled {
                session_id: session_id.clone(),
                message_id,
            },
        );
    }
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
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            let store = RegistryStore::new(data_dir)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            app.manage(Mutex::new(store));
            app.manage(ChatRuntimeManager::default());
            app.manage(tasks::registry::TaskRegistry::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_agents,
            get_agent_by_id,
            get_workflow_state,
            select_agent,
            check_browser_readiness,
            launch_active_workflow,
            get_session_details,
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
            stop_generation,
            get_settings,
            save_setting,
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

        assert_eq!(versions, vec![1, 2, 3, 4, 5]);
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
    }

    #[test]
    fn settings_repository_merges_defaults_and_saved_values() {
        let conn = test_conn();

        let defaults = get_settings_from_conn(&conn).expect("default settings");
        assert_eq!(defaults.application_language, "zh-CN");
        assert_eq!(defaults.font_size, "14px");
        assert_eq!(defaults.theme, "futuristic");

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
}
