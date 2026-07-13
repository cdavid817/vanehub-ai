use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::State;
use thiserror::Error;

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
    #[error("launch failed: {0}")]
    LaunchFailed(String),
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

struct RegistryStore {
    db_path: PathBuf,
}

impl RegistryStore {
    fn new() -> Result<Self, AppError> {
        let mut root = std::env::current_dir().map_err(|err| AppError::Storage(err.to_string()))?;
        root.push(".vanehub");
        std::fs::create_dir_all(&root).map_err(|err| AppError::Storage(err.to_string()))?;
        Ok(Self {
            db_path: root.join("vanehub.sqlite"),
        })
    }

    fn connection(&self) -> Result<Connection, AppError> {
        let conn = Connection::open(&self.db_path)?;
        migrate(&conn)?;
        seed_agents(&conn)?;
        Ok(conn)
    }
}

fn migrate(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            provider TEXT NOT NULL,
            launch_kind TEXT NOT NULL,
            launch_command TEXT,
            launch_url TEXT,
            executable_name TEXT
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
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO workflow_state (id, lifecycle_state, intent) VALUES (1, ?1, ?2)",
        params![SessionLifecycleState::Idle.as_str(), "Current development workflow"],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO session_details (id, adapter, message) VALUES (1, ?1, ?2)",
        params!["none", "No active session."],
    )?;

    Ok(())
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
            vec!["cli", "browser"],
            vec!["coding", "cli", "browser"],
        ),
    ];

    for (id, display_name, provider, kind, command, url, executable, modes, tags) in agents {
        conn.execute(
            "INSERT OR IGNORE INTO agents (id, display_name, provider, launch_kind, launch_command, launch_url, executable_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, display_name, provider, kind, command, url, executable],
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

fn load_agent(conn: &Connection, agent_id: &str) -> Result<AgentRegistryEntry, AppError> {
    let row = conn
        .query_row(
            "SELECT id, display_name, provider, launch_kind, launch_command, launch_url, executable_name
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
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::AgentNotFound(agent_id.to_string()))?;

    let modes = load_modes(conn, &row.0)?;
    let tags = load_tags(conn, &row.0)?;
    let (availability_state, unavailable_reason) = availability_for(row.6.as_deref());

    Ok(AgentRegistryEntry {
        id: row.0,
        display_name: row.1,
        provider: row.2,
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
    let mut stmt = conn.prepare("SELECT mode FROM agent_modes WHERE agent_id = ?1 ORDER BY mode")?;
    let rows = stmt.query_map(params![agent_id], |row| row.get::<_, String>(0))?;
    let mut modes = Vec::new();
    for row in rows {
        modes.push(parse_mode(&row?)?);
    }
    Ok(modes)
}

fn load_tags(conn: &Connection, agent_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT tag FROM agent_capability_tags WHERE agent_id = ?1 ORDER BY tag")?;
    let rows = stmt.query_map(params![agent_id], |row| row.get::<_, String>(0))?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}

fn availability_for(executable_name: Option<&str>) -> (AvailabilityState, Option<String>) {
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
    cfg!(any(target_os = "windows", target_os = "macos", target_os = "linux"))
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
fn list_agents(
    state: State<'_, Mutex<RegistryStore>>,
    capability_tag: Option<String>,
) -> Result<Vec<AgentRegistryEntry>, AppError> {
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let mut stmt = conn.prepare("SELECT id FROM agents ORDER BY display_name")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut agents = Vec::new();
    for row in rows {
        let agent = load_agent(&conn, &row?)?;
        if let Some(tag) = capability_tag.as_deref() {
            if !agent.capability_tags.iter().any(|candidate| candidate == tag) {
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
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    load_agent(&conn, &agent_id)
}

#[tauri::command]
fn get_workflow_state(state: State<'_, Mutex<RegistryStore>>) -> Result<WorkflowState, AppError> {
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    get_workflow_state_from_conn(&conn)
}

#[tauri::command]
fn select_agent(
    state: State<'_, Mutex<RegistryStore>>,
    agent_id: String,
    interaction_mode: InteractionMode,
) -> Result<WorkflowState, AppError> {
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let agent = load_agent(&conn, &agent_id)?;

    if matches!(agent.availability_state, AvailabilityState::Unavailable | AvailabilityState::NeedsAuth) {
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
        return Err(AppError::UnsupportedInteractionMode(interaction_mode.as_str().to_string()));
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
        params![agent_id, interaction_mode.as_str(), SessionLifecycleState::Idle.as_str()],
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
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let agent = load_agent(&conn, &agent_id)?;
    let supports_browser = agent
        .supported_interaction_modes
        .iter()
        .any(|mode| matches!(mode, InteractionMode::Browser));

    if !supports_browser {
        return Ok(ReadinessStatus {
            ready: false,
            reason: Some(format!("{} does not support browser interaction mode.", agent.display_name)),
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
fn launch_active_workflow(state: State<'_, Mutex<RegistryStore>>) -> Result<LaunchResult, AppError> {
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
    let conn = store.connection()?;
    let workflow = get_workflow_state_from_conn(&conn)?;
    let agent_id = workflow.active_agent_id.clone().ok_or(AppError::NoActiveAgent)?;
    let mode = workflow.active_interaction_mode.clone().ok_or(AppError::NoActiveAgent)?;
    let agent = load_agent(&conn, &agent_id)?;

    set_lifecycle(&conn, SessionLifecycleState::Starting)?;

    let message = match mode {
        InteractionMode::Browser => {
            let readiness = check_browser_readiness_inner(&agent);
            if !readiness.ready {
                set_lifecycle(&conn, SessionLifecycleState::Failed)?;
                return Err(AppError::LaunchFailed(
                    readiness.reason.unwrap_or_else(|| "Browser mode is not ready.".to_string()),
                ));
            }
            set_session_message(&conn, "browser", "Browser workflow routed to Playwright adapter.")?;
            "Browser workflow routed to Playwright adapter.".to_string()
        }
        InteractionMode::NativeDesktop => {
            if !native_desktop_supported() {
                set_lifecycle(&conn, SessionLifecycleState::Failed)?;
                return Err(AppError::UnsupportedInteractionMode(
                    "native-desktop is not supported on this platform".to_string(),
                ));
            }
            launch_command_if_present(&agent)?;
            set_session_message(&conn, "native-desktop", "Native desktop workflow launch routed through Tauri adapter.")?;
            "Native desktop workflow launch routed through Tauri adapter.".to_string()
        }
        InteractionMode::Cli => {
            launch_command_if_present(&agent)?;
            set_session_message(&conn, "cli", "CLI workflow launch routed through Tauri adapter.")?;
            "CLI workflow launch routed through Tauri adapter.".to_string()
        }
    };

    set_lifecycle(&conn, SessionLifecycleState::Running)?;

    Ok(LaunchResult {
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
            Some(format!("{} does not support browser interaction mode.", agent.display_name))
        },
        requires_authentication: supports_browser,
    }
}

fn launch_command_if_present(agent: &AgentRegistryEntry) -> Result<(), AppError> {
    let Some(command) = agent.launch.command.as_deref() else {
        return Ok(());
    };

    if !command_exists(command) {
        return Err(AppError::LaunchFailed(format!("Command '{command}' was not found on PATH.")));
    }

    Command::new(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| AppError::LaunchFailed(err.to_string()))
}

#[tauri::command]
fn get_session_details(state: State<'_, Mutex<RegistryStore>>) -> Result<SessionDetails, AppError> {
    let store = state.lock().map_err(|err| AppError::Storage(err.to_string()))?;
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
    details.insert("nativeDesktopSupported".to_string(), native_desktop_supported().to_string());

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
    let store = RegistryStore::new().expect("failed to initialize registry store");

    tauri::Builder::default()
        .manage(Mutex::new(store))
        .invoke_handler(tauri::generate_handler![
            list_agents,
            get_agent_by_id,
            get_workflow_state,
            select_agent,
            check_browser_readiness,
            launch_active_workflow,
            get_session_details
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
        assert!(matches!(workflow.active_interaction_mode, Some(InteractionMode::Browser)));
        assert!(matches!(workflow.lifecycle_state, SessionLifecycleState::Running));
    }

    #[test]
    fn browser_readiness_requires_browser_support() {
        let conn = test_conn();
        let gemini = load_agent(&conn, "gemini-cli").expect("gemini");
        let opencode = load_agent(&conn, "opencode").expect("opencode");

        assert!(check_browser_readiness_inner(&gemini).ready);
        assert!(!check_browser_readiness_inner(&opencode).ready);
    }
}
