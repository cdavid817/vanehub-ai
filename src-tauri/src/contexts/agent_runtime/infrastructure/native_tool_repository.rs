#![allow(dead_code)]

use crate::contexts::agent_runtime::application::{
    ArtifactRecord, ChangeSetApplyRecord, ChangeSetRecord, DelegationAttemptRecord,
    DelegationRecord, NativeToolPersistencePort, RecoveryRecord, StoredToolOperation,
};
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::params;

#[derive(Clone)]
pub(crate) struct SqliteNativeToolRepository {
    database: NativeDatabase,
}

impl NativeToolPersistencePort for SqliteNativeToolRepository {
    fn save_delegation(&self, record: &DelegationRecord) -> Result<(), ()> {
        SqliteNativeToolRepository::save_delegation(self, record).map_err(|_| ())
    }

    fn save_delegation_attempt(&self, record: &DelegationAttemptRecord) -> Result<(), ()> {
        SqliteNativeToolRepository::save_delegation_attempt(self, record).map_err(|_| ())
    }

    fn insert_change_set(&self, record: &ChangeSetRecord) -> Result<(), ()> {
        SqliteNativeToolRepository::insert_change_set(self, record).map_err(|_| ())
    }

    fn save_apply_attempt(&self, record: &ChangeSetApplyRecord) -> Result<(), ()> {
        SqliteNativeToolRepository::save_apply_attempt(self, record).map_err(|_| ())
    }

    fn is_change_set_available(&self, artifact_id: &str) -> Result<bool, ()> {
        SqliteNativeToolRepository::is_change_set_available(self, artifact_id).map_err(|_| ())
    }

    fn save_recovery(&self, record: &RecoveryRecord) -> Result<(), ()> {
        SqliteNativeToolRepository::save_recovery(self, record).map_err(|_| ())
    }
}

