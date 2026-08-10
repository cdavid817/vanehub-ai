use super::loop_repository::StoredDefinition;
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopDefinitionView, LoopEvidenceView, LoopIterationView,
    LoopRunView,
};
use crate::contexts::agent_runtime::domain::{LoopRunPhase, LoopRunStatus, LoopTerminalReason};
use crate::platform::database::NativeDatabase;
use rusqlite::{OptionalExtension, Row};
use std::collections::HashMap;

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
    let mut runs = match parameter {
        Some(value) => statement.query_map([value], read_run_view),
        None => statement.query_map([], read_run_view),
    }
    .map_err(loop_error)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(loop_error)?;

    // Hydrate iterations and evidence in two bulk queries instead of one per run plus one
    // per iteration (1+N+N×M round-trips). The run list is the hot UI path; a run with a
    // dozen iterations and a few evidence rows each previously fired dozens of queries.
    let run_ids = runs.iter().map(|run| run.id.as_str()).collect::<Vec<_>>();
    let iterations_by_run = load_iterations_by_run(&connection, &run_ids)?;
    let iteration_ids = iterations_by_run
        .values()
        .flatten()
        .map(|iteration| iteration.id.as_str())
        .collect::<Vec<_>>();
    let evidence_by_iteration = load_evidence_by_iteration(&connection, &iteration_ids)?;
    for run in &mut runs {
        let iterations = iterations_by_run.get(&run.id).cloned().unwrap_or_default();
        run.iterations = iterations
            .into_iter()
            .map(|mut iteration| {
                iteration.evidence = evidence_by_iteration
                    .get(&iteration.id)
                    .cloned()
                    .unwrap_or_default();
                iteration
            })
            .collect();
    }
    Ok(runs)
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

/// Bulk-loads iterations for many runs in a single query, grouped by `run_id`, preserving
/// the `sequence, id` ordering `hydrate_iterations` used per-run.
fn load_iterations_by_run(
    connection: &rusqlite::Connection,
    run_ids: &[&str],
) -> Result<HashMap<String, Vec<LoopIterationView>>, AgentRuntimeApplicationError> {
    let mut grouped: HashMap<String, Vec<LoopIterationView>> = HashMap::new();
    if run_ids.is_empty() {
        return Ok(grouped);
    }
    let placeholders = (0..run_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let params: Vec<&dyn rusqlite::ToSql> = run_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();
    let sql = format!(
        r#"SELECT id, run_id, sequence, status, worker_session_id, verifier_session_id,
            worker_summary, verifier_recommendation, verifier_findings, decision_reason,
            diff_fingerprint, check_failure_fingerprint, user_feedback, started_at, completed_at
           FROM loop_iterations WHERE run_id IN ({placeholders}) ORDER BY sequence, id"#,
    );
    let mut statement = connection.prepare(&sql).map_err(loop_error)?;
    let rows = statement
        .query_map(params.as_slice(), read_iteration)
        .map_err(loop_error)?;
    for iteration in rows {
        let iteration = iteration.map_err(loop_error)?;
        grouped
            .entry(iteration.run_id.clone())
            .or_default()
            .push(iteration);
    }
    Ok(grouped)
}

/// Bulk-loads evidence for many iterations in a single query, grouped by `iteration_id`,
/// preserving the `created_at, id` ordering `load_evidence` used per-iteration.
fn load_evidence_by_iteration(
    connection: &rusqlite::Connection,
    iteration_ids: &[&str],
) -> Result<HashMap<String, Vec<LoopEvidenceView>>, AgentRuntimeApplicationError> {
    let mut grouped: HashMap<String, Vec<LoopEvidenceView>> = HashMap::new();
    if iteration_ids.is_empty() {
        return Ok(grouped);
    }
    let placeholders = (0..iteration_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let params: Vec<&dyn rusqlite::ToSql> = iteration_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();
    let sql = format!(
        r#"SELECT id, run_id, iteration_id, kind, status, summary, operation_id, command_id,
            exit_code, duration_ms, details, created_at
           FROM loop_evidence WHERE iteration_id IN ({placeholders}) ORDER BY created_at, id"#,
    );
    let mut statement = connection.prepare(&sql).map_err(loop_error)?;
    let rows = statement
        .query_map(params.as_slice(), read_evidence)
        .map_err(loop_error)?;
    for evidence in rows {
        let evidence = evidence.map_err(loop_error)?;
        let key = evidence.iteration_id.clone().unwrap_or_default();
        grouped.entry(key).or_default().push(evidence);
    }
    Ok(grouped)
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
