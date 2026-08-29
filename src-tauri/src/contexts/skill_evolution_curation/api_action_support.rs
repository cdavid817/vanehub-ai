use super::api::SkillEvolutionCurationApi;
use super::api_models::*;
use super::api_queries::safe_state;
use super::{domain::CuratorApplication, infrastructure::CuratorPolicyRetentionError};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, to_value, Value};

impl SkillEvolutionCurationApi {
    pub(super) fn connection(
        &self,
    ) -> Result<crate::platform::database::PooledSqlite, CuratorApiError> {
        self.database.connection().map_err(|_| storage())
    }

    pub(super) fn workspace(&self, candidate_id: &str) -> Result<String, CuratorApiError> {
        self.connection()?
            .query_row(
                "SELECT workspace_id FROM evolution_curator_candidates WHERE candidate_id=?1",
                [candidate_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| storage())?
            .ok_or_else(not_found)
    }

    pub(super) fn action_error(&self, candidate_id: &str, message: String) -> CuratorApiError {
        let (code, reason) = classify(&message);
        let current = self
            .connection()
            .ok()
            .and_then(|connection| safe_state(&connection, candidate_id))
            .map(Box::new);
        CuratorApiError {
            code,
            message: code,
            current,
            reason_code: reason,
        }
    }
}

pub(super) fn policy_error(error: CuratorPolicyRetentionError) -> CuratorApiError {
    match error {
        CuratorPolicyRetentionError::Conflict { .. } => CuratorApiError::new("stale_conflict"),
        CuratorPolicyRetentionError::Storage => storage(),
        CuratorPolicyRetentionError::NotFound => not_found(),
        CuratorPolicyRetentionError::Policy(_) | CuratorPolicyRetentionError::InvalidInput => {
            invalid()
        }
    }
}

fn classify(message: &str) -> (&'static str, Option<String>) {
    if message.contains("not found") {
        ("not_found", None)
    } else if message.contains("concurrent")
        || message.contains("conflict")
        || message.contains("stale")
    {
        ("stale_conflict", None)
    } else if message.contains("expired") {
        ("preview_expired", None)
    } else if message.contains("Pinned") || message.contains("pinned") {
        ("pinned", None)
    } else if message.contains("rejected") {
        (
            "unsafe_content",
            message
                .rsplit_once(": ")
                .map(|(_, value)| value.to_string()),
        )
    } else if message.contains("storage") || message.contains("unavailable") {
        ("storage_unavailable", None)
    } else if message.contains("not approvable") || message.contains("current state") {
        ("not_approvable", None)
    } else if message.contains("overlay application") || message.contains("recovery") {
        ("application_failed", None)
    } else {
        ("invalid_input", None)
    }
}

pub(super) fn action_receipt(
    connection: &Connection,
    id: &str,
    action_id: &str,
    duplicate: bool,
) -> CuratorApiResult {
    let state = safe_state(connection, id).ok_or_else(not_found)?;
    merge(state, json!({"actionId":action_id,"duplicate":duplicate}))
}

pub(super) fn application_result(
    connection: &Connection,
    application: CuratorApplication,
) -> CuratorApiResult {
    let state = safe_state(connection, &application.candidate_id).ok_or_else(not_found)?;
    merge(state, to_value(application).map_err(|_| storage())?)
}

fn merge(state: impl Serialize, extra: Value) -> CuratorApiResult {
    let mut value = to_value(state).map_err(|_| storage())?;
    value
        .as_object_mut()
        .ok_or_else(storage)?
        .extend(extra.as_object().ok_or_else(storage)?.clone());
    Ok(value)
}

pub(super) fn validate_key(key: &str) -> Result<(), CuratorApiError> {
    if key.trim().is_empty() || key.len() > 160 {
        Err(invalid())
    } else {
        Ok(())
    }
}
pub(super) fn invalid() -> CuratorApiError {
    CuratorApiError::new("invalid_input")
}
fn not_found() -> CuratorApiError {
    CuratorApiError::new("not_found")
}
pub(super) fn storage() -> CuratorApiError {
    CuratorApiError::new("storage_unavailable")
}
