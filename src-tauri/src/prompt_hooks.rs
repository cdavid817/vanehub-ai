use crate::{AppError, RegistryStore};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

const MANAGED_CLI_AGENT_IDS: [&str; 4] = ["claude-code", "codex-cli", "gemini-cli", "opencode"];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptHookCategory {
    Bootstrap,
    Callback,
    Dynamic,
    Law,
    Navigation,
    Routing,
    Static,
}

impl PromptHookCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Callback => "callback",
            Self::Dynamic => "dynamic",
            Self::Law => "law",
            Self::Navigation => "navigation",
            Self::Routing => "routing",
            Self::Static => "static",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptHookStage {
    SessionInit,
    PerTurn,
}

impl PromptHookStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::SessionInit => "session-init",
            Self::PerTurn => "per-turn",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptHookSource {
    Builtin,
    User,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookGovernance {
    safety_tier: String,
    transparency_tier: String,
    governance_tier: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHook {
    id: String,
    name: String,
    description: String,
    category: PromptHookCategory,
    stage: PromptHookStage,
    order: i64,
    version: i64,
    source: PromptHookSource,
    enabled: bool,
    disableable: bool,
    cli_bindings: Vec<String>,
    governance: PromptHookGovernance,
    template_body: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookListResult {
    hooks: Vec<PromptHook>,
    stats: PromptHookStats,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookStats {
    total: usize,
    enabled: usize,
    builtin: usize,
    user: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookMutationInput {
    id: String,
    name: String,
    description: String,
    category: PromptHookCategory,
    stage: PromptHookStage,
    order: i64,
    template_body: String,
    enabled: bool,
    cli_bindings: Vec<String>,
    governance: PromptHookGovernance,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookUpdateInput {
    id: String,
    name: String,
    description: String,
    category: PromptHookCategory,
    stage: PromptHookStage,
    order: i64,
    version: i64,
    template_body: String,
    enabled: bool,
    cli_bindings: Vec<String>,
    governance: PromptHookGovernance,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookPreviewInput {
    hook_id: String,
    agent_id: String,
    sample_input: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptAssemblyPreviewInput {
    agent_id: String,
    sample_input: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookTraceSummary {
    pub(crate) id: String,
    pub(crate) hook_id: String,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) status: String,
    pub(crate) version: Option<i64>,
    pub(crate) content_hash: Option<String>,
    pub(crate) token_estimate: Option<i64>,
    pub(crate) reason: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookPreview {
    hook_id: Option<String>,
    agent_id: String,
    rendered_content: String,
    trace: Vec<PromptHookTraceSummary>,
}

#[derive(Debug)]
pub(crate) struct PromptAssemblyResult {
    pub(crate) effective_prompt: String,
    pub(crate) trace: Vec<PromptHookTraceSummary>,
}

struct BuiltinPromptHook {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: PromptHookCategory,
    stage: PromptHookStage,
    order: i64,
    enabled: bool,
    disableable: bool,
    template_body: &'static str,
}

const BUILTIN_PROMPT_HOOKS: [BuiltinPromptHook; 7] = [
    BuiltinPromptHook {
        id: "bootstrap-session-context",
        name: "Session Context",
        description: "Adds session and workspace context to each CLI prompt.",
        category: PromptHookCategory::Bootstrap,
        stage: PromptHookStage::SessionInit,
        order: 100,
        enabled: true,
        disableable: true,
        template_body: "Session context: {{sampleInput}}",
    },
    BuiltinPromptHook {
        id: "law-runtime-boundary",
        name: "Runtime Boundary",
        description: "Keeps CLI behavior inside VaneHub runtime and permission boundaries.",
        category: PromptHookCategory::Law,
        stage: PromptHookStage::SessionInit,
        order: 200,
        enabled: true,
        disableable: false,
        template_body: "Respect the active VaneHub runtime, permissions, and project boundaries.",
    },
    BuiltinPromptHook {
        id: "static-response-format",
        name: "Response Format",
        description: "Sets a concise engineering response baseline.",
        category: PromptHookCategory::Static,
        stage: PromptHookStage::SessionInit,
        order: 300,
        enabled: true,
        disableable: true,
        template_body: "Use direct, actionable engineering responses with concise verification notes.",
    },
    BuiltinPromptHook {
        id: "dynamic-session-config",
        name: "Session Configuration",
        description: "Summarizes active session configuration for the selected CLI.",
        category: PromptHookCategory::Dynamic,
        stage: PromptHookStage::PerTurn,
        order: 400,
        enabled: true,
        disableable: true,
        template_body: "Active CLI: {{agentId}}. User request follows after the hook context.",
    },
    BuiltinPromptHook {
        id: "navigation-project-hints",
        name: "Project Navigation",
        description: "Encourages grounded project inspection before code changes.",
        category: PromptHookCategory::Navigation,
        stage: PromptHookStage::PerTurn,
        order: 500,
        enabled: true,
        disableable: true,
        template_body: "Inspect relevant project files and existing patterns before making changes.",
    },
    BuiltinPromptHook {
        id: "routing-cli-capabilities",
        name: "CLI Capability Routing",
        description: "Keeps behavior aligned with the selected CLI agent capabilities.",
        category: PromptHookCategory::Routing,
        stage: PromptHookStage::PerTurn,
        order: 600,
        enabled: true,
        disableable: true,
        template_body: "Route work through capabilities available to {{agentId}}.",
    },
    BuiltinPromptHook {
        id: "callback-future-channel",
        name: "Callback Channel Placeholder",
        description: "Reserved placeholder for future callback-aware workflows.",
        category: PromptHookCategory::Callback,
        stage: PromptHookStage::PerTurn,
        order: 700,
        enabled: false,
        disableable: true,
        template_body: "Callback channel support is not active in this runtime.",
    },
];

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS prompt_hook_overrides (
            hook_id TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL,
            cli_bindings TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS prompt_hooks_user (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            category TEXT NOT NULL,
            stage TEXT NOT NULL,
            hook_order INTEGER NOT NULL,
            version INTEGER NOT NULL,
            enabled INTEGER NOT NULL,
            disableable INTEGER NOT NULL,
            cli_bindings TEXT NOT NULL,
            governance TEXT NOT NULL,
            template_body TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS prompt_hook_traces (
            id TEXT PRIMARY KEY,
            hook_id TEXT NOT NULL,
            category TEXT NOT NULL,
            stage TEXT NOT NULL,
            status TEXT NOT NULL,
            version INTEGER,
            content_hash TEXT,
            token_estimate INTEGER,
            reason TEXT,
            agent_id TEXT,
            session_id TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_prompt_hook_traces_created
            ON prompt_hook_traces(created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn list_prompt_hooks_for_conn(conn: &Connection) -> Result<PromptHookListResult, AppError> {
    let hooks = list_effective_hooks(conn)?;
    Ok(PromptHookListResult {
        stats: stats_for_hooks(&hooks),
        hooks,
    })
}

#[tauri::command]
pub(crate) fn list_prompt_hooks(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<PromptHookListResult, AppError> {
    let conn = connection_from_state(state)?;
    list_prompt_hooks_for_conn(&conn)
}

#[tauri::command]
pub(crate) fn create_prompt_hook(
    state: State<'_, Mutex<RegistryStore>>,
    input: PromptHookMutationInput,
) -> Result<PromptHook, AppError> {
    let mut conn = connection_from_state(state)?;
    create_user_hook(&mut conn, input)
}

#[tauri::command]
pub(crate) fn update_prompt_hook(
    state: State<'_, Mutex<RegistryStore>>,
    hook_id: String,
    input: PromptHookUpdateInput,
) -> Result<PromptHook, AppError> {
    let mut conn = connection_from_state(state)?;
    update_user_hook(&mut conn, &hook_id, input)
}

#[tauri::command]
pub(crate) fn delete_prompt_hook(
    state: State<'_, Mutex<RegistryStore>>,
    hook_id: String,
) -> Result<(), AppError> {
    let mut conn = connection_from_state(state)?;
    delete_user_hook(&mut conn, &hook_id)
}

#[tauri::command]
pub(crate) fn set_prompt_hook_enabled(
    state: State<'_, Mutex<RegistryStore>>,
    hook_id: String,
    enabled: bool,
) -> Result<PromptHook, AppError> {
    let mut conn = connection_from_state(state)?;
    set_enabled(&mut conn, &hook_id, enabled)
}

#[tauri::command]
pub(crate) fn set_prompt_hook_cli_bindings(
    state: State<'_, Mutex<RegistryStore>>,
    hook_id: String,
    agent_ids: Vec<String>,
) -> Result<PromptHook, AppError> {
    let mut conn = connection_from_state(state)?;
    set_cli_bindings(&mut conn, &hook_id, agent_ids)
}

#[tauri::command]
pub(crate) fn preview_prompt_hook(
    state: State<'_, Mutex<RegistryStore>>,
    input: PromptHookPreviewInput,
) -> Result<PromptHookPreview, AppError> {
    let conn = connection_from_state(state)?;
    preview_hook(&conn, input)
}

#[tauri::command]
pub(crate) fn preview_prompt_assembly(
    state: State<'_, Mutex<RegistryStore>>,
    input: PromptAssemblyPreviewInput,
) -> Result<PromptHookPreview, AppError> {
    let conn = connection_from_state(state)?;
    let result = assemble_prompt(&conn, &input.agent_id, None, &input.sample_input)?;
    Ok(PromptHookPreview {
        hook_id: None,
        agent_id: input.agent_id,
        rendered_content: result.effective_prompt,
        trace: result.trace,
    })
}

#[tauri::command]
pub(crate) fn list_prompt_hook_traces(
    state: State<'_, Mutex<RegistryStore>>,
    limit: Option<i64>,
) -> Result<Vec<PromptHookTraceSummary>, AppError> {
    let conn = connection_from_state(state)?;
    list_traces(&conn, limit.unwrap_or(25))
}

pub(crate) fn assemble_prompt(
    conn: &Connection,
    agent_id: &str,
    session_id: Option<&str>,
    user_prompt: &str,
) -> Result<PromptAssemblyResult, AppError> {
    validate_agent_id(agent_id)?;
    let mut rendered_parts = Vec::new();
    let mut traces = Vec::new();
    for hook in list_effective_hooks(conn)? {
        if !hook.enabled {
            traces.push(trace_for_hook(&hook, "disabled", None, agent_id, session_id, Some("disabled")));
            continue;
        }
        if !hook.cli_bindings.iter().any(|binding| binding == agent_id) {
            traces.push(trace_for_hook(&hook, "skipped", None, agent_id, session_id, Some("unbound-cli")));
            continue;
        }
        let content = render_template(hook.template_body.as_deref().unwrap_or_default(), agent_id, user_prompt);
        traces.push(trace_for_hook(&hook, "fired", Some(&content), agent_id, session_id, None));
        rendered_parts.push(content);
    }
    let effective_prompt = rendered_parts
        .into_iter()
        .chain(std::iter::once(user_prompt.to_string()))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    persist_traces(conn, &traces)?;
    Ok(PromptAssemblyResult {
        effective_prompt,
        trace: traces,
    })
}

fn connection_from_state(state: State<'_, Mutex<RegistryStore>>) -> Result<Connection, AppError> {
    let store = state
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?;
    store.connection()
}

fn default_governance(disableable: bool) -> PromptHookGovernance {
    PromptHookGovernance {
        safety_tier: "readonly".to_string(),
        transparency_tier: if disableable {
            "opt-in-view".to_string()
        } else {
            "visible-by-default".to_string()
        },
        governance_tier: if disableable {
            "human-gated".to_string()
        } else {
            "immutable".to_string()
        },
    }
}

fn builtin_to_hook(seed: &BuiltinPromptHook) -> PromptHook {
    let timestamp = "2026-07-18T00:00:00Z".to_string();
    PromptHook {
        id: seed.id.to_string(),
        name: seed.name.to_string(),
        description: seed.description.to_string(),
        category: seed.category,
        stage: seed.stage,
        order: seed.order,
        version: 1,
        source: PromptHookSource::Builtin,
        enabled: seed.enabled,
        disableable: seed.disableable,
        cli_bindings: MANAGED_CLI_AGENT_IDS.iter().map(|id| (*id).to_string()).collect(),
        governance: default_governance(seed.disableable),
        template_body: Some(seed.template_body.to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    }
}

fn list_effective_hooks(conn: &Connection) -> Result<Vec<PromptHook>, AppError> {
    let overrides = load_overrides(conn)?;
    let mut hooks = BUILTIN_PROMPT_HOOKS
        .iter()
        .map(builtin_to_hook)
        .map(|hook| overrides.get(&hook.id).map_or(hook.clone(), |override_row| apply_override(hook, override_row)))
        .collect::<Vec<_>>();
    hooks.extend(load_user_hooks(conn)?);
    hooks.sort_by(|left, right| {
        left.stage
            .as_str()
            .cmp(right.stage.as_str())
            .then(left.category.as_str().cmp(right.category.as_str()))
            .then(left.order.cmp(&right.order))
            .then(left.id.cmp(&right.id))
    });
    Ok(hooks)
}

struct HookOverride {
    enabled: bool,
    cli_bindings: Vec<String>,
    updated_at: String,
}

fn apply_override(mut hook: PromptHook, override_row: &HookOverride) -> PromptHook {
    hook.enabled = override_row.enabled;
    hook.cli_bindings = override_row.cli_bindings.clone();
    hook.updated_at.clone_from(&override_row.updated_at);
    hook
}

fn load_overrides(conn: &Connection) -> Result<HashMap<String, HookOverride>, AppError> {
    let mut stmt = conn.prepare("SELECT hook_id, enabled, cli_bindings, updated_at FROM prompt_hook_overrides")?;
    let rows = stmt.query_map([], |row| {
        let bindings_json: String = row.get(2)?;
        let cli_bindings = serde_json::from_str(&bindings_json).unwrap_or_default();
        Ok((
            row.get::<_, String>(0)?,
            HookOverride {
                enabled: row.get::<_, i64>(1)? != 0,
                cli_bindings,
                updated_at: row.get(3)?,
            },
        ))
    })?;
    let mut overrides = HashMap::new();
    for row in rows {
        let (id, override_row) = row?;
        overrides.insert(id, override_row);
    }
    Ok(overrides)
}

fn load_user_hooks(conn: &Connection) -> Result<Vec<PromptHook>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, category, stage, hook_order, version, enabled, disableable, cli_bindings, governance, template_body, created_at, updated_at FROM prompt_hooks_user",
    )?;
    let rows = stmt.query_map([], |row| {
        let category: String = row.get(3)?;
        let stage: String = row.get(4)?;
        let bindings_json: String = row.get(9)?;
        let governance_json: String = row.get(10)?;
        Ok(PromptHook {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            category: parse_category(&category).unwrap_or(PromptHookCategory::Dynamic),
            stage: parse_stage(&stage).unwrap_or(PromptHookStage::PerTurn),
            order: row.get(5)?,
            version: row.get(6)?,
            source: PromptHookSource::User,
            enabled: row.get::<_, i64>(7)? != 0,
            disableable: row.get::<_, i64>(8)? != 0,
            cli_bindings: serde_json::from_str(&bindings_json).unwrap_or_default(),
            governance: serde_json::from_str(&governance_json).unwrap_or_else(|_| default_governance(true)),
            template_body: Some(row.get(11)?),
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

fn stats_for_hooks(hooks: &[PromptHook]) -> PromptHookStats {
    PromptHookStats {
        total: hooks.len(),
        enabled: hooks.iter().filter(|hook| hook.enabled).count(),
        builtin: hooks
            .iter()
            .filter(|hook| hook.source == PromptHookSource::Builtin)
            .count(),
        user: hooks
            .iter()
            .filter(|hook| hook.source == PromptHookSource::User)
            .count(),
    }
}

fn create_user_hook(conn: &mut Connection, input: PromptHookMutationInput) -> Result<PromptHook, AppError> {
    validate_mutation(&input)?;
    if find_hook(conn, &input.id)?.is_some() {
        return Err(AppError::Validation(format!(
            "Prompt Hook already exists: {}",
            input.id
        )));
    }
    let tx = conn.transaction()?;
    let now = Utc::now().to_rfc3339();
    let hook = PromptHook {
        id: input.id,
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        category: input.category,
        stage: input.stage,
        order: input.order,
        version: 1,
        source: PromptHookSource::User,
        enabled: input.enabled,
        disableable: true,
        cli_bindings: input.cli_bindings,
        governance: input.governance,
        template_body: Some(input.template_body),
        created_at: now.clone(),
        updated_at: now,
    };
    insert_user_hook(&tx, &hook)?;
    tx.commit()?;
    Ok(hook)
}

fn update_user_hook(
    conn: &mut Connection,
    hook_id: &str,
    input: PromptHookUpdateInput,
) -> Result<PromptHook, AppError> {
    if hook_id != input.id {
        return Err(AppError::Validation(
            "Prompt Hook id cannot be changed.".to_string(),
        ));
    }
    let Some(current) = find_hook(conn, hook_id)? else {
        return Err(AppError::Validation(format!("Prompt Hook not found: {hook_id}")));
    };
    if current.source == PromptHookSource::Builtin {
        return Err(AppError::Validation(
            "Built-in Prompt Hook content cannot be edited.".to_string(),
        ));
    }
    validate_update(&input)?;
    let tx = conn.transaction()?;
    let hook = PromptHook {
        id: input.id,
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        category: input.category,
        stage: input.stage,
        order: input.order,
        version: input.version,
        source: PromptHookSource::User,
        enabled: input.enabled,
        disableable: true,
        cli_bindings: input.cli_bindings,
        governance: input.governance,
        template_body: Some(input.template_body),
        created_at: current.created_at,
        updated_at: Utc::now().to_rfc3339(),
    };
    insert_user_hook(&tx, &hook)?;
    tx.commit()?;
    Ok(hook)
}

fn delete_user_hook(conn: &mut Connection, hook_id: &str) -> Result<(), AppError> {
    let Some(current) = find_hook(conn, hook_id)? else {
        return Err(AppError::Validation(format!("Prompt Hook not found: {hook_id}")));
    };
    if current.source == PromptHookSource::Builtin {
        return Err(AppError::Validation(
            "Built-in Prompt Hook cannot be deleted.".to_string(),
        ));
    }
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM prompt_hooks_user WHERE id = ?1", params![hook_id])?;
    tx.execute(
        "DELETE FROM prompt_hook_overrides WHERE hook_id = ?1",
        params![hook_id],
    )?;
    tx.commit()?;
    Ok(())
}

fn set_enabled(conn: &mut Connection, hook_id: &str, enabled: bool) -> Result<PromptHook, AppError> {
    let Some(current) = find_hook(conn, hook_id)? else {
        return Err(AppError::Validation(format!("Prompt Hook not found: {hook_id}")));
    };
    if !enabled && !current.disableable {
        return Err(AppError::Validation(
            "Prompt Hook cannot be disabled.".to_string(),
        ));
    }
    if current.source == PromptHookSource::User {
        conn.execute(
            "UPDATE prompt_hooks_user SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![bool_to_i64(enabled), Utc::now().to_rfc3339(), hook_id],
        )?;
    } else {
        upsert_override(conn, hook_id, enabled, &current.cli_bindings)?;
    }
    find_hook(conn, hook_id)?.ok_or_else(|| AppError::Validation(format!("Prompt Hook not found: {hook_id}")))
}

fn set_cli_bindings(
    conn: &mut Connection,
    hook_id: &str,
    agent_ids: Vec<String>,
) -> Result<PromptHook, AppError> {
    validate_bindings(&agent_ids)?;
    let Some(current) = find_hook(conn, hook_id)? else {
        return Err(AppError::Validation(format!("Prompt Hook not found: {hook_id}")));
    };
    let cli_bindings = dedupe_bindings(agent_ids);
    if current.source == PromptHookSource::User {
        conn.execute(
            "UPDATE prompt_hooks_user SET cli_bindings = ?1, updated_at = ?2 WHERE id = ?3",
            params![json_string(&cli_bindings)?, Utc::now().to_rfc3339(), hook_id],
        )?;
    } else {
        upsert_override(conn, hook_id, current.enabled, &cli_bindings)?;
    }
    find_hook(conn, hook_id)?.ok_or_else(|| AppError::Validation(format!("Prompt Hook not found: {hook_id}")))
}

fn upsert_override(
    conn: &Connection,
    hook_id: &str,
    enabled: bool,
    cli_bindings: &[String],
) -> Result<(), AppError> {
    conn.execute(
        r#"
        INSERT INTO prompt_hook_overrides (hook_id, enabled, cli_bindings, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(hook_id) DO UPDATE SET
            enabled = excluded.enabled,
            cli_bindings = excluded.cli_bindings,
            updated_at = excluded.updated_at
        "#,
        params![
            hook_id,
            bool_to_i64(enabled),
            json_string(cli_bindings)?,
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

fn insert_user_hook(conn: &Connection, hook: &PromptHook) -> Result<(), AppError> {
    conn.execute(
        r#"
        INSERT INTO prompt_hooks_user (
            id, name, description, category, stage, hook_order, version, enabled, disableable,
            cli_bindings, governance, template_body, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            category = excluded.category,
            stage = excluded.stage,
            hook_order = excluded.hook_order,
            version = excluded.version,
            enabled = excluded.enabled,
            disableable = excluded.disableable,
            cli_bindings = excluded.cli_bindings,
            governance = excluded.governance,
            template_body = excluded.template_body,
            updated_at = excluded.updated_at
        "#,
        params![
            hook.id,
            hook.name,
            hook.description,
            hook.category.as_str(),
            hook.stage.as_str(),
            hook.order,
            hook.version,
            bool_to_i64(hook.enabled),
            bool_to_i64(hook.disableable),
            json_string(&hook.cli_bindings)?,
            json_string(&hook.governance)?,
            hook.template_body.as_deref().unwrap_or_default(),
            hook.created_at,
            hook.updated_at,
        ],
    )?;
    Ok(())
}

fn preview_hook(conn: &Connection, input: PromptHookPreviewInput) -> Result<PromptHookPreview, AppError> {
    validate_agent_id(&input.agent_id)?;
    let Some(hook) = find_hook(conn, &input.hook_id)? else {
        return Err(AppError::Validation(format!(
            "Prompt Hook not found: {}",
            input.hook_id
        )));
    };
    let sample_input = input
        .sample_input
        .unwrap_or_else(|| "Preview request".to_string());
    let rendered_content = render_template(
        hook.template_body.as_deref().unwrap_or_default(),
        &input.agent_id,
        &sample_input,
    );
    let trace = vec![trace_for_hook(
        &hook,
        if hook.enabled { "fired" } else { "disabled" },
        hook.enabled.then_some(rendered_content.as_str()),
        &input.agent_id,
        None,
        (!hook.enabled).then_some("disabled"),
    )];
    persist_traces(conn, &trace)?;
    Ok(PromptHookPreview {
        hook_id: Some(hook.id),
        agent_id: input.agent_id,
        rendered_content,
        trace,
    })
}

fn find_hook(conn: &Connection, hook_id: &str) -> Result<Option<PromptHook>, AppError> {
    Ok(list_effective_hooks(conn)?
        .into_iter()
        .find(|hook| hook.id == hook_id))
}

fn validate_mutation(input: &PromptHookMutationInput) -> Result<(), AppError> {
    validate_identity(&input.id, &input.name, input.order, &input.template_body)?;
    validate_bindings(&input.cli_bindings)
}

fn validate_update(input: &PromptHookUpdateInput) -> Result<(), AppError> {
    validate_identity(&input.id, &input.name, input.order, &input.template_body)?;
    validate_bindings(&input.cli_bindings)
}

fn validate_identity(id: &str, name: &str, order: i64, body: &str) -> Result<(), AppError> {
    if id.len() < 3
        || id.len() > 64
        || !id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        || !id
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    {
        return Err(AppError::Validation("Invalid Prompt Hook id.".to_string()));
    }
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "Prompt Hook name is required.".to_string(),
        ));
    }
    if order < 0 {
        return Err(AppError::Validation(
            "Prompt Hook order must be non-negative.".to_string(),
        ));
    }
    if body
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t')
    {
        return Err(AppError::Validation(
            "Prompt Hook content contains unsupported control characters.".to_string(),
        ));
    }
    Ok(())
}

fn validate_bindings(bindings: &[String]) -> Result<(), AppError> {
    for binding in bindings {
        validate_agent_id(binding)?;
    }
    Ok(())
}

fn validate_agent_id(agent_id: &str) -> Result<(), AppError> {
    if MANAGED_CLI_AGENT_IDS.contains(&agent_id) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Unsupported Prompt Hook CLI binding: {agent_id}"
        )))
    }
}

fn dedupe_bindings(bindings: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    bindings
        .into_iter()
        .filter(|binding| seen.insert(binding.clone()))
        .collect()
}

fn render_template(template: &str, agent_id: &str, sample_input: &str) -> String {
    template
        .replace("{{agentId}}", agent_id)
        .replace("{{sampleInput}}", sample_input)
}

fn trace_for_hook(
    hook: &PromptHook,
    status: &str,
    content: Option<&str>,
    agent_id: &str,
    session_id: Option<&str>,
    reason: Option<&str>,
) -> PromptHookTraceSummary {
    PromptHookTraceSummary {
        id: format!("prompt-hook-trace-{}", Uuid::new_v4()),
        hook_id: hook.id.clone(),
        category: hook.category,
        stage: hook.stage,
        status: status.to_string(),
        version: (status == "fired").then_some(hook.version),
        content_hash: content.map(hash_content),
        token_estimate: content.map(|value| value.chars().count().div_ceil(4) as i64),
        reason: reason.map(str::to_string),
        agent_id: Some(agent_id.to_string()),
        session_id: session_id.map(str::to_string),
        created_at: Utc::now().to_rfc3339(),
    }
}

fn persist_traces(conn: &Connection, traces: &[PromptHookTraceSummary]) -> Result<(), AppError> {
    for trace in traces {
        conn.execute(
            r#"
            INSERT INTO prompt_hook_traces (
                id, hook_id, category, stage, status, version, content_hash, token_estimate,
                reason, agent_id, session_id, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                trace.id,
                trace.hook_id,
                trace.category.as_str(),
                trace.stage.as_str(),
                trace.status,
                trace.version,
                trace.content_hash,
                trace.token_estimate,
                trace.reason,
                trace.agent_id,
                trace.session_id,
                trace.created_at,
            ],
        )?;
    }
    conn.execute(
        "DELETE FROM prompt_hook_traces WHERE id NOT IN (SELECT id FROM prompt_hook_traces ORDER BY created_at DESC LIMIT 50)",
        [],
    )?;
    Ok(())
}

fn list_traces(conn: &Connection, limit: i64) -> Result<Vec<PromptHookTraceSummary>, AppError> {
    let bounded_limit = limit.clamp(1, 100);
    let mut stmt = conn.prepare(
        "SELECT id, hook_id, category, stage, status, version, content_hash, token_estimate, reason, agent_id, session_id, created_at FROM prompt_hook_traces ORDER BY created_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![bounded_limit], |row| {
        let category: String = row.get(2)?;
        let stage: String = row.get(3)?;
        Ok(PromptHookTraceSummary {
            id: row.get(0)?,
            hook_id: row.get(1)?,
            category: parse_category(&category).unwrap_or(PromptHookCategory::Dynamic),
            stage: parse_stage(&stage).unwrap_or(PromptHookStage::PerTurn),
            status: row.get(4)?,
            version: row.get(5)?,
            content_hash: row.get(6)?,
            token_estimate: row.get(7)?,
            reason: row.get(8)?,
            agent_id: row.get(9)?,
            session_id: row.get(10)?,
            created_at: row.get(11)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    format!("{digest:x}").chars().take(16).collect()
}

fn json_string<T: Serialize + ?Sized>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|err| AppError::Storage(err.to_string()))
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn parse_category(value: &str) -> Option<PromptHookCategory> {
    match value {
        "bootstrap" => Some(PromptHookCategory::Bootstrap),
        "callback" => Some(PromptHookCategory::Callback),
        "dynamic" => Some(PromptHookCategory::Dynamic),
        "law" => Some(PromptHookCategory::Law),
        "navigation" => Some(PromptHookCategory::Navigation),
        "routing" => Some(PromptHookCategory::Routing),
        "static" => Some(PromptHookCategory::Static),
        _ => None,
    }
}

fn parse_stage(value: &str) -> Option<PromptHookStage> {
    match value {
        "session-init" => Some(PromptHookStage::SessionInit),
        "per-turn" => Some(PromptHookStage::PerTurn),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("memory db");
        apply_schema(&conn).expect("schema");
        conn
    }

    fn user_input(id: &str) -> PromptHookMutationInput {
        PromptHookMutationInput {
            id: id.to_string(),
            name: "User Hook".to_string(),
            description: "User hook description".to_string(),
            category: PromptHookCategory::Dynamic,
            stage: PromptHookStage::PerTurn,
            order: 450,
            template_body: "Custom {{agentId}} {{sampleInput}}".to_string(),
            enabled: true,
            cli_bindings: vec!["codex-cli".to_string()],
            governance: PromptHookGovernance {
                safety_tier: "editable".to_string(),
                transparency_tier: "visible-by-default".to_string(),
                governance_tier: "human-gated".to_string(),
            },
        }
    }

    #[test]
    fn catalog_covers_all_categories() {
        let conn = conn();
        let result = list_prompt_hooks_for_conn(&conn).expect("hooks");
        let categories = result
            .hooks
            .iter()
            .map(|hook| hook.category.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(result.stats.builtin, 7);
        for category in [
            "bootstrap",
            "callback",
            "dynamic",
            "law",
            "navigation",
            "routing",
            "static",
        ] {
            assert!(categories.contains(category));
        }
    }

    #[test]
    fn immutable_builtin_cannot_be_disabled() {
        let mut conn = conn();
        let err = set_enabled(&mut conn, "law-runtime-boundary", false).expect_err("immutable");
        assert!(err.to_string().contains("cannot be disabled"));
    }

    #[test]
    fn user_hook_crud_and_binding_validation() {
        let mut conn = conn();
        let created = create_user_hook(&mut conn, user_input("user-review-focus")).expect("create");
        assert_eq!(created.source, PromptHookSource::User);
        assert!(set_cli_bindings(&mut conn, &created.id, vec!["unknown".to_string()]).is_err());
        let rebound = set_cli_bindings(&mut conn, &created.id, vec!["codex-cli".to_string(), "codex-cli".to_string()])
            .expect("bind");
        assert_eq!(rebound.cli_bindings, vec!["codex-cli"]);
        delete_user_hook(&mut conn, &created.id).expect("delete");
        assert!(find_hook(&conn, &created.id).expect("find").is_none());
    }

    #[test]
    fn pipeline_skips_unbound_and_preserves_user_prompt() {
        let mut conn = conn();
        set_cli_bindings(
            &mut conn,
            "navigation-project-hints",
            vec!["codex-cli".to_string()],
        )
        .expect("bind");
        let result = assemble_prompt(&conn, "gemini-cli", Some("session-1"), "hello").expect("assemble");
        assert!(result.effective_prompt.ends_with("hello"));
        assert!(result
            .trace
            .iter()
            .any(|trace| trace.hook_id == "navigation-project-hints" && trace.status == "skipped"));
        assert!(result.trace.iter().all(|trace| trace.content_hash.as_deref() != Some("hello")));
    }

    #[test]
    fn pipeline_accepts_four_stable_cli_ids_and_rejects_unknown_agent() {
        let conn = conn();

        for agent_id in ["claude-code", "codex-cli", "gemini-cli", "opencode"] {
            let result = assemble_prompt(&conn, agent_id, Some("session-1"), "hello")
                .expect("stable cli agent should assemble");
            assert!(result.effective_prompt.ends_with("hello"));
            assert!(result.trace.iter().any(|trace| trace.agent_id.as_deref() == Some(agent_id)));
        }

        assert!(assemble_prompt(&conn, "browser-agent", Some("session-1"), "hello").is_err());
    }

    #[test]
    fn preview_returns_content_and_safe_trace() {
        let conn = conn();
        let preview = preview_hook(
            &conn,
            PromptHookPreviewInput {
                hook_id: "dynamic-session-config".to_string(),
                agent_id: "codex-cli".to_string(),
                sample_input: Some("ship it".to_string()),
            },
        )
        .expect("preview");
        assert!(preview.rendered_content.contains("codex-cli"));
        assert_eq!(preview.trace[0].status, "fired");
        assert!(preview.trace[0].content_hash.is_some());
    }
}
