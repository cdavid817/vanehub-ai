use super::repository::{storage_error, SqlitePlanRepository};
use crate::contexts::task_orchestration::application::PlanApplicationError;
use crate::contexts::task_orchestration::domain::PlanRunStatus;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlRequestResult {
    pub(crate) run_status: PlanRunStatus,
    pub(crate) active_session_id: Option<String>,
}

impl SqlitePlanRepository {
    pub(crate) fn run_status(&self, run_id: &str) -> Result<PlanRunStatus, PlanApplicationError> {
        let status: String = self
            .connection()?
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        PlanRunStatus::parse(&status)
            .ok_or_else(|| PlanApplicationError::Storage("unknown PlanRun status".to_string()))
    }

    pub(crate) fn latest_control_id(&self, run_id: &str) -> Result<String, PlanApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT id FROM plan_control_requests WHERE plan_run_id = ?1
                   ORDER BY requested_at DESC, id DESC LIMIT 1"#,
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)
    }

    pub(crate) fn request_pause(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<ControlRequestResult, PlanApplicationError> {
        self.persist_control_transition(
            run_id,
            "pause",
            &[PlanRunStatus::Running],
            PlanRunStatus::PauseRequested,
            now,
        )?;
        if self.active_session(run_id)?.is_none() {
            self.settle_control_boundary(run_id, now)?;
        }
        self.control_result(run_id)
    }

    pub(crate) fn resume_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<ControlRequestResult, PlanApplicationError> {
        self.persist_control_transition(
            run_id,
            "resume",
            &[PlanRunStatus::Paused],
            PlanRunStatus::Running,
            now,
        )?;
        self.resolve_latest_control(run_id, "resume", now)?;
        self.control_result(run_id)
    }

    pub(crate) fn request_cancel(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<ControlRequestResult, PlanApplicationError> {
        self.persist_control_transition(
            run_id,
            "cancel",
            &[
                PlanRunStatus::Running,
                PlanRunStatus::PauseRequested,
                PlanRunStatus::Paused,
                PlanRunStatus::RecoveryRequired,
            ],
            PlanRunStatus::CancelRequested,
            now,
        )?;
        let active_session_id = self.active_session(run_id)?;
        if active_session_id.is_none() {
            self.settle_control_boundary(run_id, now)?;
        }
        let mut result = self.control_result(run_id)?;
        result.active_session_id = active_session_id;
        Ok(result)
    }

    pub(crate) fn settle_control_boundary(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanRunStatus, PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let current: String = transaction
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        let current = PlanRunStatus::parse(&current)
            .ok_or_else(|| PlanApplicationError::Storage("unknown PlanRun status".to_string()))?;
        let (next, kind) = match current {
            PlanRunStatus::PauseRequested => (PlanRunStatus::Paused, "pause"),
            PlanRunStatus::CancelRequested => (PlanRunStatus::Cancelled, "cancel"),
            other => return Ok(other),
        };
        current
            .transition(next)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        if next == PlanRunStatus::Cancelled {
            transaction
                .execute(
                    r#"UPDATE plan_subtask_runs SET status = 'cancelled', updated_at = ?2,
                              completed_at = ?2
                       WHERE plan_run_id = ?1
                         AND status IN ('pending', 'ready', 'dispatching', 'running', 'verifying')"#,
                    params![run_id, now],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    r#"UPDATE plan_subtask_attempts SET status = 'cancelled',
                              error_class = 'cancelled', completed_at = ?2
                       WHERE subtask_run_id IN (
                           SELECT id FROM plan_subtask_runs WHERE plan_run_id = ?1
                       ) AND status IN ('dispatching', 'running', 'verifying')"#,
                    params![run_id, now],
                )
                .map_err(storage_error)?;
        }
        transaction
            .execute(
                r#"UPDATE plan_runs SET status = ?2, updated_at = ?3,
                          completed_at = CASE WHEN ?2 = 'cancelled' THEN ?3 ELSE completed_at END
                   WHERE id = ?1 AND status = ?4"#,
                params![run_id, next.as_str(), now, current.as_str()],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"UPDATE plan_control_requests SET status = 'completed', resolved_at = ?3
                   WHERE id = (
                       SELECT id FROM plan_control_requests
                       WHERE plan_run_id = ?1 AND kind = ?2 AND status = 'pending'
                       ORDER BY requested_at DESC, id DESC LIMIT 1
                   )"#,
                params![run_id, kind, now],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(next)
    }

    pub(crate) fn retry_subtask(
        &self,
        run_id: &str,
        subtask_run_id: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let run_status: String = transaction
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        let run_status = PlanRunStatus::parse(&run_status)
            .ok_or_else(|| PlanApplicationError::Storage("unknown PlanRun status".to_string()))?;
        if !matches!(
            run_status,
            PlanRunStatus::Failed | PlanRunStatus::RecoveryRequired
        ) {
            return Err(PlanApplicationError::Conflict);
        }
        run_status
            .transition(PlanRunStatus::Running)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let changed = transaction
            .execute(
                r#"UPDATE plan_subtask_runs SET status = 'ready', updated_at = ?3,
                          completed_at = NULL
                   WHERE id = ?1 AND plan_run_id = ?2 AND status IN ('failed', 'interrupted')"#,
                params![subtask_run_id, run_id, now],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction
            .execute(
                "UPDATE plan_subtask_runs SET status = 'pending', updated_at = ?2 WHERE plan_run_id = ?1 AND status = 'blocked'",
                params![run_id, now],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE plan_runs SET status = 'running', updated_at = ?2, completed_at = NULL WHERE id = ?1 AND status = ?3",
                params![run_id, now, run_status.as_str()],
            )
            .map_err(storage_error)?;
        insert_control(&transaction, run_id, "retry", "completed", now, Some(now))?;
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn accept_run(&self, run_id: &str, now: &str) -> Result<(), PlanApplicationError> {
        PlanRunStatus::AwaitingAcceptance
            .transition(PlanRunStatus::Completed)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let changed = transaction
            .execute(
                "UPDATE plan_runs SET status = 'completed', updated_at = ?2, completed_at = ?2 WHERE id = ?1 AND status = 'awaiting_acceptance'",
                params![run_id, now],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        insert_control(&transaction, run_id, "accept", "completed", now, Some(now))?;
        transaction.commit().map_err(storage_error)
    }

    fn persist_control_transition(
        &self,
        run_id: &str,
        kind: &str,
        allowed: &[PlanRunStatus],
        next: PlanRunStatus,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let current: String = transaction
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        let current = PlanRunStatus::parse(&current)
            .ok_or_else(|| PlanApplicationError::Storage("unknown PlanRun status".to_string()))?;
        if !allowed.contains(&current) {
            return Err(PlanApplicationError::Conflict);
        }
        current
            .transition(next)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        insert_control(&transaction, run_id, kind, "pending", now, None)?;
        let changed = transaction
            .execute(
                "UPDATE plan_runs SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = ?4",
                params![run_id, next.as_str(), now, current.as_str()],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.commit().map_err(storage_error)
    }

    fn active_session(&self, run_id: &str) -> Result<Option<String>, PlanApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT attempt.session_id
                   FROM plan_subtask_attempts AS attempt
                   JOIN plan_subtask_runs AS task_run ON task_run.id = attempt.subtask_run_id
                   WHERE task_run.plan_run_id = ?1
                     AND attempt.status IN ('dispatching', 'running', 'verifying')
                   ORDER BY attempt.started_at DESC, attempt.sequence DESC LIMIT 1"#,
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)
    }

    fn control_result(&self, run_id: &str) -> Result<ControlRequestResult, PlanApplicationError> {
        let status: String = self
            .connection()?
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        Ok(ControlRequestResult {
            run_status: PlanRunStatus::parse(&status).ok_or_else(|| {
                PlanApplicationError::Storage("unknown PlanRun status".to_string())
            })?,
            active_session_id: self.active_session(run_id)?,
        })
    }

    fn resolve_latest_control(
        &self,
        run_id: &str,
        kind: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        self.connection()?
            .execute(
                r#"UPDATE plan_control_requests SET status = 'completed', resolved_at = ?3
                   WHERE id = (
                       SELECT id FROM plan_control_requests
                       WHERE plan_run_id = ?1 AND kind = ?2 AND status = 'pending'
                       ORDER BY requested_at DESC, id DESC LIMIT 1
                   )"#,
                params![run_id, kind, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn insert_control(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
    kind: &str,
    status: &str,
    requested_at: &str,
    resolved_at: Option<&str>,
) -> Result<(), PlanApplicationError> {
    transaction
        .execute(
            r#"INSERT INTO plan_control_requests
               (id, plan_run_id, kind, status, requested_at, resolved_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![
                Uuid::new_v4().to_string(),
                run_id,
                kind,
                status,
                requested_at,
                resolved_at
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}