impl SqliteNativeToolRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn save_operation(&self, record: &StoredToolOperation) -> Result<(), DatabaseError> {
        let artifact_ids = to_json(&record.result_artifact_ids)?;
        self.database.connection()?.execute(
            r#"
            INSERT INTO native_tool_operations (
                id, contract_version, session_id, generation_id, tool_name, status,
                progress_sequence, progress_message, result_artifact_ids_json, error_code,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                progress_sequence = excluded.progress_sequence,
                progress_message = excluded.progress_message,
                result_artifact_ids_json = excluded.result_artifact_ids_json,
                error_code = excluded.error_code,
                updated_at = excluded.updated_at
            WHERE excluded.progress_sequence >= native_tool_operations.progress_sequence
              AND (
                native_tool_operations.status NOT IN ('succeeded', 'failed', 'cancelled')
                OR excluded.status = native_tool_operations.status
              )
            "#,
            params![
                record.id,
                record.contract_version,
                record.session_id,
                record.generation_id,
                record.tool_name,
                record.status.as_str(),
                record.progress_sequence,
                record.progress_message,
                artifact_ids,
                record.error_code,
                record.created_at,
                record.updated_at,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn insert_artifact(&self, record: &ArtifactRecord) -> Result<(), DatabaseError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            r#"
            INSERT INTO native_tool_artifacts (
                id, contract_version, content_hash, media_type, size_bytes, display_name,
                source_operation_id, created_at, expires_at, publication_ref
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                record.id,
                record.contract_version,
                record.content_hash,
                record.media_type,
                checked_i64(record.size_bytes)?,
                record.display_name,
                record.source_operation_id,
                record.created_at,
                record.expires_at,
                record.publication_ref,
            ],
        )?;
        for (ordinal, source_id) in record.source_artifact_ids.iter().enumerate() {
            transaction.execute(
                "INSERT INTO native_tool_artifact_lineage \
                 (artifact_id, source_artifact_id, ordinal) VALUES (?1, ?2, ?3)",
                params![record.id, source_id, checked_i64(ordinal as u64)?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_delegation(&self, record: &DelegationRecord) -> Result<(), DatabaseError> {
        self.database.connection()?.execute(
            r#"
            INSERT INTO native_tool_delegations (
                id, contract_version, session_id, task_hash, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET status = excluded.status, updated_at = excluded.updated_at
            WHERE native_tool_delegations.status NOT IN ('succeeded', 'failed', 'cancelled')
               OR excluded.status = native_tool_delegations.status
            "#,
            params![
                record.id,
                record.contract_version,
                record.session_id,
                record.task_hash,
                record.status.as_str(),
                record.created_at,
                record.updated_at,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn save_delegation_attempt(
        &self,
        record: &DelegationAttemptRecord,
    ) -> Result<(), DatabaseError> {
        self.database.connection()?.execute(
            r#"
            INSERT INTO native_tool_delegation_attempts (
                id, contract_version, delegation_id, attempt_number, target, mode, status,
                safe_summary, report_artifact_id, change_set_artifact_id, error_code,
                started_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                safe_summary = excluded.safe_summary,
                report_artifact_id = excluded.report_artifact_id,
                change_set_artifact_id = excluded.change_set_artifact_id,
                error_code = excluded.error_code,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at
            WHERE native_tool_delegation_attempts.status NOT IN ('succeeded', 'failed', 'cancelled')
               OR excluded.status = native_tool_delegation_attempts.status
            "#,
            params![
                record.id,
                record.contract_version,
                record.delegation_id,
                record.attempt_number,
                record.target.as_str(),
                record.mode.as_str(),
                record.status.as_str(),
                record.safe_summary,
                record.report_artifact_id,
                record.change_set_artifact_id,
                record.error_code,
                record.started_at,
                record.completed_at,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn insert_change_set(&self, record: &ChangeSetRecord) -> Result<(), DatabaseError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            r#"
            INSERT INTO native_tool_change_sets (
                artifact_id, contract_version, content_hash, repository_identity,
                base_commit, attempt_id, warnings_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                record.artifact_id,
                record.contract_version,
                record.content_hash,
                record.repository_identity,
                record.base_commit,
                record.attempt_id,
                to_json(&record.warnings)?,
                record.created_at,
            ],
        )?;
        for (ordinal, file) in record.files.iter().enumerate() {
            transaction.execute(
                r#"
                INSERT INTO native_tool_change_set_files (
                    change_set_artifact_id, ordinal, path, change_kind, old_hash, new_hash,
                    binary, mode
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    record.artifact_id,
                    checked_i64(ordinal as u64)?,
                    file.path,
                    file.change_kind.as_str(),
                    file.old_hash,
                    file.new_hash,
                    file.binary,
                    file.mode,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_apply_attempt(
        &self,
        record: &ChangeSetApplyRecord,
    ) -> Result<(), DatabaseError> {
        self.database.connection()?.execute(
            r#"
            INSERT INTO native_tool_apply_attempts (
                id, contract_version, change_set_artifact_id, target_repository_identity,
                expected_base_commit, approval_input_hash, status, error_code, consumed_at,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                error_code = excluded.error_code,
                consumed_at = excluded.consumed_at,
                updated_at = excluded.updated_at
            "#,
            params![
                record.id,
                record.contract_version,
                record.change_set_artifact_id,
                record.target_repository_identity,
                record.expected_base_commit,
                record.approval_input_hash,
                record.status.as_str(),
                record.error_code,
                record.consumed_at,
                record.created_at,
                record.updated_at,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn is_change_set_available(&self, artifact_id: &str) -> Result<bool, DatabaseError> {
        let count: i64 = self.database.connection()?.query_row(
            "SELECT COUNT(*) FROM native_tool_apply_attempts WHERE change_set_artifact_id = ?1 AND status = 'succeeded'",
            params![artifact_id],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    pub(crate) fn save_recovery(&self, record: &RecoveryRecord) -> Result<(), DatabaseError> {
        self.database.connection()?.execute(
            r#"
            INSERT INTO native_tool_apply_recovery (
                apply_attempt_id, contract_version, status, recovery_reference,
                safe_instructions_json, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(apply_attempt_id) DO UPDATE SET
                status = excluded.status,
                recovery_reference = excluded.recovery_reference,
                safe_instructions_json = excluded.safe_instructions_json,
                updated_at = excluded.updated_at
            "#,
            params![
                record.apply_attempt_id,
                record.contract_version,
                record.status.as_str(),
                record.recovery_reference,
                to_json(&record.safe_instructions)?,
                record.updated_at,
            ],
        )?;
        Ok(())
    }
}

fn checked_i64(value: u64) -> Result<i64, DatabaseError> {
    i64::try_from(value)
        .map_err(|_| DatabaseError::Storage("native tool numeric value exceeds SQLite".to_owned()))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, DatabaseError> {
    serde_json::to_string(value).map_err(|error| DatabaseError::Storage(error.to_string()))
}

#[cfg(test)]
#[path = "native_tool_repository_tests.rs"]
mod tests;
