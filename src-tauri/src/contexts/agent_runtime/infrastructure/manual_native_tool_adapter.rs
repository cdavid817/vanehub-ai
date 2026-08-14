use crate::contexts::agent_runtime::application::{
    ManualNativeToolAuthorityPort, ManualNativeToolOperationPort, StoredToolOperation,
    StoredToolOperationStatus,
};
use crate::contexts::sessions::api::SessionsApi;
use crate::platform::database::NativeDatabase;
use crate::platform::filesystem::normalize_windows_extended_length_path;
use rusqlite::params;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub(crate) struct ManualNativeToolAuthorityAdapter {
    sessions: SessionsApi,
    database: NativeDatabase,
}

impl ManualNativeToolAuthorityAdapter {
    pub(crate) fn new(sessions: SessionsApi, database: NativeDatabase) -> Self {
        Self { sessions, database }
    }
}

impl ManualNativeToolAuthorityPort for ManualNativeToolAuthorityAdapter {
    fn resolve(
        &self,
        session_id: &str,
        agent_id: &str,
        artifact_id: Option<&str>,
    ) -> Result<PathBuf, &'static str> {
        let session = self
            .sessions
            .runtime_session(session_id)
            .map_err(|_| "session_unavailable")?
            .ok_or("session_not_found")?;
        if session.agent_id != agent_id || agent_id != "onepiece" || session.archived {
            return Err("session_authority_mismatch");
        }
        if let Some(artifact_id) = artifact_id {
            self.verify_change_set_ownership(session_id, artifact_id)?;
        }
        canonical_local_workspace(session.folder.as_deref())
    }
}

impl ManualNativeToolAuthorityAdapter {
    fn verify_change_set_ownership(
        &self,
        session_id: &str,
        artifact_id: &str,
    ) -> Result<(), &'static str> {
        let count: i64 = self
            .database
            .connection()
            .map_err(|_| "storage_unavailable")?
            .query_row(
                "SELECT COUNT(*) FROM native_tool_change_sets c \
                 JOIN native_tool_delegation_attempts a ON a.id = c.attempt_id \
                 JOIN native_tool_delegations d ON d.id = a.delegation_id \
                 WHERE c.artifact_id = ?1 AND d.session_id = ?2",
                params![artifact_id, session_id],
                |row| row.get(0),
            )
            .map_err(|_| "storage_unavailable")?;
        if count == 1 {
            Ok(())
        } else {
            Err("change_set_authority_mismatch")
        }
    }
}

fn canonical_local_workspace(folder: Option<&str>) -> Result<PathBuf, &'static str> {
    let folder = folder
        .filter(|value| !value.trim().is_empty())
        .ok_or("workspace_unavailable")?;
    if folder.contains("://") {
        return Err("local_workspace_required");
    }
    let canonical = Path::new(folder)
        .canonicalize()
        .map_err(|_| "workspace_unavailable")?;
    if !canonical.is_dir() {
        return Err("workspace_unavailable");
    }
    Ok(PathBuf::from(normalize_windows_extended_length_path(
        &canonical.to_string_lossy(),
    )))
}

#[derive(Clone)]
pub(crate) struct ManualNativeToolOperationAdapter {
    repository: super::SqliteNativeToolRepository,
    app: AppHandle,
}

impl ManualNativeToolOperationAdapter {
    pub(crate) fn new(repository: super::SqliteNativeToolRepository, app: AppHandle) -> Self {
        Self { repository, app }
    }
}

impl ManualNativeToolOperationPort for ManualNativeToolOperationAdapter {
    fn save(&self, operation: &StoredToolOperation) -> Result<(), ()> {
        self.repository.save_operation(operation).map_err(|_| ())?;
        let _ = self
            .app
            .emit("builtin-tool-operation", operation_event(operation));
        Ok(())
    }
}

fn operation_event(record: &StoredToolOperation) -> Value {
    let progress = record.progress_message.as_ref().map(|message| {
        json!({
            "phase": message,
            "completedUnits": record.progress_sequence,
            "totalUnits": Value::Null,
            "messageCode": Value::Null
        })
    });
    json!({
        "kind": "snapshot",
        "operation": {
            "id": record.id,
            "agentId": "onepiece",
            "sessionId": record.session_id,
            "capability": "delegation",
            "operation": record.tool_name,
            "status": frontend_status(record.status),
            "progress": progress,
            "artifactIds": record.result_artifact_ids,
            "errorCode": record.error_code,
            "simulated": false,
            "createdAt": record.created_at,
            "updatedAt": record.updated_at
        }
    })
}

fn frontend_status(status: StoredToolOperationStatus) -> &'static str {
    match status {
        StoredToolOperationStatus::AwaitingApproval => "queued",
        other => other.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_local_workspace;

    #[test]
    fn remote_workspace_strings_never_become_local_authority() {
        assert_eq!(
            canonical_local_workspace(Some("ssh://host/repository")),
            Err("local_workspace_required")
        );
    }
}
