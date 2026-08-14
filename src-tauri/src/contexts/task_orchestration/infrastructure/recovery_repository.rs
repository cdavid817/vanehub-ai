use super::repository::{storage_error, SqlitePlanRepository};
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::domain::recovery::RecoveryDecision;
use crate::contexts::task_orchestration::application::PlanApplicationError;
use crate::contexts::task_orchestration::domain::PlanRunStatus;
use rusqlite::{params, OptionalExtension, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryTerminal {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecoveryEvidence {
    pub(crate) session: Option<RecoveryTerminal>,
    pub(crate) operation: Option<RecoveryTerminal>,
}

pub(crate) trait RecoveryEvidenceGateway: Send + Sync {
    fn inspect(
        &self,
        session_id: Option<&str>,
        execution_run_id: Option<&str>,
        operation_id: Option<&str>,
    ) -> RecoveryEvidence;
}

pub(crate) struct NativeRecoveryEvidenceGateway {
    sessions: SessionsApi,
}

impl NativeRecoveryEvidenceGateway {
    pub(crate) fn new(sessions: SessionsApi, _operations: OperationsApi) -> Self {
        Self { sessions }
    }
}

impl RecoveryEvidenceGateway for NativeRecoveryEvidenceGateway {
    fn inspect(
        &self,
        session_id: Option<&str>,
        execution_run_id: Option<&str>,
        _operation_id: Option<&str>,
    ) -> RecoveryEvidence {
        let session = session_id
            .and_then(|id| self.sessions.recovery_projection(id, execution_run_id).ok())
            .and_then(|projection| match projection.decision {
                Some(RecoveryDecision::Completed) => Some(RecoveryTerminal::Succeeded),
                Some(
                    RecoveryDecision::Failed | RecoveryDecision::InterruptedWithoutToolAmbiguity,
                ) => Some(RecoveryTerminal::Failed),
                Some(RecoveryDecision::Cancelled) => Some(RecoveryTerminal::Cancelled),
                Some(
                    RecoveryDecision::ActionRequired
                    | RecoveryDecision::Quarantined
                    | RecoveryDecision::RetryLater
                    | RecoveryDecision::Acknowledged,
                )
                | None => None,
            });
        RecoveryEvidence {
            session,
            operation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryResolution {
    Succeeded,
    Interrupted,
    Failed,
    Cancelled,
}

#[derive(Debug)]
struct ActiveAttempt {
    task_run_id: String,
    attempt_id: Option<String>,
    session_id: Option<String>,
    execution_run_id: Option<String>,
    operation_id: Option<String>,
}

impl SqlitePlanRepository {
    pub(crate) fn recover_ambiguous_inflight(
        &self,
        evidence: &dyn RecoveryEvidenceGateway,
        now: &str,
    ) -> Result<Vec<String>, PlanApplicationError> {
        let runs = self.recovery_runs()?;
        for (run_id, status) in &runs {
            if status == PlanRunStatus::CancelRequested.as_str() {
                self.settle_cancelled_restart(run_id, now)?;
                continue;
            }
            let attempts = self.active_recovery_attempts(run_id)?;
            if status == PlanRunStatus::PauseRequested.as_str() && attempts.is_empty() {
                self.settle_paused_restart(run_id, now)?;
                continue;
            }
            if status == PlanRunStatus::Preparing.as_str() || attempts.is_empty() {
                self.mark_preparation_ambiguous(run_id, now)?;
                continue;
            }
            let resolutions = attempts
                .iter()
                .map(|attempt| {
                    resolve_evidence(evidence.inspect(
                        attempt.session_id.as_deref(),
                        attempt.execution_run_id.as_deref(),
                        attempt.operation_id.as_deref(),
                    ))
                })
                .collect::<Vec<_>>();
            self.apply_recovery_resolutions(run_id, &attempts, &resolutions, now)?;
        }
        Ok(runs.into_iter().map(|(id, _)| id).collect())
    }

    fn recovery_runs(&self) -> Result<Vec<(String, String)>, PlanApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"SELECT DISTINCT run.id, run.status
                   FROM plan_runs AS run
                   LEFT JOIN plan_subtask_runs AS task
                     ON task.plan_run_id = run.id
                    AND task.status IN ('dispatching', 'running', 'verifying')
                   WHERE run.status IN ('preparing', 'running', 'pause_requested', 'cancel_requested')
                     AND (run.status IN ('preparing', 'pause_requested', 'cancel_requested')
                          OR task.id IS NOT NULL)
                   ORDER BY run.id"#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn active_recovery_attempts(
        &self,
        run_id: &str,
    ) -> Result<Vec<ActiveAttempt>, PlanApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"SELECT task.id, attempt.id, attempt.session_id,
                          attempt.execution_run_id, attempt.operation_id
                   FROM plan_subtask_runs AS task
                   LEFT JOIN plan_subtask_attempts AS attempt
                     ON attempt.subtask_run_id = task.id
                    AND attempt.status IN ('dispatching', 'running', 'verifying')
                   WHERE task.plan_run_id = ?1
                     AND task.status IN ('dispatching', 'running', 'verifying')
                   ORDER BY task.topological_rank, task.ordinal, task.id, attempt.sequence DESC"#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([run_id], |row| {
                Ok(ActiveAttempt {
                    task_run_id: row.get(0)?,
                    attempt_id: row.get(1)?,
                    session_id: row.get(2)?,
                    execution_run_id: row.get(3)?,
                    operation_id: row.get(4)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn apply_recovery_resolutions(
        &self,
        run_id: &str,
        attempts: &[ActiveAttempt],
        resolutions: &[RecoveryResolution],
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        for (attempt, resolution) in attempts.iter().zip(resolutions) {
            let (status, error_class) = match resolution {
                RecoveryResolution::Succeeded => ("succeeded", "restart_reconciled_succeeded"),
                RecoveryResolution::Interrupted => ("interrupted", "restart_interrupted"),
                RecoveryResolution::Failed => ("failed", "restart_reconciled_failed"),
                RecoveryResolution::Cancelled => ("cancelled", "restart_reconciled_cancelled"),
            };
            if let Some(attempt_id) = &attempt.attempt_id {
                transaction
                    .execute(
                        r#"UPDATE plan_subtask_attempts SET status = ?2, error_class = ?3,
                                  completed_at = ?4
                           WHERE id = ?1 AND status IN ('dispatching', 'running', 'verifying')"#,
                        params![attempt_id, status, error_class, now],
                    )
                    .map_err(storage_error)?;
            }
            transaction
                .execute(
                    r#"UPDATE plan_subtask_runs SET status = ?2, updated_at = ?3, completed_at = ?3
                       WHERE id = ?1 AND status IN ('dispatching', 'running', 'verifying')"#,
                    params![attempt.task_run_id, status, now],
                )
                .map_err(storage_error)?;
        }
        let run_resolution = if resolutions.contains(&RecoveryResolution::Interrupted) {
            Some(RecoveryResolution::Interrupted)
        } else if resolutions.contains(&RecoveryResolution::Cancelled) {
            Some(RecoveryResolution::Cancelled)
        } else if resolutions.contains(&RecoveryResolution::Failed) {
            Some(RecoveryResolution::Failed)
        } else {
            None
        };
        if let Some(run_resolution) = run_resolution {
            apply_run_resolution(&transaction, run_id, run_resolution, now)?;
        }
        transaction.commit().map_err(storage_error)
    }

    fn settle_cancelled_restart(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                r#"UPDATE plan_subtask_attempts SET status = 'cancelled',
                          error_class = 'restart_reconciled_cancelled', completed_at = ?2
                   WHERE subtask_run_id IN (SELECT id FROM plan_subtask_runs WHERE plan_run_id = ?1)
                     AND status IN ('dispatching', 'running', 'verifying')"#,
                params![run_id, now],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"UPDATE plan_subtask_runs SET status = 'cancelled', updated_at = ?2, completed_at = ?2
                   WHERE plan_run_id = ?1
                     AND status IN ('pending', 'ready', 'dispatching', 'running', 'verifying')"#,
                params![run_id, now],
            )
            .map_err(storage_error)?;
        apply_run_resolution(&transaction, run_id, RecoveryResolution::Cancelled, now)?;
        transaction.commit().map_err(storage_error)
    }

    fn settle_paused_restart(&self, run_id: &str, now: &str) -> Result<(), PlanApplicationError> {
        self.connection()?
            .execute(
                "UPDATE plan_runs SET status = 'paused', updated_at = ?2 WHERE id = ?1 AND status = 'pause_requested'",
                params![run_id, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn mark_preparation_ambiguous(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        self.connection()?
            .execute(
                r#"UPDATE plan_runs SET status = 'recovery_required', updated_at = ?2
                   WHERE id = ?1 AND status IN ('preparing', 'running', 'pause_requested')"#,
                params![run_id, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(crate) fn recover_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<String, PlanApplicationError> {
        PlanRunStatus::RecoveryRequired
            .transition(PlanRunStatus::Paused)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let interrupted_id: String = transaction
            .query_row(
                r#"SELECT id FROM plan_subtask_runs
                   WHERE plan_run_id = ?1 AND status = 'interrupted'
                   ORDER BY topological_rank, ordinal, id LIMIT 1"#,
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::Conflict)?;
        let changed = transaction
            .execute(
                r#"UPDATE plan_runs SET status = 'paused', updated_at = ?2
                   WHERE id = ?1 AND status = 'recovery_required'"#,
                params![run_id, now],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction
            .execute(
                "UPDATE plan_subtask_runs SET status = 'ready', completed_at = NULL, updated_at = ?2 WHERE id = ?1 AND status = 'interrupted'",
                params![interrupted_id, now],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE plan_subtask_runs SET status = 'pending', updated_at = ?2 WHERE plan_run_id = ?1 AND status = 'blocked'",
                params![run_id, now],
            )
            .map_err(storage_error)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        transaction
            .execute(
                r#"INSERT INTO plan_control_requests
                   (id, plan_run_id, kind, status, requested_at, resolved_at)
                   VALUES (?1, ?2, 'recover', 'completed', ?3, ?3)"#,
                params![request_id, run_id, now],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(request_id)
    }
}

fn resolve_evidence(evidence: RecoveryEvidence) -> RecoveryResolution {
    match (evidence.session, evidence.operation) {
        (Some(left), Some(right)) if left != right => RecoveryResolution::Interrupted,
        (Some(RecoveryTerminal::Failed), None)
        | (None, Some(RecoveryTerminal::Failed))
        | (Some(RecoveryTerminal::Failed), Some(RecoveryTerminal::Failed)) => {
            RecoveryResolution::Failed
        }
        (Some(RecoveryTerminal::Cancelled), None)
        | (None, Some(RecoveryTerminal::Cancelled))
        | (Some(RecoveryTerminal::Cancelled), Some(RecoveryTerminal::Cancelled)) => {
            RecoveryResolution::Cancelled
        }
        (Some(RecoveryTerminal::Succeeded), None)
        | (None, Some(RecoveryTerminal::Succeeded))
        | (Some(RecoveryTerminal::Succeeded), Some(RecoveryTerminal::Succeeded)) => {
            RecoveryResolution::Succeeded
        }
        _ => RecoveryResolution::Interrupted,
    }
}

fn apply_run_resolution(
    transaction: &Transaction<'_>,
    run_id: &str,
    resolution: RecoveryResolution,
    now: &str,
) -> Result<(), PlanApplicationError> {
    let (status, completed_at) = match resolution {
        RecoveryResolution::Succeeded => {
            return Err(PlanApplicationError::Validation(
                "successful recovery leaves the PlanRun active for scheduling".to_string(),
            ));
        }
        RecoveryResolution::Interrupted => ("recovery_required", None),
        RecoveryResolution::Failed => ("failed", Some(now)),
        RecoveryResolution::Cancelled => ("cancelled", Some(now)),
    };
    transaction
        .execute(
            r#"UPDATE plan_runs SET status = ?2, updated_at = ?3, completed_at = ?4
               WHERE id = ?1
                 AND status IN ('preparing', 'running', 'pause_requested', 'cancel_requested')"#,
            params![run_id, status, now, completed_at],
        )
        .map_err(storage_error)?;
    Ok(())
}
