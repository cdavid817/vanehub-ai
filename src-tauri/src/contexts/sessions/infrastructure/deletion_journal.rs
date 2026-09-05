//! The durable record of every session deletion: what was asked, what was authorized, what
//! happened to the directory, and what happened to the rows.
//!
//! No table here references `sessions` by foreign key. The journal must describe the deletion
//! of a session after that session is gone, and a claim must be able to outlive a session row
//! that failed to delete — so cascade would destroy exactly the evidence recovery needs.

use crate::contexts::sessions::application::{
    deletion_error_code, DeletionGroupResult, DeletionGroupStatus, DeletionJournalPort,
    DeletionOutcome, DeletionOwner, DeletionPhase, DeletionRuntimeEffect, GroupCompletion,
    GroupPatch, GroupSnapshot, JournalCreateOutcome, NewDeletionOperation, OperationOwnership,
    OperationPatch, SessionDbEffect, SessionDeletionClaim, SessionDeletionOperation,
    SessionsApplicationError, WorktreeDeletionPolicy, WorktreeEffect,
};
use crate::platform::database::{DatabaseError, NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

pub(crate) fn apply_session_deletion_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS session_deletion_operations (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL UNIQUE,
            request_hash TEXT NOT NULL,
            outcome TEXT NOT NULL,
            phase TEXT NOT NULL,
            revision INTEGER NOT NULL,
            runtime_effect TEXT NOT NULL,
            owner_instance_id TEXT NOT NULL,
            owner_epoch INTEGER NOT NULL,
            last_retry_request_id TEXT,
            error_code TEXT,
            operation_task_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_session_deletion_operations_outcome
            ON session_deletion_operations(outcome, created_at);
        CREATE TABLE IF NOT EXISTS session_deletion_groups (
            id TEXT PRIMARY KEY,
            operation_id TEXT NOT NULL REFERENCES session_deletion_operations(id),
            sequence INTEGER NOT NULL,
            worktree_key TEXT,
            worktree_id TEXT,
            policy TEXT NOT NULL,
            session_ids TEXT NOT NULL,
            status TEXT NOT NULL,
            phase TEXT NOT NULL,
            worktree_effect TEXT NOT NULL,
            db_effect TEXT NOT NULL,
            error_code TEXT,
            retained_path TEXT,
            attempt INTEGER NOT NULL,
            revision INTEGER NOT NULL,
            authorization TEXT,
            execution_snapshot TEXT,
            receipt TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_session_deletion_groups_operation
            ON session_deletion_groups(operation_id, sequence);
        CREATE TABLE IF NOT EXISTS session_deletion_claims (
            session_id TEXT PRIMARY KEY,
            operation_id TEXT NOT NULL,
            group_id TEXT NOT NULL,
            claimed_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct SqliteDeletionJournal {
    database: NativeDatabase,
    clock: std::sync::Arc<dyn crate::contexts::sessions::application::DeletionClockPort>,
}

impl SqliteDeletionJournal {
    pub(crate) fn new(
        database: NativeDatabase,
        clock: std::sync::Arc<dyn crate::contexts::sessions::application::DeletionClockPort>,
    ) -> Self {
        Self { database, clock }
    }

    fn connection(&self) -> Result<PooledSqlite, SessionsApplicationError> {
        self.database
            .connection()
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))
    }
}

const OPERATION_COLUMNS: &str = "id, request_id, request_hash, outcome, phase, revision, runtime_effect, owner_instance_id, owner_epoch, last_retry_request_id, error_code, operation_task_id, created_at, updated_at, completed_at";
const GROUP_COLUMNS: &str = "id, operation_id, sequence, worktree_key, worktree_id, policy, session_ids, status, phase, worktree_effect, db_effect, error_code, retained_path, attempt, revision, authorization, execution_snapshot, receipt, updated_at";

struct OperationRow {
    operation: SessionDeletionOperation,
    owner: DeletionOwner,
    last_retry_request_id: Option<String>,
}

fn read_operation(row: &Row<'_>) -> rusqlite::Result<OperationRow> {
    let outcome: String = row.get(3)?;
    let phase: String = row.get(4)?;
    let runtime_effect: String = row.get(6)?;
    Ok(OperationRow {
        operation: SessionDeletionOperation {
            operation_id: row.get(0)?,
            request_id: row.get(1)?,
            outcome: DeletionOutcome::parse(&outcome).unwrap_or(DeletionOutcome::NeedsAttention),
            phase: DeletionPhase::parse(&phase).unwrap_or(DeletionPhase::Accepted),
            revision: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
            runtime_effect: if runtime_effect == "simulated" {
                DeletionRuntimeEffect::Simulated
            } else {
                DeletionRuntimeEffect::Native
            },
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
            completed_at: row.get(14)?,
            groups: Vec::new(),
            error_code: row.get(10)?,
            operation_task_id: row.get(11)?,
        },
        owner: DeletionOwner {
            instance_id: row.get(7)?,
            epoch: u64::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
        },
        last_retry_request_id: row.get(9)?,
    })
}

fn read_group(row: &Row<'_>) -> rusqlite::Result<DeletionGroupResult> {
    let policy: String = row.get(5)?;
    let session_ids: String = row.get(6)?;
    let status: String = row.get(7)?;
    let phase: String = row.get(8)?;
    let worktree_effect: String = row.get(9)?;
    let db_effect: String = row.get(10)?;
    Ok(DeletionGroupResult {
        group_id: row.get(0)?,
        worktree_key: row.get(3)?,
        worktree_id: row.get(4)?,
        policy: WorktreeDeletionPolicy::parse(&policy).unwrap_or(WorktreeDeletionPolicy::Keep),
        session_ids: serde_json::from_str(&session_ids).unwrap_or_default(),
        status: DeletionGroupStatus::parse(&status).unwrap_or(DeletionGroupStatus::NeedsAttention),
        phase: DeletionPhase::parse(&phase).unwrap_or(DeletionPhase::Accepted),
        worktree_effect: WorktreeEffect::parse(&worktree_effect)
            .unwrap_or(WorktreeEffect::RemovalUnknown),
        db_effect: SessionDbEffect::parse(&db_effect).unwrap_or(SessionDbEffect::Pending),
        error_code: row.get(11)?,
        retained_path: row.get(12)?,
        attempt: u32::try_from(row.get::<_, i64>(13)?).unwrap_or(0),
        revision: u64::try_from(row.get::<_, i64>(14)?).unwrap_or(0),
    })
}

fn load_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<OperationRow>, SessionsApplicationError> {
    let mut row = connection
        .query_row(
            &format!("SELECT {OPERATION_COLUMNS} FROM session_deletion_operations WHERE id = ?1"),
            [operation_id],
            read_operation,
        )
        .optional()
        .map_err(repository_error)?;
    if let Some(row) = row.as_mut() {
        row.operation.groups = load_groups(connection, operation_id)?;
    }
    Ok(row)
}

fn load_groups(
    connection: &Connection,
    operation_id: &str,
) -> Result<Vec<DeletionGroupResult>, SessionsApplicationError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {GROUP_COLUMNS} FROM session_deletion_groups WHERE operation_id = ?1 ORDER BY sequence"
        ))
        .map_err(repository_error)?;
    let groups = statement
        .query_map([operation_id], read_group)
        .map_err(repository_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(repository_error)?;
    Ok(groups)
}

