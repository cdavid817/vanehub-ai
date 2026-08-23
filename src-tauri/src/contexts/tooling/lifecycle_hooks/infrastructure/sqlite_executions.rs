// The dispatch engine that appends these lands with Task Group 7; see `sqlite_definitions.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! The SQLite adapter for execution evidence.
//!
//! Append and prune, and nothing else. There is no update and no delete-by-id, so "an execution
//! row was edited afterwards" is not a thing this adapter can be asked to do.
//!
//! `sequence` is assigned here rather than by the caller: it is `MAX + 1` per subject, read and
//! written inside one write transaction, because only storage can see the other writers. A caller
//! choosing its own number would have to read, decide, and write across a window in which another
//! dispatch does the same, and the unique index would then reject one of them at random.
//!
//! Pruning keeps the newest `keep` rows whatever their status, and removes **only terminal rows**
//! from what is left. Two consequences, both intended: an unfinished execution is never deleted no
//! matter how old, and a subject that has ever run always retains at least one row — which is what
//! keeps `MAX + 1` from reissuing a sequence a previous execution already used.

use crate::contexts::tooling::lifecycle_hooks::application::HookExecutionRepository;
use crate::contexts::tooling::lifecycle_hooks::domain::{
    HookExecutionError, HookExecutionId, HookExecutionRecord, HookExecutionRetention,
    HookExecutionStatus, HookGlobalId, HookOutcomeCode,
};
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

use super::is_foreign_key_violation;

pub(crate) struct SqliteHookExecutionRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteHookExecutionRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, HookExecutionError> {
        self.database
            .connection()
            .map_err(|error| HookExecutionError::Storage(error.to_string()))
    }
}

/// A foreign-key failure is the database saying no such subject; a uniqueness failure on the
/// primary key is a re-append of an id that is already recorded. Both are domain answers rather
/// than storage failures.
fn execution_error(error: rusqlite::Error) -> HookExecutionError {
    if is_foreign_key_violation(&error) {
        return HookExecutionError::UnknownSubject;
    }
    let text = error.to_string();
    if text.contains("UNIQUE constraint failed: lifecycle_hook_executions.execution_id") {
        return HookExecutionError::DuplicateExecution;
    }
    HookExecutionError::Storage(text)
}

type ExecutionRow = (
    String,
    i64,
    String,
    Option<String>,
    Option<i64>,
    String,
    Option<String>,
);

/// Rebuilds a record from a row, refusing one whose stored values no longer parse.
fn read_execution(
    hook: &HookGlobalId,
    row: ExecutionRow,
) -> Result<HookExecutionRecord, HookExecutionError> {
    let (execution, sequence, status, outcome, duration_ms, started_at, finished_at) = row;
    let storage = |code: &str| HookExecutionError::Storage(code.to_string());
    Ok(HookExecutionRecord {
        execution: HookExecutionId::parse(&execution).map_err(|error| storage(error.code()))?,
        hook: hook.clone(),
        sequence,
        status: HookExecutionStatus::parse(&status)
            .ok_or_else(|| storage("invalid_hook_execution_status"))?,
        outcome: outcome
            .map(|code| HookOutcomeCode::parse(&code).map_err(|error| storage(error.code())))
            .transpose()?,
        duration_ms,
        started_at,
        finished_at,
    })
}

impl HookExecutionRepository for SqliteHookExecutionRepository {
    fn append(
        &self,
        record: &HookExecutionRecord,
    ) -> Result<HookExecutionRecord, HookExecutionError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| HookExecutionError::Storage(error.to_string()))?;

        // Read the high-water mark inside the transaction. A deferred transaction could not do
        // this: SQLite refuses the read-to-write lock upgrade without honouring `busy_timeout`,
        // so two concurrent appends would surface as an opaque "database is locked".
        let highest: Option<i64> = transaction
            .query_row(
                "SELECT MAX(sequence) FROM lifecycle_hook_executions WHERE hook_global_id = ?1",
                params![record.hook.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(execution_error)?
            .flatten();
        let sequence = highest.unwrap_or(0) + 1;

        transaction
            .execute(
                "INSERT INTO lifecycle_hook_executions \
                     (execution_id, hook_global_id, sequence, status, terminal, outcome_code, \
                      duration_ms, started_at, finished_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.execution.as_str(),
                    record.hook.as_str(),
                    sequence,
                    record.status.as_str(),
                    i64::from(record.status.is_terminal()),
                    record.outcome.as_ref().map(HookOutcomeCode::as_str),
                    record.duration_ms,
                    record.started_at,
                    record.finished_at,
                ],
            )
            .map_err(execution_error)?;
        transaction
            .commit()
            .map_err(|error| HookExecutionError::Storage(error.to_string()))?;

        Ok(HookExecutionRecord {
            sequence,
            ..record.clone()
        })
    }

    fn prune(
        &self,
        hook: &HookGlobalId,
        retention: HookExecutionRetention,
    ) -> Result<usize, HookExecutionError> {
        let connection = self.connection()?;
        let removed = connection
            .execute(
                "DELETE FROM lifecycle_hook_executions \
                 WHERE hook_global_id = ?1 \
                   AND terminal = 1 \
                   AND sequence NOT IN ( \
                       SELECT sequence FROM lifecycle_hook_executions \
                       WHERE hook_global_id = ?1 \
                       ORDER BY sequence DESC LIMIT ?2 \
                   )",
                params![
                    hook.as_str(),
                    i64::try_from(retention.keep()).unwrap_or(i64::MAX)
                ],
            )
            .map_err(execution_error)?;
        Ok(removed)
    }

    fn recent(
        &self,
        hook: &HookGlobalId,
        limit: usize,
    ) -> Result<Vec<HookExecutionRecord>, HookExecutionError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT execution_id, sequence, status, outcome_code, duration_ms, started_at, \
                        finished_at \
                 FROM lifecycle_hook_executions WHERE hook_global_id = ?1 \
                 ORDER BY sequence DESC LIMIT ?2",
            )
            .map_err(execution_error)?;
        let rows = statement
            .query_map(
                params![hook.as_str(), i64::try_from(limit).unwrap_or(i64::MAX)],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .map_err(execution_error)?;

        let mut records = Vec::new();
        for row in rows {
            records.push(read_execution(hook, row.map_err(execution_error)?)?);
        }
        Ok(records)
    }
}
