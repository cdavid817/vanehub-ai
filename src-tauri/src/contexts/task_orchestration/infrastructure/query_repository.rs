use super::repository::{storage_error, SqlitePlanRepository};
use crate::contexts::task_orchestration::application::{
    PlanApplicationError, PlanAttemptEvidenceView, PlanRunDetailView, PlanRunPageView,
    PlanRunSummaryView, PlanSubTaskAttemptView, PlanSubTaskRunView,
};
use crate::contexts::task_orchestration::domain::PlanRunStatus;
use rusqlite::{params, OptionalExtension, Row};

const RUN_PAGE_SIZE: usize = 25;
const RUN_QUERY_LIMIT: i64 = 26;

impl SqlitePlanRepository {
    pub(crate) fn get_attempt_evidence(
        &self,
        attempt_id: &str,
    ) -> Result<Vec<PlanAttemptEvidenceView>, PlanApplicationError> {
        let connection = self.connection()?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM plan_subtask_attempts WHERE id = ?1",
                [attempt_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !exists {
            return Err(PlanApplicationError::NotFound);
        }
        read_evidence(&connection, attempt_id)
    }

    pub(crate) fn list_run_summaries(
        &self,
        cursor: Option<&str>,
    ) -> Result<PlanRunPageView, PlanApplicationError> {
        let connection = self.connection()?;
        let cursor_key = cursor
            .map(|id| {
                connection
                    .query_row(
                        "SELECT created_at, id FROM plan_runs WHERE id = ?1",
                        [id],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or(PlanApplicationError::Validation(
                        "invalid PlanRun cursor".to_string(),
                    ))
            })
            .transpose()?;
        let mut items = if let Some((created_at, id)) = cursor_key {
            let mut statement = connection
                .prepare(&format!("{} WHERE (run.created_at < ?1 OR (run.created_at = ?1 AND run.id < ?2)) GROUP BY run.id ORDER BY run.created_at DESC, run.id DESC LIMIT ?3", summary_select()))
                .map_err(storage_error)?;
            let result = statement
                .query_map(params![created_at, id, RUN_QUERY_LIMIT], read_summary)
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            result
        } else {
            let mut statement = connection
                .prepare(&format!(
                    "{} GROUP BY run.id ORDER BY run.created_at DESC, run.id DESC LIMIT ?1",
                    summary_select()
                ))
                .map_err(storage_error)?;
            let result = statement
                .query_map([RUN_QUERY_LIMIT], read_summary)
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            result
        };
        let next_cursor =
            (items.len() > RUN_PAGE_SIZE).then(|| items[RUN_PAGE_SIZE - 1].id.clone());
        items.truncate(RUN_PAGE_SIZE);
        Ok(PlanRunPageView { items, next_cursor })
    }

    pub(crate) fn get_run_detail(
        &self,
        run_id: &str,
    ) -> Result<PlanRunDetailView, PlanApplicationError> {
        let connection = self.connection()?;
        let header = connection
            .query_row(
                &format!("{} WHERE run.id = ?1 GROUP BY run.id", detail_select()),
                [run_id],
                read_detail_header,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        let mut tasks = read_task_runs(&connection, run_id)?;
        for task in &mut tasks {
            task.attempts = read_attempts(&connection, &task.id)?;
        }
        let available_controls = controls_for_status(&header.0.status);
        Ok(PlanRunDetailView {
            summary: header.0,
            project_path: header.1,
            base_ref: header.2,
            base_oid: header.3,
            worktree_path: header.4,
            worktree_name: header.5,
            worktree_branch: header.6,
            tasks,
            available_controls,
        })
    }
}

fn summary_select() -> &'static str {
    r#"SELECT run.id, run.plan_id, run.status,
              COALESCE(SUM(CASE WHEN task.status = 'succeeded' THEN 1 ELSE 0 END), 0),
              COUNT(task.id), run.simulated, run.created_at, run.updated_at
       FROM plan_runs AS run
       LEFT JOIN plan_subtask_runs AS task ON task.plan_run_id = run.id"#
}

fn detail_select() -> &'static str {
    r#"SELECT run.id, run.plan_id, run.status,
              COALESCE(SUM(CASE WHEN task.status = 'succeeded' THEN 1 ELSE 0 END), 0),
              COUNT(task.id), run.simulated, run.created_at, run.updated_at,
              run.project_path, run.base_ref, run.base_oid, run.worktree_path,
              run.worktree_name, run.worktree_branch
       FROM plan_runs AS run
       LEFT JOIN plan_subtask_runs AS task ON task.plan_run_id = run.id"#
}

fn read_summary(row: &Row<'_>) -> rusqlite::Result<PlanRunSummaryView> {
    Ok(PlanRunSummaryView {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        status: row.get(2)?,
        completed_tasks: row.get(3)?,
        total_tasks: row.get(4)?,
        simulated: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

type DetailHeader = (
    PlanRunSummaryView,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn read_detail_header(row: &Row<'_>) -> rusqlite::Result<DetailHeader> {
    Ok((
        read_summary(row)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
    ))
}

fn read_task_runs(
    connection: &crate::platform::database::PooledSqlite,
    run_id: &str,
) -> Result<Vec<PlanSubTaskRunView>, PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT task_run.id, task_run.subtask_id, task.title, task_run.status,
                      task_run.topological_rank, task_run.ordinal, task_run.result_summary,
                      task_run.changed_files, task_run.verification_summary
               FROM plan_subtask_runs AS task_run
               JOIN plan_runs AS run ON run.id = task_run.plan_run_id
               JOIN plan_subtasks AS task ON task.plan_version_id = run.plan_version_id AND task.id = task_run.subtask_id
               WHERE task_run.plan_run_id = ?1
               ORDER BY task_run.topological_rank, task_run.ordinal, task_run.id"#,
        )
        .map_err(storage_error)?;
    let result = statement
        .query_map([run_id], |row| {
            let raw: String = row.get(7)?;
            let changed_files = serde_json::from_str(&raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?;
            Ok(PlanSubTaskRunView {
                id: row.get(0)?,
                subtask_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                topological_rank: row.get(4)?,
                ordinal: row.get(5)?,
                result_summary: row.get(6)?,
                changed_files,
                verification_summary: row.get(8)?,
                attempts: Vec::new(),
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error);
    result
}

fn read_attempts(
    connection: &crate::platform::database::PooledSqlite,
    subtask_run_id: &str,
) -> Result<Vec<PlanSubTaskAttemptView>, PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT id, sequence, status, session_id, profile_id, execution_run_id,
                      operation_id, token_usage, tool_call_count, error_class, started_at, completed_at
               FROM plan_subtask_attempts WHERE subtask_run_id = ?1 ORDER BY sequence"#,
        )
        .map_err(storage_error)?;
    let attempts = statement
        .query_map([subtask_run_id], |row| {
            Ok(PlanSubTaskAttemptView {
                id: row.get(0)?,
                sequence: row.get(1)?,
                status: row.get(2)?,
                session_id: row.get(3)?,
                profile_id: row.get(4)?,
                execution_run_id: row.get(5)?,
                operation_id: row.get(6)?,
                token_usage: row.get(7)?,
                tool_call_count: row.get(8)?,
                error_class: row.get(9)?,
                started_at: row.get(10)?,
                completed_at: row.get(11)?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(attempts)
}

fn read_evidence(
    connection: &crate::platform::database::PooledSqlite,
    attempt_id: &str,
) -> Result<Vec<PlanAttemptEvidenceView>, PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT id, command_id, status, exit_code, duration_ms, output_summary, created_at
           FROM plan_verification_evidence WHERE attempt_id = ?1 ORDER BY created_at, id"#,
        )
        .map_err(storage_error)?;
    let result = statement
        .query_map([attempt_id], |row| {
            let duration: Option<i64> = row.get(4)?;
            let duration_ms = duration.map(u64::try_from).transpose().map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Integer,
                    error.into(),
                )
            })?;
            Ok(PlanAttemptEvidenceView {
                id: row.get(0)?,
                command_id: row.get(1)?,
                status: row.get(2)?,
                exit_code: row.get(3)?,
                duration_ms,
                output_summary: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error);
    result
}

fn controls_for_status(status: &str) -> Vec<String> {
    match PlanRunStatus::parse(status) {
        Some(PlanRunStatus::Running) => vec!["pause".into(), "cancel".into()],
        Some(PlanRunStatus::PauseRequested | PlanRunStatus::Paused) => {
            vec!["resume".into(), "cancel".into()]
        }
        Some(PlanRunStatus::RecoveryRequired) => vec!["recover".into(), "cancel".into()],
        Some(PlanRunStatus::AwaitingAcceptance) => vec!["accept".into()],
        _ => Vec::new(),
    }
}
