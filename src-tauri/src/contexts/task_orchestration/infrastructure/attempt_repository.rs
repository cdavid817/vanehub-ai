use super::repository::{storage_error, SqlitePlanRepository};
use crate::contexts::task_orchestration::application::{
    PlanApplicationError, PredecessorContextSource,
};
use crate::contexts::task_orchestration::domain::{
    ResourceLimits, SubTaskRunStatus, SubTaskSpec, VerificationCommand,
};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptDispatch {
    pub(crate) plan_run_id: String,
    pub(crate) subtask_run_id: String,
    pub(crate) task: SubTaskSpec,
    pub(crate) project_path: String,
    pub(crate) worktree_path: String,
    pub(crate) profile_id: String,
    pub(crate) direct_predecessor_ids: Vec<String>,
    pub(crate) predecessor_sources: Vec<PredecessorContextSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptRecord {
    pub(crate) id: String,
    pub(crate) sequence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptTerminalUpdate {
    pub(crate) result_summary: Option<String>,
    pub(crate) changed_files: Vec<String>,
    pub(crate) token_usage: u32,
    pub(crate) tool_call_count: u32,
    pub(crate) error_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptVerificationDispatch {
    pub(crate) attempt_id: String,
    pub(crate) plan_run_id: String,
    pub(crate) subtask_run_id: String,
    pub(crate) worktree_path: String,
    pub(crate) commands: Vec<VerificationCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerificationEvidenceUpdate {
    pub(crate) command_id: String,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) output_summary: Option<String>,
}

impl SqlitePlanRepository {
    pub(crate) fn load_attempt_dispatch(
        &self,
        subtask_run_id: &str,
    ) -> Result<AttemptDispatch, PlanApplicationError> {
        let connection = self.connection()?;
        let header = connection
            .query_row(
                r#"SELECT run.id, task_run.id, task_run.subtask_id, run.project_path,
                          run.worktree_path, version.planner_profile_id, run.plan_version_id
                   FROM plan_subtask_runs AS task_run
                   JOIN plan_runs AS run ON run.id = task_run.plan_run_id
                   JOIN plan_versions AS version ON version.id = run.plan_version_id
                   WHERE task_run.id = ?1 AND task_run.status = 'dispatching'
                     AND run.status = 'running' AND run.worktree_path IS NOT NULL"#,
                [subtask_run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::Conflict)?;
        let profile_id = header.5.ok_or_else(|| {
            PlanApplicationError::Validation(
                "approved Plan version has no captured OnePiece Profile".to_string(),
            )
        })?;
        let task = read_task(&connection, &header.6, &header.2)?;
        let (direct_predecessor_ids, predecessor_sources) =
            read_predecessors(&connection, &header.0, &header.6, &header.2)?;
        Ok(AttemptDispatch {
            plan_run_id: header.0,
            subtask_run_id: header.1,
            task,
            project_path: header.3,
            worktree_path: header.4,
            profile_id,
            direct_predecessor_ids,
            predecessor_sources,
        })
    }

    pub(crate) fn create_attempt(
        &self,
        subtask_run_id: &str,
        now: &str,
    ) -> Result<AttemptRecord, PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let status: Option<String> = transaction
            .query_row(
                "SELECT status FROM plan_subtask_runs WHERE id = ?1",
                [subtask_run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if status.as_deref() != Some(SubTaskRunStatus::Dispatching.as_str()) {
            return Err(PlanApplicationError::Conflict);
        }
        let sequence: u32 = transaction
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM plan_subtask_attempts WHERE subtask_run_id = ?1",
                [subtask_run_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let id = Uuid::new_v4().to_string();
        transaction
            .execute(
                r#"INSERT INTO plan_subtask_attempts
                   (id, subtask_run_id, sequence, status, started_at)
                   VALUES (?1, ?2, ?3, 'dispatching', ?4)"#,
                params![id, subtask_run_id, sequence, now],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(AttemptRecord { id, sequence })
    }

    pub(crate) fn start_attempt(
        &self,
        subtask_run_id: &str,
        attempt_id: &str,
        session_id: &str,
        profile_id: &str,
        operation_id: Option<&str>,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        SubTaskRunStatus::Dispatching
            .transition(SubTaskRunStatus::Running)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let attempt_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_attempts
                   SET status = 'running', session_id = ?2, profile_id = ?3, operation_id = ?4
                   WHERE id = ?1 AND subtask_run_id = ?5 AND status = 'dispatching'"#,
                params![
                    attempt_id,
                    session_id,
                    profile_id,
                    operation_id,
                    subtask_run_id
                ],
            )
            .map_err(storage_error)?;
        let task_changed = transaction
            .execute(
                "UPDATE plan_subtask_runs SET status = 'running', updated_at = ?2 WHERE id = ?1 AND status = 'dispatching'",
                params![subtask_run_id, now],
            )
            .map_err(storage_error)?;
        if attempt_changed != 1 || task_changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn correlate_attempt_execution(
        &self,
        attempt_id: &str,
        operation_id: Option<&str>,
        execution_run_id: Option<&str>,
    ) -> Result<(), PlanApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE plan_subtask_attempts
               SET operation_id = ?2, execution_run_id = ?3
               WHERE id = ?1 AND status IN ('running', 'verifying')"#,
                params![attempt_id, operation_id, execution_run_id],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        Ok(())
    }

    pub(crate) fn fail_attempt_dispatch(
        &self,
        subtask_run_id: &str,
        attempt_id: &str,
        error_class: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        SubTaskRunStatus::Dispatching
            .transition(SubTaskRunStatus::Failed)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                r#"UPDATE plan_subtask_attempts
                   SET status = 'failed', error_class = ?2, completed_at = ?3
                   WHERE id = ?1 AND subtask_run_id = ?4 AND status = 'dispatching'"#,
                params![attempt_id, error_class, now, subtask_run_id],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"UPDATE plan_subtask_runs SET status = 'failed', updated_at = ?2,
                          completed_at = ?2
                   WHERE id = ?1 AND status = 'dispatching'"#,
                params![subtask_run_id, now],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn finish_attempt_generation(
        &self,
        subtask_run_id: &str,
        attempt_id: &str,
        update: &AttemptTerminalUpdate,
        succeeded: bool,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let attempt_status = if succeeded { "verifying" } else { "failed" };
        let task_status = if succeeded {
            SubTaskRunStatus::Verifying
        } else {
            SubTaskRunStatus::Failed
        };
        SubTaskRunStatus::Running
            .transition(task_status)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let changed_files = serde_json::to_string(&update.changed_files).map_err(storage_error)?;
        let attempt_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_attempts
                   SET status = ?2, token_usage = ?3, tool_call_count = ?4,
                       error_class = ?5, completed_at = CASE WHEN ?2 = 'failed' THEN ?6 ELSE NULL END
                   WHERE id = ?1 AND subtask_run_id = ?7 AND status = 'running'"#,
                params![attempt_id, attempt_status, update.token_usage, update.tool_call_count, update.error_class, now, subtask_run_id],
            )
            .map_err(storage_error)?;
        let task_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_runs
                   SET status = ?2, result_summary = ?3, changed_files = ?4,
                       updated_at = ?5, completed_at = CASE WHEN ?2 = 'failed' THEN ?5 ELSE NULL END
                   WHERE id = ?1 AND status = 'running'"#,
                params![
                    subtask_run_id,
                    task_status.as_str(),
                    update.result_summary,
                    changed_files,
                    now
                ],
            )
            .map_err(storage_error)?;
        if attempt_changed != 1 || task_changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn cancel_attempt_generation(
        &self,
        subtask_run_id: &str,
        attempt_id: &str,
        update: &AttemptTerminalUpdate,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let changed_files = serde_json::to_string(&update.changed_files).map_err(storage_error)?;
        let attempt_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_attempts
                   SET status = 'cancelled', token_usage = ?2, tool_call_count = ?3,
                       error_class = 'cancelled', completed_at = ?4
                   WHERE id = ?1 AND subtask_run_id = ?5
                     AND status IN ('running', 'verifying', 'failed')"#,
                params![
                    attempt_id,
                    update.token_usage,
                    update.tool_call_count,
                    now,
                    subtask_run_id
                ],
            )
            .map_err(storage_error)?;
        let task_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_runs
                   SET status = 'cancelled', result_summary = ?2, changed_files = ?3,
                       updated_at = ?4, completed_at = ?4
                   WHERE id = ?1 AND status IN ('running', 'verifying', 'failed')"#,
                params![subtask_run_id, update.result_summary, changed_files, now],
            )
            .map_err(storage_error)?;
        if attempt_changed != 1 || task_changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn load_attempt_verification(
        &self,
        subtask_run_id: &str,
    ) -> Result<AttemptVerificationDispatch, PlanApplicationError> {
        let connection = self.connection()?;
        let header = connection
            .query_row(
                r#"SELECT attempt.id, run.id, task_run.id, run.worktree_path,
                          run.plan_version_id, task_run.subtask_id
                   FROM plan_subtask_runs AS task_run
                   JOIN plan_runs AS run ON run.id = task_run.plan_run_id
                   JOIN plan_subtask_attempts AS attempt
                     ON attempt.subtask_run_id = task_run.id
                   WHERE task_run.id = ?1 AND task_run.status = 'verifying'
                     AND attempt.status = 'verifying' AND run.status = 'running'
                   ORDER BY attempt.sequence DESC LIMIT 1"#,
                [subtask_run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::Conflict)?;
        let task = read_task(&connection, &header.4, &header.5)?;
        Ok(AttemptVerificationDispatch {
            attempt_id: header.0,
            plan_run_id: header.1,
            subtask_run_id: header.2,
            worktree_path: header.3,
            commands: task.validation_commands,
        })
    }

    pub(crate) fn finish_attempt_verification(
        &self,
        dispatch: &AttemptVerificationDispatch,
        evidence: &[VerificationEvidenceUpdate],
        passed: bool,
        summary: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let terminal_status = if passed {
            SubTaskRunStatus::Succeeded
        } else {
            SubTaskRunStatus::Failed
        };
        self.finish_attempt_verification_as(dispatch, evidence, terminal_status, summary, now)
    }

    pub(crate) fn cancel_attempt_verification(
        &self,
        dispatch: &AttemptVerificationDispatch,
        evidence: &[VerificationEvidenceUpdate],
        summary: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        self.finish_attempt_verification_as(
            dispatch,
            evidence,
            SubTaskRunStatus::Cancelled,
            summary,
            now,
        )
    }

    fn finish_attempt_verification_as(
        &self,
        dispatch: &AttemptVerificationDispatch,
        evidence: &[VerificationEvidenceUpdate],
        terminal_status: SubTaskRunStatus,
        summary: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        SubTaskRunStatus::Verifying
            .transition(terminal_status)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        for item in evidence {
            let duration_ms = item
                .duration_ms
                .map(i64::try_from)
                .transpose()
                .map_err(storage_error)?;
            transaction
                .execute(
                    r#"INSERT INTO plan_verification_evidence (
                           id, attempt_id, command_id, status, exit_code, duration_ms,
                           output_summary, created_at
                       ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                    params![
                        Uuid::new_v4().to_string(),
                        dispatch.attempt_id,
                        item.command_id,
                        item.status,
                        item.exit_code,
                        duration_ms,
                        item.output_summary,
                        now
                    ],
                )
                .map_err(storage_error)?;
        }
        let attempt_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_attempts
                   SET status = ?2, error_class = CASE
                       WHEN ?2 = 'failed' THEN 'verification_failed'
                       WHEN ?2 = 'cancelled' THEN 'cancelled'
                       ELSE error_class END, completed_at = ?3
                   WHERE id = ?1 AND subtask_run_id = ?4 AND status = 'verifying'"#,
                params![
                    dispatch.attempt_id,
                    terminal_status.as_str(),
                    now,
                    dispatch.subtask_run_id
                ],
            )
            .map_err(storage_error)?;
        let task_changed = transaction
            .execute(
                r#"UPDATE plan_subtask_runs
                   SET status = ?2, verification_summary = ?3, updated_at = ?4,
                       completed_at = ?4
                   WHERE id = ?1 AND plan_run_id = ?5 AND status = 'verifying'"#,
                params![
                    dispatch.subtask_run_id,
                    terminal_status.as_str(),
                    summary,
                    now,
                    dispatch.plan_run_id
                ],
            )
            .map_err(storage_error)?;
        if attempt_changed != 1 || task_changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.commit().map_err(storage_error)
    }
}

fn read_task(
    connection: &crate::platform::database::PooledSqlite,
    version_id: &str,
    subtask_id: &str,
) -> Result<SubTaskSpec, PlanApplicationError> {
    let values = connection
        .query_row(
            r#"SELECT id, title, description, acceptance_criteria, ordinal, assigned_role,
                      token_budget, tool_call_limit, timeout_seconds, validation_commands
               FROM plan_subtasks WHERE plan_version_id = ?1 AND id = ?2"#,
            params![version_id, subtask_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, u16>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<u32>>(6)?,
                    row.get::<_, Option<u32>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .map_err(storage_error)?;
    Ok(SubTaskSpec {
        id: values.0,
        title: values.1,
        description: values.2,
        acceptance_criteria: serde_json::from_str(&values.3).map_err(storage_error)?,
        ordinal: values.4,
        assigned_role: values.5,
        limits: ResourceLimits {
            token_budget: values.6,
            tool_call_limit: values.7,
            timeout_seconds: values
                .8
                .map(u64::try_from)
                .transpose()
                .map_err(storage_error)?,
        },
        validation_commands: serde_json::from_str::<Vec<VerificationCommand>>(&values.9)
            .map_err(storage_error)?,
    })
}

fn read_predecessors(
    connection: &crate::platform::database::PooledSqlite,
    run_id: &str,
    version_id: &str,
    subtask_id: &str,
) -> Result<(Vec<String>, Vec<PredecessorContextSource>), PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT predecessor_run.subtask_id, predecessor_run.topological_rank,
                      predecessor_run.ordinal, predecessor_run.status,
                      predecessor_run.result_summary, predecessor_run.changed_files,
                      predecessor_run.verification_summary
               FROM plan_subtask_dependencies AS dependency
               JOIN plan_subtask_runs AS predecessor_run
                 ON predecessor_run.plan_run_id = ?1
                AND predecessor_run.subtask_id = dependency.predecessor_id
               WHERE dependency.plan_version_id = ?2 AND dependency.successor_id = ?3
               ORDER BY predecessor_run.topological_rank, predecessor_run.ordinal,
                        predecessor_run.subtask_id"#,
        )
        .map_err(storage_error)?;
    let sources = statement
        .query_map(params![run_id, version_id, subtask_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u16>(1)?,
                row.get::<_, u16>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(storage_error)?
        .map(|row| {
            let value = row.map_err(storage_error)?;
            Ok(PredecessorContextSource {
                subtask_id: value.0,
                topological_rank: value.1,
                ordinal: value.2,
                outcome: value.3,
                result_summary: value.4,
                changed_files: serde_json::from_str(&value.5).map_err(storage_error)?,
                verification_summary: value.6,
            })
        })
        .collect::<Result<Vec<_>, PlanApplicationError>>()?;
    let ids = sources
        .iter()
        .map(|source| source.subtask_id.clone())
        .collect();
    Ok((ids, sources))
}
