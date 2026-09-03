use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, EvolutionRunStatus,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use super::{
    sql_revisions, validate_lease_input, OrchestrationPersistenceError, OrchestrationRepository,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunLifecycleUpdate {
    pub(crate) run_id: String,
    pub(crate) status: EvolutionRunStatus,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpiredRunLease {
    pub(crate) run_id: String,
    pub(crate) status: EvolutionRunStatus,
    pub(crate) revision: u64,
    pub(crate) lease_owner: String,
    pub(crate) lease_expires_at_ms: i64,
}

impl OrchestrationRepository {
    pub(crate) fn heartbeat_run_lease(
        &self,
        run_id: &str,
        expected_revision: u64,
        lease_owner: &str,
        now_ms: i64,
        lease_expires_at_ms: i64,
    ) -> Result<RunLifecycleUpdate, OrchestrationPersistenceError> {
        validate_lease_input(run_id, lease_owner, now_ms, lease_expires_at_ms)?;
        let (expected, revision) = sql_revisions(expected_revision)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = connection
            .execute(
                "UPDATE evolution_runs SET lease_expires_at_ms=?1,revision=?2,updated_at_ms=?3
                 WHERE run_id=?4 AND revision=?5 AND lease_owner=?6
                 AND lease_expires_at_ms>?3 AND status IN
                 ('requested','waiting_idle','running','partial','cancel_requested','recovered')",
                params![
                    lease_expires_at_ms,
                    revision,
                    now_ms,
                    run_id,
                    expected,
                    lease_owner,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 0 {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        let status = load_status(&connection, run_id)?;
        Ok(RunLifecycleUpdate {
            run_id: run_id.into(),
            status,
            revision: expected_revision + 1,
        })
    }

    pub(crate) fn transition_run_status(
        &self,
        run_id: &str,
        expected_revision: u64,
        lease_owner: &str,
        next: EvolutionRunStatus,
        updated_at_ms: i64,
    ) -> Result<RunLifecycleUpdate, OrchestrationPersistenceError> {
        if !is_safe_identifier(lease_owner, 128) {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        self.transition(
            run_id,
            expected_revision,
            Some(lease_owner),
            next,
            updated_at_ms,
        )
    }

    pub(crate) fn request_run_cancellation(
        &self,
        run_id: &str,
        expected_revision: u64,
        requested_at_ms: i64,
    ) -> Result<RunLifecycleUpdate, OrchestrationPersistenceError> {
        self.transition(
            run_id,
            expected_revision,
            None,
            EvolutionRunStatus::CancelRequested,
            requested_at_ms,
        )
    }

    pub(crate) fn expired_run_leases(
        &self,
        now_ms: i64,
        limit: u16,
    ) -> Result<Vec<ExpiredRunLease>, OrchestrationPersistenceError> {
        if now_ms < 0 || limit == 0 || limit > 100 {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let mut statement = connection
            .prepare(
                "SELECT run_id,status,revision,lease_owner,lease_expires_at_ms
                 FROM evolution_runs WHERE lease_owner IS NOT NULL AND lease_expires_at_ms<=?1
                 AND status IN ('requested','waiting_idle','running','partial','cancel_requested')
                 ORDER BY lease_expires_at_ms,run_id LIMIT ?2",
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let rows = statement
            .query_map(params![now_ms, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        rows.map(|row| {
            let (run_id, status, revision, lease_owner, lease_expires_at_ms) =
                row.map_err(|_| OrchestrationPersistenceError::Storage)?;
            Ok(ExpiredRunLease {
                run_id,
                status: EvolutionRunStatus::from_persisted(&status)
                    .map_err(|_| OrchestrationPersistenceError::Corrupt)?,
                revision: u64::try_from(revision)
                    .map_err(|_| OrchestrationPersistenceError::Corrupt)?,
                lease_owner,
                lease_expires_at_ms,
            })
        })
        .collect()
    }

    pub(crate) fn recover_expired_run(
        &self,
        run_id: &str,
        expected_revision: u64,
        lease_owner: &str,
        now_ms: i64,
        lease_expires_at_ms: i64,
    ) -> Result<RunLifecycleUpdate, OrchestrationPersistenceError> {
        validate_lease_input(run_id, lease_owner, now_ms, lease_expires_at_ms)?;
        let (expected, revision) = sql_revisions(expected_revision)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = connection
            .execute(
                "UPDATE evolution_runs SET status='recovered',lease_owner=?1,
                 lease_expires_at_ms=?2,revision=?3,updated_at_ms=?4
                 WHERE run_id=?5 AND revision=?6 AND lease_expires_at_ms<=?4
                 AND status IN ('requested','waiting_idle','running','partial','cancel_requested')",
                params![
                    lease_owner,
                    lease_expires_at_ms,
                    revision,
                    now_ms,
                    run_id,
                    expected,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 0 {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        Ok(RunLifecycleUpdate {
            run_id: run_id.into(),
            status: EvolutionRunStatus::Recovered,
            revision: expected_revision + 1,
        })
    }

    fn transition(
        &self,
        run_id: &str,
        expected_revision: u64,
        required_owner: Option<&str>,
        next: EvolutionRunStatus,
        updated_at_ms: i64,
    ) -> Result<RunLifecycleUpdate, OrchestrationPersistenceError> {
        if !is_safe_identifier(run_id, 128) || updated_at_ms < 0 {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let (expected, revision) = sql_revisions(expected_revision)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let (current, stored_revision, lease_owner) = load_run_state(&transaction, run_id)?;
        if stored_revision != expected
            || required_owner.is_some_and(|owner| lease_owner.as_deref() != Some(owner))
        {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        if !current.can_transition_to(next) {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        let terminal = next.is_terminal();
        transaction
            .execute(
                "UPDATE evolution_runs SET status=?1,
                 cancel_requested_at_ms=CASE WHEN ?1='cancel_requested' THEN ?2 ELSE cancel_requested_at_ms END,
                 lease_owner=CASE WHEN ?3 THEN NULL ELSE lease_owner END,
                 lease_expires_at_ms=CASE WHEN ?3 THEN NULL ELSE lease_expires_at_ms END,
                 revision=?4,updated_at_ms=?2 WHERE run_id=?5 AND revision=?6",
                params![next.as_str(), updated_at_ms, terminal, revision, run_id, expected],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(RunLifecycleUpdate {
            run_id: run_id.into(),
            status: next,
            revision: expected_revision + 1,
        })
    }
}

fn load_status(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<EvolutionRunStatus, OrchestrationPersistenceError> {
    let status: String = connection
        .query_row(
            "SELECT status FROM evolution_runs WHERE run_id=?1",
            [run_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| OrchestrationPersistenceError::Storage)?
        .ok_or(OrchestrationPersistenceError::NotFound)?;
    EvolutionRunStatus::from_persisted(&status).map_err(|_| OrchestrationPersistenceError::Corrupt)
}

type StoredRunState = (EvolutionRunStatus, i64, Option<String>);

fn load_run_state(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<StoredRunState, OrchestrationPersistenceError> {
    let (status, revision, owner): (String, i64, Option<String>) = transaction
        .query_row(
            "SELECT status,revision,lease_owner FROM evolution_runs WHERE run_id=?1",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| OrchestrationPersistenceError::Storage)?
        .ok_or(OrchestrationPersistenceError::NotFound)?;
    Ok((
        EvolutionRunStatus::from_persisted(&status)
            .map_err(|_| OrchestrationPersistenceError::Corrupt)?,
        revision,
        owner,
    ))
}
