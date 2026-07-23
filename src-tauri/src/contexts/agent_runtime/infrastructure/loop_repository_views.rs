use super::loop_repository::StoredDefinition;
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopDefinitionView, LoopEvidenceView, LoopIterationView,
    LoopRunView,
};
use crate::contexts::agent_runtime::domain::{LoopRunPhase, LoopRunStatus, LoopTerminalReason};
use crate::platform::database::NativeDatabase;
use rusqlite::{OptionalExtension, Row};

const RUN_SELECT: &str = r#"SELECT id, definition_id, definition_snapshot, status, phase,
    terminal_reason, current_iteration, consecutive_runtime_errors, consecutive_no_progress,
    pause_requested, project_path, worktree_path, worktree_name, worktree_branch,
    active_operation_id, simulated, created_at, started_at, updated_at, completed_at
    FROM loop_runs"#;

pub(super) fn list_run_views(
    database: &NativeDatabase,
    definition_id: Option<&str>,
) -> Result<Vec<LoopRunView>, AgentRuntimeApplicationError> {
    let connection = database.connection().map_err(loop_error)?;
    let (sql, parameter) = match definition_id {
        Some(value) => (
            format!("{RUN_SELECT} WHERE definition_id = ?1 ORDER BY created_at DESC, id"),
            Some(value),
        ),
        None => (format!("{RUN_SELECT} ORDER BY created_at DESC, id"), None),
    };
    let mut statement = connection.prepare(&sql).map_err(loop_error)?;
    let rows = match parameter {
        Some(value) => statement.query_map([value], read_run_view),
        None => statement.query_map([], read_run_view),
    }
    .map_err(loop_error)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(loop_error)?;
    rows.into_iter()
        .map(|run| hydrate_iterations(&connection, run))
        .collect()
}

pub(super) fn find_run_view(
    database: &NativeDatabase,
    run_id: &str,
) -> Result<Option<LoopRunView>, AgentRuntimeApplicationError> {
    let connection = database.connection().map_err(loop_error)?;
    let run = connection
        .query_row(
            &format!("{RUN_SELECT} WHERE id = ?1"),
            [run_id],
            read_run_view,
        )
        .optional()
        .map_err(loop_error)?;
    run.map(|value| hydrate_iterations(&connection, value))
        .transpose()
}

fn hydrate_iterations(
    connection: &rusqlite::Connection,
    mut run: LoopRunView,
) -> Result<LoopRunView, AgentRuntimeApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT id, run_id, sequence, status, worker_session_id, verifier_session_id,
                worker_summary, verifier_recommendation, verifier_findings, decision_reason,
                diff_fingerprint, check_failure_fingerprint, user_feedback, started_at, completed_at
               FROM loop_iterations WHERE run_id = ?1 ORDER BY sequence, id"#,
        )
        .map_err(loop_error)?;
    let iterations = statement
        .query_map([run.id.as_str()], read_iteration)
        .map_err(loop_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(loop_error)?;
    run.iterations = iterations
        .into_iter()
        .map(|mut iteration| {
            iteration.evidence = load_evidence(connection, &iteration.id)?;
            Ok(iteration)
        })
        .collect::<Result<Vec<_>, AgentRuntimeApplicationError>>()?;
    Ok(run)
}

fn load_evidence(
    connection: &rusqlite::Connection,
    iteration_id: &str,
) -> Result<Vec<LoopEvidenceView>, AgentRuntimeApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT id, run_id, iteration_id, kind, status, summary, operation_id, command_id,
                exit_code, duration_ms, details, created_at
               FROM loop_evidence WHERE iteration_id = ?1 ORDER BY created_at, id"#,
        )
        .map_err(loop_error)?;
    let evidence = statement
        .query_map([iteration_id], read_evidence)
        .map_err(loop_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(loop_error)?;
    Ok(evidence)
}

fn read_run_view(row: &Row<'_>) -> rusqlite::Result<LoopRunView> {
    let stored: StoredDefinition =
        serde_json::from_str(&row.get::<_, String>(2)?).map_err(sql_conversion)?;
    let definition = stored.into_domain().map_err(sql_conversion)?;
    Ok(LoopRunView {
        id: row.get(0)?,
        definition_id: row.get(1)?,
        definition_snapshot: LoopDefinitionView::from(&definition),
        status: parse_status(row.get(3)?)?,
        phase: LoopRunPhase::parse(&row.get::<_, String>(4)?).map_err(sql_conversion)?,
        terminal_reason: row
            .get::<_, Option<String>>(5)?
            .map(|value| LoopTerminalReason::parse(&value))
            .transpose()
            .map_err(sql_conversion)?,
        current_iteration: to_u16(row.get(6)?)?,
        consecutive_runtime_errors: to_u16(row.get(7)?)?,
        consecutive_no_progress: to_u16(row.get(8)?)?,
        pause_requested: row.get(9)?,
        project_path: row.get(10)?,
        worktree_path: row.get(11)?,
        worktree_name: row.get(12)?,
        worktree_branch: row.get(13)?,
        active_operation_id: row.get(14)?,
        simulated: row.get(15)?,
        created_at: row.get(16)?,
        started_at: row.get(17)?,
        updated_at: row.get(18)?,
        completed_at: row.get(19)?,
        iterations: Vec::new(),
    })
}

fn read_iteration(row: &Row<'_>) -> rusqlite::Result<LoopIterationView> {
    Ok(LoopIterationView {
        id: row.get(0)?,
        run_id: row.get(1)?,
        sequence: to_u16(row.get(2)?)?,
        status: parse_status(row.get(3)?)?,
        worker_session_id: row.get(4)?,
        verifier_session_id: row.get(5)?,
        worker_summary: row.get(6)?,
        verifier_recommendation: row.get(7)?,
        verifier_findings: parse_json(row, 8)?,
        decision_reason: row.get(9)?,
        diff_fingerprint: row.get(10)?,
        check_failure_fingerprint: row.get(11)?,
        user_feedback: row.get(12)?,
        started_at: row.get(13)?,
        completed_at: row.get(14)?,
        evidence: Vec::new(),
    })
}

fn read_evidence(row: &Row<'_>) -> rusqlite::Result<LoopEvidenceView> {
    Ok(LoopEvidenceView {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        kind: row.get(3)?,
        status: row.get(4)?,
        summary: row.get(5)?,
        operation_id: row.get(6)?,
        command_id: row.get(7)?,
        exit_code: row.get(8)?,
        duration_ms: row
            .get::<_, Option<i64>>(9)?
            .map(|value| u64::try_from(value).map_err(sql_conversion))
            .transpose()?,
        details: row
            .get::<_, Option<String>>(10)?
            .map(|value| serde_json::from_str(&value).map_err(sql_conversion))
            .transpose()?,
        created_at: row.get(11)?,
    })
}

fn parse_status(value: String) -> rusqlite::Result<LoopRunStatus> {
    LoopRunStatus::parse(&value).map_err(sql_conversion)
}

fn parse_json<T: for<'de> serde::Deserialize<'de>>(
    row: &Row<'_>,
    index: usize,
) -> rusqlite::Result<T> {
    serde_json::from_str(&row.get::<_, String>(index)?).map_err(sql_conversion)
}

fn to_u16(value: i64) -> rusqlite::Result<u16> {
    u16::try_from(value).map_err(sql_conversion)
}

fn sql_conversion(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn loop_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(error.to_string())
}
