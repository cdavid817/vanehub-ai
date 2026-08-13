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

    pub(crate) fn set_driver_intent(
        &self,
        run_id: &str,
        intent: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        if !matches!(intent, "stopped" | "run" | "pause" | "cancel") {
            return Err(PlanApplicationError::Validation(
                "invalid Plan driver intent".to_string(),
            ));
        }
        let changed = self
            .connection()?
            .execute(
                "UPDATE plan_runs SET driver_intent = ?2, updated_at = ?3 WHERE id = ?1",
                params![run_id, intent, now],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::NotFound);
        }
        Ok(())
    }

    pub(crate) fn runnable_driver_run_ids(&self) -> Result<Vec<String>, PlanApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id FROM plan_runs WHERE status = 'running' AND driver_intent = 'run' ORDER BY created_at, id",
            )
            .map_err(storage_error)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(ids)
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
            &[
                PlanRunStatus::Running,
                PlanRunStatus::Verifying,
                PlanRunStatus::Repairing,
            ],
            PlanRunStatus::PauseRequested,
            now,
        )?;
        self.set_driver_intent(run_id, "pause", now)?;
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
        self.set_driver_intent(run_id, "run", now)?;
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
                PlanRunStatus::Verifying,
                PlanRunStatus::Repairing,
                PlanRunStatus::PauseRequested,
                PlanRunStatus::Paused,
                PlanRunStatus::RecoveryRequired,
                PlanRunStatus::ActionRequired,
                PlanRunStatus::FinalVerifying,
            ],
            PlanRunStatus::CancelRequested,
            now,
        )?;
        self.set_driver_intent(run_id, "cancel", now)?;
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
                    r#"UPDATE plan_final_repair_attempts SET status = 'cancelled',
                              error_class = 'cancelled', completed_at = ?2
                       WHERE finalization_id IN (
                           SELECT id FROM plan_finalizations WHERE plan_run_id = ?1
                       ) AND status IN ('dispatching', 'running')"#,
                    params![run_id, now],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    r#"UPDATE plan_finalizations SET status = 'cancelled', completed_at = ?2
                       WHERE plan_run_id = ?1 AND status = 'running'"#,
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
            PlanRunStatus::Failed
                | PlanRunStatus::RecoveryRequired
                | PlanRunStatus::ActionRequired
                | PlanRunStatus::Running
        ) {
            return Err(PlanApplicationError::Conflict);
        }
        if run_status != PlanRunStatus::Running {
            run_status
                .transition(PlanRunStatus::Running)
                .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        }
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

    pub(crate) fn auto_retry_failed_attempt(
        &self,
        run_id: &str,
        attempt_id: &str,
        now: &str,
    ) -> Result<bool, PlanApplicationError> {
        let connection = self.connection()?;
        let candidate = connection
            .query_row(
                r#"SELECT task_run.id, attempt.error_class, attempt.sequence,
                          policy.max_attempts_per_subtask, policy.repair_eligible_classes
                   FROM plan_subtask_attempts AS attempt
                   JOIN plan_subtask_runs AS task_run ON task_run.id = attempt.subtask_run_id
                   JOIN plan_run_policies AS policy ON policy.plan_run_id = task_run.plan_run_id
                   WHERE attempt.id = ?1 AND task_run.plan_run_id = ?2
                     AND attempt.status = 'failed' AND task_run.status = 'failed'"#,
                params![attempt_id, run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, u16>(2)?,
                        row.get::<_, u16>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?;
        drop(connection);
        let Some((subtask_run_id, error_class, sequence, maximum, raw_classes)) = candidate else {
            return Ok(false);
        };
        let classes: Vec<String> = serde_json::from_str(&raw_classes).map_err(storage_error)?;
        if sequence >= maximum
            || !error_class
                .as_ref()
                .is_some_and(|value| classes.iter().any(|class| class == value))
        {
            return Ok(false);
        }
        self.retry_subtask(run_id, &subtask_run_id, now)?;
        Ok(true)
    }

    pub(crate) fn retry_final_verification(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let candidate = transaction
            .query_row(
                r#"SELECT COUNT(finalization.id), policy.max_attempts_per_subtask
                   FROM plan_runs AS run
                   JOIN plan_run_policies AS policy ON policy.plan_run_id = run.id
                   JOIN plan_finalizations AS finalization ON finalization.plan_run_id = run.id
                   WHERE run.id = ?1 AND run.status = 'action_required'
                     AND finalization.status = 'failed'
                     AND NOT EXISTS (
                         SELECT 1 FROM plan_subtask_runs AS task
                         WHERE task.plan_run_id = run.id AND task.status != 'succeeded'
                     )"#,
                [run_id],
                |row| Ok((row.get::<_, u16>(0)?, row.get::<_, u16>(1)?)),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::Conflict)?;
        if candidate.0 == 0 || candidate.0 >= candidate.1 {
            return Err(PlanApplicationError::Conflict);
        }
        PlanRunStatus::ActionRequired
            .transition(PlanRunStatus::Running)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let changed = transaction
            .execute(
                r#"UPDATE plan_runs SET status = 'running', driver_intent = 'run',
                          updated_at = ?2, completed_at = NULL
                   WHERE id = ?1 AND status = 'action_required'"#,
                params![run_id, now],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
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
                r#"SELECT session_id FROM (
                       SELECT attempt.session_id, attempt.started_at
                       FROM plan_subtask_attempts AS attempt
                       JOIN plan_subtask_runs AS task_run ON task_run.id = attempt.subtask_run_id
                       WHERE task_run.plan_run_id = ?1
                         AND attempt.status IN ('dispatching', 'running', 'verifying')
                       UNION ALL
                       SELECT repair.session_id, repair.started_at
                       FROM plan_final_repair_attempts AS repair
                       JOIN plan_finalizations AS finalization
                         ON finalization.id = repair.finalization_id
                       WHERE finalization.plan_run_id = ?1
                         AND repair.status IN ('dispatching', 'running')
                   ) WHERE session_id IS NOT NULL
                   ORDER BY started_at DESC LIMIT 1"#,
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
