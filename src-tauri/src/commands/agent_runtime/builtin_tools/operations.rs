use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListOperationsInput {
    session_id: String,
    capability: Option<String>,
    limit: Option<u32>,
}

#[tauri::command]
pub(crate) fn get_builtin_tool_operation(
    database: State<'_, NativeDatabase>,
    operation_id: String,
) -> Result<Value, String> {
    find_operation(database.inner(), operation_id)
}

fn find_operation(database: &NativeDatabase, operation_id: String) -> Result<Value, String> {
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    connection
        .query_row(
            "SELECT id, session_id, generation_id, tool_name, status, progress_sequence, \
             progress_message, result_artifact_ids_json, error_code, created_at, updated_at \
             FROM native_tool_operations WHERE id = ?1",
            [operation_id],
            operation_from_row,
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "operation_not_found".to_owned(),
            _ => "storage_unavailable".to_owned(),
        })
}

#[tauri::command]
pub(crate) fn list_builtin_tool_operations(
    database: State<'_, NativeDatabase>,
    input: ListOperationsInput,
) -> Result<Vec<Value>, String> {
    list_operations(database.inner(), input)
}

fn list_operations(
    database: &NativeDatabase,
    input: ListOperationsInput,
) -> Result<Vec<Value>, String> {
    let limit = input.limit.unwrap_or(50);
    if input.session_id.trim().is_empty() || !(1..=100).contains(&limit) {
        return Err("invalid_input".to_owned());
    }
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let mut statement = connection
        .prepare(
            "SELECT id, session_id, generation_id, tool_name, status, progress_sequence, \
             progress_message, result_artifact_ids_json, error_code, created_at, updated_at \
             FROM native_tool_operations WHERE session_id = ?1 \
             ORDER BY updated_at DESC, id DESC LIMIT ?2",
        )
        .map_err(|_| "storage_unavailable")?;
    let rows = statement
        .query_map(params![input.session_id, limit], operation_from_row)
        .map_err(|_| "storage_unavailable")?;
    let mut operations = Vec::new();
    for row in rows {
        let operation = row.map_err(|_| "storage_unavailable")?;
        if input.capability.as_ref().is_none_or(|capability| {
            operation.get("capability").and_then(Value::as_str) == Some(capability)
        }) {
            operations.push(operation);
        }
    }
    Ok(operations)
}

#[tauri::command]
pub(crate) fn cancel_builtin_tool_operation(
    api: State<'_, AgentRuntimeApi>,
    database: State<'_, NativeDatabase>,
    operation_id: String,
) -> Result<Value, String> {
    let _ = api.cancel_manual_native_tool(&operation_id);
    cancel_operation(database.inner(), operation_id)
}

fn cancel_operation(database: &NativeDatabase, operation_id: String) -> Result<Value, String> {
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let updated = connection
        .execute(
            "UPDATE native_tool_operations SET status = 'cancelled', updated_at = ?2 \
             WHERE id = ?1 AND status NOT IN ('succeeded', 'failed', 'cancelled')",
            params![operation_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|_| "storage_unavailable")?;
    if updated == 0 {
        return Err("operation_not_cancellable".to_owned());
    }
    connection
        .query_row(
            "SELECT id, session_id, generation_id, tool_name, status, progress_sequence, \
             progress_message, result_artifact_ids_json, error_code, created_at, updated_at \
             FROM native_tool_operations WHERE id = ?1",
            [operation_id],
            operation_from_row,
        )
        .map_err(|_| "storage_unavailable".to_owned())
}

fn operation_from_row(row: &Row<'_>) -> rusqlite::Result<Value> {
    let tool_name = row.get::<_, String>(3)?;
    let raw_status = row.get::<_, String>(4)?;
    let progress_sequence = row.get::<_, i64>(5)?;
    let progress_message = row.get::<_, Option<String>>(6)?;
    let artifacts_json = row.get::<_, String>(7)?;
    let artifact_ids = serde_json::from_str::<Value>(&artifacts_json).unwrap_or_else(|_| json!([]));
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "agentId": "onepiece",
        "sessionId": row.get::<_, String>(1)?,
        "capability": capability(&tool_name),
        "operation": tool_name,
        "status": frontend_status(&raw_status),
        "progress": progress_message.map(|message| json!({
            "phase": message,
            "completedUnits": progress_sequence,
            "totalUnits": Value::Null,
            "messageCode": Value::Null
        })),
        "artifactIds": artifact_ids,
        "errorCode": row.get::<_, Option<String>>(8)?,
        "simulated": false,
        "createdAt": row.get::<_, String>(9)?,
        "updatedAt": row.get::<_, String>(10)?
    }))
}

fn capability(tool_name: &str) -> &'static str {
    match tool_name {
        "browser" => "browser",
        "web_search" | "web_fetch" => "web",
        "code_execution" => "code_execution",
        "ocr" => "ocr",
        "artifact" => "artifact",
        "delegate_cli" | "apply_delegation_changes" => "delegation",
        "shell" => "command",
        _ => "filesystem",
    }
}

fn frontend_status(status: &str) -> &str {
    match status {
        "awaiting_approval" => "queued",
        known
        @ ("queued" | "running" | "awaiting_human" | "succeeded" | "failed" | "cancelled") => known,
        _ => "failed",
    }
}
