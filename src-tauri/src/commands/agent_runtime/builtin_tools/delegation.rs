use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::platform::database::NativeDatabase;
use serde_json::{json, Value};
use tauri::State;

#[tauri::command]
pub(crate) fn list_delegation_attempts(
    database: State<'_, NativeDatabase>,
    session_id: String,
) -> Result<Vec<Value>, String> {
    list_attempts(database.inner(), session_id)
}

fn list_attempts(database: &NativeDatabase, session_id: String) -> Result<Vec<Value>, String> {
    super::require_feature("delegation", "read")?;
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let mut statement = connection
        .prepare(
            "SELECT a.id, a.delegation_id, a.target, a.mode, a.status, c.base_commit, \
             a.change_set_artifact_id, COALESCE(a.started_at, d.created_at), a.completed_at \
             FROM native_tool_delegation_attempts a \
             JOIN native_tool_delegations d ON d.id = a.delegation_id \
             LEFT JOIN native_tool_change_sets c ON c.attempt_id = a.id \
             WHERE d.session_id = ?1 ORDER BY d.updated_at DESC, a.attempt_number DESC LIMIT 50",
        )
        .map_err(|_| "storage_unavailable")?;
    let attempts = statement
        .query_map([session_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "delegationId": row.get::<_, String>(1)?,
                "provider": row.get::<_, String>(2)?,
                "mode": row.get::<_, String>(3)?,
                "status": row.get::<_, String>(4)?,
                "baseCommit": row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                "changeSetArtifactId": row.get::<_, Option<String>>(6)?,
                "createdAt": row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                "completedAt": row.get::<_, Option<String>>(8)?
            }))
        })
        .map_err(|_| "storage_unavailable")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "storage_unavailable".to_owned())?;
    Ok(attempts)
}

#[tauri::command]
pub(crate) fn get_delegation_report(
    database: State<'_, NativeDatabase>,
    attempt_id: String,
) -> Result<Value, String> {
    delegation_report(database.inner(), attempt_id)
}

fn delegation_report(database: &NativeDatabase, attempt_id: String) -> Result<Value, String> {
    super::require_feature("delegation", "read")?;
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let attempt = connection
        .query_row(
            "SELECT a.id, a.delegation_id, a.target, a.mode, a.status, c.base_commit, \
             a.change_set_artifact_id, COALESCE(a.started_at, d.created_at), a.completed_at, \
             COALESCE(a.safe_summary, ''), COALESCE(c.warnings_json, '[]') \
             FROM native_tool_delegation_attempts a \
             JOIN native_tool_delegations d ON d.id = a.delegation_id \
             LEFT JOIN native_tool_change_sets c ON c.attempt_id = a.id WHERE a.id = ?1",
            [attempt_id],
            |row| {
                Ok((
                    json!({
                        "id": row.get::<_, String>(0)?, "delegationId": row.get::<_, String>(1)?,
                        "provider": row.get::<_, String>(2)?, "mode": row.get::<_, String>(3)?,
                        "status": row.get::<_, String>(4)?,
                        "baseCommit": row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                        "changeSetArtifactId": row.get::<_, Option<String>>(6)?,
                        "createdAt": row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                        "completedAt": row.get::<_, Option<String>>(8)?
                    }),
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "delegation_attempt_not_found".to_owned(),
            _ => "storage_unavailable".to_owned(),
        })?;
    let warnings = serde_json::from_str::<Value>(&attempt.2).unwrap_or_else(|_| json!([]));
    let outcome = attempt.0.get("status").cloned().unwrap_or(Value::Null);
    let host_evidence = change_set_paths(&connection, &attempt.0)?;
    Ok(json!({
        "attempt": attempt.0,
        "outcome": outcome,
        "summary": attempt.1,
        "hostEvidence": host_evidence,
        "providerClaims": [],
        "warnings": warnings
    }))
}

#[tauri::command]
pub(crate) fn get_delegation_recovery(
    database: State<'_, NativeDatabase>,
    operation_id: String,
) -> Result<Value, String> {
    delegation_recovery(database.inner(), operation_id)
}

fn delegation_recovery(database: &NativeDatabase, operation_id: String) -> Result<Value, String> {
    super::require_feature("delegation", "read")?;
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    connection.query_row(
        "SELECT a.status, r.status, r.recovery_reference FROM native_tool_apply_attempts a \
         LEFT JOIN native_tool_apply_recovery r ON r.apply_attempt_id = a.id WHERE a.id = ?1",
        [&operation_id],
        |row| {
            let apply_status = row.get::<_, String>(0)?;
            let recovery_status = row.get::<_, Option<String>>(1)?;
            let state = if apply_status == "succeeded" { "safely_completed" }
                else if recovery_status.as_deref() == Some("rolled_back") { "rolled_back" }
                else { "manual_recovery" };
            Ok(json!({"operationId": operation_id, "state": state, "capsuleReference": row.get::<_, Option<String>>(2)?}))
        },
    ).map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => "apply_operation_not_found".to_owned(),
        _ => "storage_unavailable".to_owned(),
    })
}

#[tauri::command]
pub(crate) fn get_browser_handoff(
    api: State<'_, AgentRuntimeApi>,
    operation_id: String,
) -> Result<Value, String> {
    browser_handoff(api.inner(), operation_id)
}

fn browser_handoff(api: &AgentRuntimeApi, operation_id: String) -> Result<Value, String> {
    super::require_feature("browser", "execute")?;
    api.get_browser_handoff(&operation_id)
        .map_err(|_| "browser_handoff_unavailable".to_owned())
}

#[tauri::command]
pub(crate) fn begin_browser_handoff(
    api: State<'_, AgentRuntimeApi>,
    operation_id: String,
) -> Result<Value, String> {
    start_browser_handoff(api.inner(), operation_id)
}

fn start_browser_handoff(api: &AgentRuntimeApi, operation_id: String) -> Result<Value, String> {
    super::require_feature("browser", "execute")?;
    api.begin_browser_handoff(&operation_id)
        .map_err(|_| "browser_handoff_stale".to_owned())
}

#[tauri::command]
pub(crate) fn resume_browser_automation(
    api: State<'_, AgentRuntimeApi>,
    operation_id: String,
    ownership_token: String,
) -> Result<Value, String> {
    resume_browser(api.inner(), operation_id, ownership_token)
}

fn resume_browser(
    api: &AgentRuntimeApi,
    operation_id: String,
    ownership_token: String,
) -> Result<Value, String> {
    super::require_feature("browser", "execute")?;
    api.resume_browser_automation(&operation_id, &ownership_token)
        .map_err(|_| "browser_handoff_stale".to_owned())
}

fn change_set_paths(
    connection: &rusqlite::Connection,
    attempt: &Value,
) -> Result<Vec<String>, String> {
    let Some(artifact_id) = attempt.get("changeSetArtifactId").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare("SELECT path FROM native_tool_change_set_files WHERE change_set_artifact_id = ?1 ORDER BY ordinal")
        .map_err(|_| "storage_unavailable")?;
    let paths = statement
        .query_map([artifact_id], |row| row.get::<_, String>(0))
        .map_err(|_| "storage_unavailable")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "storage_unavailable".to_owned())?;
    Ok(paths)
}
