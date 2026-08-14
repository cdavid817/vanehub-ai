use crate::contexts::agent_runtime::application::{
    ManualApplyDelegationRequest, ManualNativeToolRequest, ManualNativeToolService,
    ManualStartDelegationRequest, NativeToolErrorCode, NativeToolResultStatus,
    ToolApprovalDecision,
};
use crate::platform::database::NativeDatabase;
use rusqlite::Row;
use serde_json::{json, Value};

#[derive(Clone)]
pub(crate) struct ManualNativeToolControl {
    service: ManualNativeToolService,
    database: NativeDatabase,
}

impl ManualNativeToolControl {
    pub(crate) fn new(service: ManualNativeToolService, database: NativeDatabase) -> Self {
        Self { service, database }
    }

    pub(crate) fn start_delegation(
        &self,
        input: ManualStartDelegationRequest,
    ) -> Result<Value, String> {
        validate_start(&input)?;
        let result = self
            .service
            .execute(ManualNativeToolRequest {
                agent_id: input.agent_id,
                session_id: input.session_id,
                tool_name: "delegate_cli".to_owned(),
                input: json!({
                    "target": input.provider,
                    "mode": input.mode,
                    "task": input.prompt,
                    "artifact_ids": input.artifact_ids,
                }),
                authority_artifact_id: None,
            })
            .map_err(|error| error.code.as_str().to_owned())?;
        require_success(result.result.status, result.result.error_code)?;
        self.attempt_summary(&result.operation_id)
    }

    pub(crate) fn apply_delegation_changes(
        &self,
        input: ManualApplyDelegationRequest,
    ) -> Result<Value, String> {
        validate_apply(&input)?;
        let artifact_id = input.artifact_id.clone();
        let result = self
            .service
            .execute(ManualNativeToolRequest {
                agent_id: input.agent_id,
                session_id: input.session_id,
                tool_name: "apply_delegation_changes".to_owned(),
                input: json!({
                    "artifact_id": input.artifact_id,
                    "content_hash": input.expected_content_hash,
                    "diff_hash": input.expected_diff_hash,
                    "target_repository_identity": input.repository_identity,
                    "base_commit": input.base_commit,
                    "acknowledged": input.acknowledgement,
                }),
                authority_artifact_id: Some(artifact_id),
            })
            .map_err(|error| error.code.as_str().to_owned())?;
        self.operation(&result.operation_id)
    }

    pub(crate) fn resolve_approval(
        &self,
        session_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> bool {
        self.service.resolve_approval(session_id, call_id, decision)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> bool {
        self.service.cancel(operation_id)
    }

    fn attempt_summary(&self, attempt_id: &str) -> Result<Value, String> {
        self.database
            .connection()
            .map_err(|_| "storage_unavailable")?
            .query_row(
                "SELECT a.id, a.delegation_id, a.target, a.mode, a.status, c.base_commit, \
                 a.change_set_artifact_id, COALESCE(a.started_at, d.created_at), a.completed_at \
                 FROM native_tool_delegation_attempts a \
                 JOIN native_tool_delegations d ON d.id = a.delegation_id \
                 LEFT JOIN native_tool_change_sets c ON c.attempt_id = a.id WHERE a.id = ?1",
                [attempt_id],
                attempt_from_row,
            )
            .map_err(query_error)
    }

    fn operation(&self, operation_id: &str) -> Result<Value, String> {
        self.database
            .connection()
            .map_err(|_| "storage_unavailable")?
            .query_row(
                "SELECT id, session_id, generation_id, tool_name, status, progress_sequence, \
                 progress_message, result_artifact_ids_json, error_code, created_at, updated_at \
                 FROM native_tool_operations WHERE id = ?1",
                [operation_id],
                operation_from_row,
            )
            .map_err(query_error)
    }
}

fn validate_start(input: &ManualStartDelegationRequest) -> Result<(), String> {
    if input.agent_id != "onepiece"
        || input.session_id.trim().is_empty()
        || !matches!(input.provider.as_str(), "claude_code" | "codex_cli")
        || !matches!(input.mode.as_str(), "analyze" | "edit")
        || input.prompt.trim().is_empty()
        || input.prompt.len() > 8_000
        || input.artifact_ids.len() > 16
    {
        return Err("invalid_input".to_owned());
    }
    Ok(())
}

fn validate_apply(input: &ManualApplyDelegationRequest) -> Result<(), String> {
    if input.agent_id != "onepiece"
        || input.session_id.trim().is_empty()
        || !input.acknowledgement
        || input.artifact_id.trim().is_empty()
        || input.expected_content_hash.trim().is_empty()
        || input.expected_diff_hash.trim().is_empty()
        || input.repository_identity.trim().is_empty()
        || input.base_commit.trim().is_empty()
    {
        return Err("invalid_input".to_owned());
    }
    Ok(())
}

fn require_success(
    status: NativeToolResultStatus,
    error: Option<NativeToolErrorCode>,
) -> Result<(), String> {
    if status == NativeToolResultStatus::Succeeded {
        Ok(())
    } else {
        Err(error.map_or_else(
            || "manual_dispatch_failed".to_owned(),
            |code| code.as_str().to_owned(),
        ))
    }
}

fn attempt_from_row(row: &Row<'_>) -> rusqlite::Result<Value> {
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
}

fn operation_from_row(row: &Row<'_>) -> rusqlite::Result<Value> {
    let status = row.get::<_, String>(4)?;
    let artifacts =
        serde_json::from_str::<Value>(&row.get::<_, String>(7)?).unwrap_or_else(|_| json!([]));
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "agentId": "onepiece",
        "sessionId": row.get::<_, String>(1)?,
        "capability": "delegation",
        "operation": row.get::<_, String>(3)?,
        "status": frontend_status(&status),
        "progress": row.get::<_, Option<String>>(6)?.map(|message| json!({
            "phase": message,
            "completedUnits": row.get::<_, i64>(5).unwrap_or_default(),
            "totalUnits": Value::Null,
            "messageCode": Value::Null
        })),
        "artifactIds": artifacts,
        "errorCode": row.get::<_, Option<String>>(8)?,
        "simulated": false,
        "createdAt": row.get::<_, String>(9)?,
        "updatedAt": row.get::<_, String>(10)?
    }))
}

fn frontend_status(status: &str) -> &str {
    match status {
        "awaiting_approval" => "queued",
        known
        @ ("queued" | "running" | "awaiting_human" | "succeeded" | "failed" | "cancelled") => known,
        _ => "failed",
    }
}

fn query_error(error: rusqlite::Error) -> String {
    match error {
        rusqlite::Error::QueryReturnedNoRows => "manual_operation_not_found".to_owned(),
        _ => "storage_unavailable".to_owned(),
    }
}