fn json_text(value: &Option<serde_json::Value>) -> Option<String> {
    value.as_ref().map(ToString::to_string)
}

fn json_value(text: Option<String>) -> Option<serde_json::Value> {
    text.and_then(|text| serde_json::from_str(&text).ok())
}

// Every write here is read-then-write inside one transaction (a compare-and-set on the revision,
// or a claim lookup before the insert), so each begins `IMMEDIATE`. A deferred transaction that
// tries to upgrade after another connection has written gets `SQLITE_BUSY` immediately, without
// the busy timeout being consulted — which is how a Git removal that had just succeeded once
// failed to have its receipt recorded while the terminal usage poll was writing.
impl DeletionJournalPort for SqliteDeletionJournal {
    fn create(
        &self,
        operation: &NewDeletionOperation,
    ) -> Result<JournalCreateOutcome, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(repository_error)?;
        let existing: Option<(String, String)> = transaction
            .query_row(
                "SELECT id, request_hash FROM session_deletion_operations WHERE request_id = ?1",
                [&operation.request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(repository_error)?;
        if let Some((existing_id, existing_hash)) = existing {
            if existing_hash != operation.request_hash {
                return Ok(JournalCreateOutcome::RequestConflict);
            }
            let row = load_operation(&transaction, &existing_id)?.ok_or_else(|| {
                SessionsApplicationError::Repository("operation vanished".to_string())
            })?;
            return Ok(JournalCreateOutcome::Existing(row.operation));
        }
        for group in &operation.groups {
            for session_id in &group.session_ids {
                let claimed: Option<String> = transaction
                    .query_row(
                        "SELECT operation_id FROM session_deletion_claims WHERE session_id = ?1",
                        [session_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(repository_error)?;
                if let Some(operation_id) = claimed {
                    return Ok(JournalCreateOutcome::SessionClaimed {
                        session_id: session_id.clone(),
                        operation_id,
                    });
                }
            }
        }
        transaction
            .execute(
                &format!("INSERT INTO session_deletion_operations ({OPERATION_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, NULL, NULL, ?9, ?10, ?10, NULL)"),
                params![
                    operation.operation_id,
                    operation.request_id,
                    operation.request_hash,
                    DeletionOutcome::Pending.as_str(),
                    DeletionPhase::Accepted.as_str(),
                    match operation.runtime_effect {
                        DeletionRuntimeEffect::Native => "native",
                        DeletionRuntimeEffect::Simulated => "simulated",
                    },
                    operation.owner.instance_id,
                    i64::try_from(operation.owner.epoch).unwrap_or(i64::MAX),
                    operation.operation_task_id,
                    operation.created_at,
                ],
            )
            .map_err(repository_error)?;
        for (sequence, group) in operation.groups.iter().enumerate() {
            transaction
                .execute(
                    &format!("INSERT INTO session_deletion_groups ({GROUP_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, 0, 1, ?13, NULL, NULL, ?14)"),
                    params![
                        group.group_id,
                        operation.operation_id,
                        sequence as i64,
                        group.worktree_key,
                        group.worktree_id,
                        group.policy.as_str(),
                        serde_json::to_string(&group.session_ids).unwrap_or_else(|_| "[]".to_string()),
                        DeletionGroupStatus::Pending.as_str(),
                        DeletionPhase::Accepted.as_str(),
                        WorktreeEffect::NotRequested.as_str(),
                        SessionDbEffect::Pending.as_str(),
                        group.retained_path,
                        json_text(&group.authorization),
                        operation.created_at,
                    ],
                )
                .map_err(repository_error)?;
            for session_id in &group.session_ids {
                transaction
                    .execute(
                        "INSERT INTO session_deletion_claims (session_id, operation_id, group_id, claimed_at) VALUES (?1, ?2, ?3, ?4)",
                        params![session_id, operation.operation_id, group.group_id, operation.created_at],
                    )
                    .map_err(repository_error)?;
            }
        }
        let created = load_operation(&transaction, &operation.operation_id)?.ok_or_else(|| {
            SessionsApplicationError::Repository("operation not written".to_string())
        })?;
        transaction.commit().map_err(repository_error)?;
        Ok(JournalCreateOutcome::Created(created.operation))
    }

    fn load(
        &self,
        operation_id: &str,
    ) -> Result<Option<SessionDeletionOperation>, SessionsApplicationError> {
        Ok(load_operation(&*self.connection()?, operation_id)?.map(|row| row.operation))
    }

    fn list_pending(&self) -> Result<Vec<SessionDeletionOperation>, SessionsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT id FROM session_deletion_operations WHERE outcome = 'pending' ORDER BY created_at")
            .map_err(repository_error)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        let mut operations = Vec::new();
        for id in ids {
            if let Some(row) = load_operation(&connection, &id)? {
                operations.push(row.operation);
            }
        }
        Ok(operations)
    }

    fn ownership(
        &self,
        operation_id: &str,
    ) -> Result<Option<OperationOwnership>, SessionsApplicationError> {
        Ok(
            load_operation(&*self.connection()?, operation_id)?.map(|row| OperationOwnership {
                owner: row.owner,
                last_retry_request_id: row.last_retry_request_id,
            }),
        )
    }

    fn update_operation(
        &self,
        operation_id: &str,
        expected_revision: u64,
        patch: &OperationPatch,
    ) -> Result<u64, SessionsApplicationError> {
        let connection = self.connection()?;
        let Some(current) = load_operation(&connection, operation_id)? else {
            return Err(SessionsApplicationError::Validation(
                deletion_error_code::OPERATION_NOT_FOUND.to_string(),
            ));
        };
        let now = self.clock.now();
        let next_revision = expected_revision + 1;
        let outcome = patch.outcome.unwrap_or(current.operation.outcome);
        let phase = patch.phase.unwrap_or(current.operation.phase);
        let error_code = match &patch.error_code {
            Some(code) => code.clone(),
            None => current.operation.error_code.clone(),
        };
        let owner = patch.owner.clone().unwrap_or(current.owner);
        let retry = patch
            .last_retry_request_id
            .clone()
            .or(current.last_retry_request_id);
        let completed_at = if patch.completed {
            Some(now.clone())
        } else {
            current.operation.completed_at
        };
        let changed = connection
            .execute(
                "UPDATE session_deletion_operations SET outcome = ?2, phase = ?3, revision = ?4, error_code = ?5, owner_instance_id = ?6, owner_epoch = ?7, last_retry_request_id = ?8, updated_at = ?9, completed_at = ?10 WHERE id = ?1 AND revision = ?11",
                params![
                    operation_id,
                    outcome.as_str(),
                    phase.as_str(),
                    i64::try_from(next_revision).unwrap_or(i64::MAX),
                    error_code,
                    owner.instance_id,
                    i64::try_from(owner.epoch).unwrap_or(i64::MAX),
                    retry,
                    now,
                    completed_at,
                    i64::try_from(expected_revision).unwrap_or(i64::MAX),
                ],
            )
            .map_err(repository_error)?;
        if changed != 1 {
            return Err(SessionsApplicationError::Validation(
                deletion_error_code::REVISION_CONFLICT.to_string(),
            ));
        }
        Ok(next_revision)
    }

    fn update_group(
        &self,
        operation_id: &str,
        group_id: &str,
        expected_revision: u64,
        patch: &GroupPatch,
    ) -> Result<u64, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(repository_error)?;
        let revision = apply_group_patch(
            &transaction,
            operation_id,
            group_id,
            expected_revision,
            patch,
            &self.clock.now(),
        )?;
        transaction.commit().map_err(repository_error)?;
        Ok(revision)
    }

    fn group_snapshot(
        &self,
        operation_id: &str,
        group_id: &str,
    ) -> Result<Option<GroupSnapshot>, SessionsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT execution_snapshot, receipt, authorization FROM session_deletion_groups WHERE operation_id = ?1 AND id = ?2",
                params![operation_id, group_id],
                |row| {
                    Ok(GroupSnapshot {
                        execution_snapshot: json_value(row.get(0)?),
                        receipt: json_value(row.get(1)?),
                        authorization: json_value(row.get(2)?),
                    })
                },
            )
            .optional()
            .map_err(repository_error)
    }

    fn complete_group_deleting_sessions(
        &self,
        operation_id: &str,
        group_id: &str,
        expected_revision: u64,
        session_ids: &[String],
    ) -> Result<GroupCompletion, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(repository_error)?;
        let mut active_session_cleared = false;
        for session_id in session_ids {
            // A row already gone is not a failure: the row is what this step is for.
            transaction
                .execute("DELETE FROM sessions WHERE id = ?1", [session_id])
                .map_err(repository_error)?;
            let cleared = transaction
                .execute(
                    "UPDATE workflow_state SET active_session_id = NULL WHERE id = 1 AND active_session_id = ?1",
                    [session_id],
                )
                .map_err(repository_error)?;
            active_session_cleared |= cleared > 0;
        }
        let revision = apply_group_patch(
            &transaction,
            operation_id,
            group_id,
            expected_revision,
            &GroupPatch {
                status: Some(DeletionGroupStatus::Succeeded),
                phase: Some(DeletionPhase::Completed),
                db_effect: Some(SessionDbEffect::Deleted),
                error_code: Some(None),
                ..GroupPatch::default()
            },
            &self.clock.now(),
        )?;
        transaction
            .execute(
                "DELETE FROM session_deletion_claims WHERE operation_id = ?1 AND group_id = ?2",
                params![operation_id, group_id],
            )
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(GroupCompletion {
            revision,
            active_session_cleared,
        })
    }

    fn release_group_claims(
        &self,
        operation_id: &str,
        group_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM session_deletion_claims WHERE operation_id = ?1 AND group_id = ?2",
                params![operation_id, group_id],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn reclaim_group(
        &self,
        operation_id: &str,
        group_id: &str,
        session_ids: &[String],
    ) -> Result<Option<SessionDeletionClaim>, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(repository_error)?;
        for session_id in session_ids {
            let existing: Option<(String, String)> = transaction
                .query_row(
                    "SELECT operation_id, group_id FROM session_deletion_claims WHERE session_id = ?1",
                    [session_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(repository_error)?;
            if let Some((other_operation, other_group)) = existing {
                if other_operation != operation_id {
                    return Ok(Some(SessionDeletionClaim {
                        session_id: session_id.clone(),
                        operation_id: other_operation,
                        group_id: other_group,
                    }));
                }
            }
        }
        let now = self.clock.now();
        for session_id in session_ids {
            transaction
                .execute(
                    "INSERT OR REPLACE INTO session_deletion_claims (session_id, operation_id, group_id, claimed_at) VALUES (?1, ?2, ?3, ?4)",
                    params![session_id, operation_id, group_id, now],
                )
                .map_err(repository_error)?;
        }
        transaction.commit().map_err(repository_error)?;
        Ok(None)
    }

    fn active_claim(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionDeletionClaim>, SessionsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT session_id, operation_id, group_id FROM session_deletion_claims WHERE session_id = ?1",
                [session_id],
                |row| {
                    Ok(SessionDeletionClaim {
                        session_id: row.get(0)?,
                        operation_id: row.get(1)?,
                        group_id: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(repository_error)
    }
}

fn apply_group_patch(
    connection: &Connection,
    operation_id: &str,
    group_id: &str,
    expected_revision: u64,
    patch: &GroupPatch,
    now: &str,
) -> Result<u64, SessionsApplicationError> {
    let current = connection
        .query_row(
            &format!("SELECT {GROUP_COLUMNS} FROM session_deletion_groups WHERE operation_id = ?1 AND id = ?2"),
            params![operation_id, group_id],
            read_group,
        )
        .optional()
        .map_err(repository_error)?
        .ok_or_else(|| SessionsApplicationError::Validation(deletion_error_code::OPERATION_NOT_FOUND.to_string()))?;
    let next_revision = expected_revision + 1;
    let status = patch.status.unwrap_or(current.status);
    let phase = patch.phase.unwrap_or(current.phase);
    let worktree_effect = patch.worktree_effect.unwrap_or(current.worktree_effect);
    let db_effect = patch.db_effect.unwrap_or(current.db_effect);
    let error_code = match &patch.error_code {
        Some(code) => code.clone(),
        None => current.error_code,
    };
    let attempt = patch.attempt.unwrap_or(current.attempt);
    let policy = patch.policy.unwrap_or(current.policy);
    let mut sql = String::from(
        "UPDATE session_deletion_groups SET status = ?3, phase = ?4, worktree_effect = ?5, db_effect = ?6, error_code = ?7, attempt = ?8, policy = ?9, revision = ?10, updated_at = ?11",
    );
    if patch.execution_snapshot.is_some() {
        sql.push_str(", execution_snapshot = ?12");
    }
    if patch.receipt.is_some() {
        sql.push_str(", receipt = ?13");
    }
    if patch.authorization.is_some() {
        sql.push_str(", authorization = ?14");
    }
    sql.push_str(" WHERE operation_id = ?1 AND id = ?2 AND revision = ?15");
    let changed = connection
        .execute(
            &sql,
            params![
                operation_id,
                group_id,
                status.as_str(),
                phase.as_str(),
                worktree_effect.as_str(),
                db_effect.as_str(),
                error_code,
                i64::from(attempt),
                policy.as_str(),
                i64::try_from(next_revision).unwrap_or(i64::MAX),
                now,
                json_text(&patch.execution_snapshot),
                json_text(&patch.receipt),
                json_text(&patch.authorization),
                i64::try_from(expected_revision).unwrap_or(i64::MAX),
            ],
        )
        .map_err(repository_error)?;
    if changed != 1 {
        return Err(SessionsApplicationError::Validation(
            deletion_error_code::REVISION_CONFLICT.to_string(),
        ));
    }
    Ok(next_revision)
}

fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}
