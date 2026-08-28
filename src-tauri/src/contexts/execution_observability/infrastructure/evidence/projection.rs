use super::payload_row::verification_token;
use crate::contexts::execution_observability::application::evidence::models::ExecutionRecordKind;
use crate::contexts::execution_observability::domain::{
    fidelity_token, status_token, ExecutionEvidenceEvent, ExecutionStatus, SafeEvidencePayload,
};
use rusqlite::{params, Connection};

/// The columns one event contributes to its record.
///
/// A field is `None` when this event did not observe it, and the upsert leaves an existing value
/// alone rather than writing the `None` over it. That is what lets a start and a completion each
/// contribute what they saw without either erasing the other.
pub(super) struct ProjectionUpdate {
    pub(super) record_id: String,
    pub(super) record_kind: ExecutionRecordKind,
    pub(super) session_id: String,
    pub(super) run_id: Option<String>,
    pub(super) trace_id: Option<String>,
    pub(super) span_id: Option<String>,
    pub(super) operation_id: Option<String>,
    pub(super) agent_id: Option<String>,
    pub(super) seat_id: Option<String>,
    pub(super) started_at: Option<String>,
    pub(super) ended_at: Option<String>,
    pub(super) duration_ms: Option<u64>,
    pub(super) status: ExecutionStatus,
    pub(super) fidelity: &'static str,
    pub(super) occurred_at: String,
    pub(super) command_runtime_kind: Option<String>,
    pub(super) redacted_display: Option<String>,
    pub(super) cwd_display: Option<String>,
    pub(super) exit_code: Option<i32>,
    pub(super) signal: Option<String>,
    pub(super) output_availability: Option<String>,
    pub(super) output_truncated: Option<bool>,
    pub(super) tool_name: Option<String>,
    pub(super) tool_call_id: Option<String>,
    pub(super) attempt: Option<u32>,
    pub(super) verification_name: Option<String>,
    pub(super) verification_outcome: Option<String>,
    pub(super) verification_passed: Option<u32>,
    pub(super) verification_failed: Option<u32>,
    pub(super) sequence: i64,
}

/// Derives the record an event belongs to.
///
/// The id is built from the correlation the lifecycle shares, so a start and its completion land
/// on the same row without the producer having to know a record id exists. Events that are
/// evidence but not execution records — Shell lifecycle, file mutations, review decisions, usage,
/// gaps — return `None` and stay in the journal only.
pub(super) fn project_event(
    event: &ExecutionEvidenceEvent,
    sequence: i64,
) -> Option<ProjectionUpdate> {
    let kind = ExecutionRecordKind::for_kind(event.kind())?;
    let correlation = event.correlation();
    let session_id = correlation.session()?.as_str().to_string();
    let record_id = match kind {
        ExecutionRecordKind::Command => {
            format!("command:{}", correlation.command_id.as_ref()?.as_str())
        }
        ExecutionRecordKind::Tool => {
            format!("tool:{}", correlation.tool_call_id.as_ref()?.as_str())
        }
        ExecutionRecordKind::Delegation => format!(
            "delegation:{}:{}",
            correlation.agent_id.as_ref()?.as_str(),
            correlation
                .run_id
                .as_ref()
                .map(|run| run.as_str())
                .unwrap_or("-")
        ),
        // A verification has no paired start, so its identity is the producer's own event id.
        ExecutionRecordKind::Verification => format!(
            "verification:{}:{}",
            event.source_context().as_str(),
            event.source_event_id().as_str()
        ),
    };

    let is_start = matches!(
        event.payload(),
        SafeEvidencePayload::CommandStarted { .. }
            | SafeEvidencePayload::ToolStarted { .. }
            | SafeEvidencePayload::AgentDelegated { .. }
    );
    let status = event.status().unwrap_or(if is_start {
        ExecutionStatus::Running
    } else {
        // A completion whose producer sent no status is incomplete, not succeeded. Nothing
        // observed the outcome, and guessing one is how a failed run gets reported as fine.
        ExecutionStatus::Incomplete
    });

    let mut update = ProjectionUpdate {
        record_id,
        record_kind: kind,
        session_id,
        run_id: correlation
            .run_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        trace_id: correlation
            .trace_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        span_id: correlation
            .span_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        operation_id: correlation
            .operation_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        agent_id: correlation
            .agent_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        seat_id: correlation
            .seat_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        // Only a start observes a start time. A completion leaves it absent so the record stays
        // honestly startless rather than borrowing its own timestamp.
        started_at: is_start.then(|| event.occurred_at().to_string()),
        ended_at: (!is_start).then(|| event.occurred_at().to_string()),
        duration_ms: None,
        status,
        fidelity: fidelity_token(event.fidelity()),
        occurred_at: event.occurred_at().to_string(),
        command_runtime_kind: None,
        redacted_display: None,
        cwd_display: None,
        exit_code: None,
        signal: None,
        output_availability: None,
        output_truncated: None,
        tool_name: None,
        tool_call_id: correlation
            .tool_call_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        attempt: None,
        verification_name: None,
        verification_outcome: None,
        verification_passed: None,
        verification_failed: None,
        sequence,
    };

    match event.payload() {
        SafeEvidencePayload::CommandStarted {
            runtime_kind,
            redacted_display,
            cwd_display,
        } => {
            update.command_runtime_kind = Some(runtime_kind.as_str().to_string());
            update.redacted_display = redacted_display
                .as_ref()
                .map(|value| value.as_str().to_string());
            update.cwd_display = cwd_display.as_ref().map(|value| value.as_str().to_string());
        }
        SafeEvidencePayload::CommandCompleted {
            duration_ms,
            exit_code,
            signal,
            output_availability,
            output_truncated,
            ..
        } => {
            update.duration_ms = *duration_ms;
            update.exit_code = *exit_code;
            update.signal = signal.as_ref().map(|value| value.as_str().to_string());
            update.output_availability = Some(output_availability.as_str().to_string());
            update.output_truncated = Some(*output_truncated);
        }
        SafeEvidencePayload::ToolStarted { tool_name } => {
            update.tool_name = Some(tool_name.as_str().to_string());
        }
        SafeEvidencePayload::ToolCompleted {
            tool_name,
            duration_ms,
            ..
        } => {
            update.tool_name = Some(tool_name.as_str().to_string());
            update.duration_ms = *duration_ms;
        }
        SafeEvidencePayload::AgentDelegated { attempt } => {
            update.attempt = *attempt;
        }
        SafeEvidencePayload::AgentCompleted { duration_ms, .. } => {
            update.duration_ms = *duration_ms;
        }
        SafeEvidencePayload::VerificationCompleted {
            name,
            outcome,
            passed_count,
            failed_count,
        } => {
            update.verification_name = Some(name.as_str().to_string());
            update.verification_outcome = Some(verification_token(*outcome).to_string());
            update.verification_passed = *passed_count;
            update.verification_failed = *failed_count;
        }
        _ => {}
    }

    Some(update)
}

/// Upserts behind two guards, because arrival order alone is not enough.
///
/// `last_sequence` decides which event last touched the row, but producers publish through a
/// bounded queue and a start can legitimately arrive *after* its own completion. By sequence that
/// start is the newer event, so a sequence guard on its own would happily move a finished command
/// back to running. Terminal status is therefore sticky: once a record has succeeded, failed, been
/// cancelled, or been marked incomplete, a later non-terminal observation cannot un-finish it.
///
/// `occurred_at` takes the maximum rather than the latest writer's value, so the page position a
/// record holds reflects the newest thing observed about it and does not jump backwards when a
/// late start lands. `COALESCE(excluded.x, x)` keeps a field the newer event did not observe.
pub(super) fn upsert(
    connection: &Connection,
    update: &ProjectionUpdate,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        r#"INSERT INTO execution_evidence_records (
            record_id, record_kind, session_id, run_id, trace_id, span_id, operation_id, agent_id,
            seat_id, started_at, ended_at, duration_ms, status, fidelity, occurred_at,
            command_runtime_kind, redacted_display, cwd_display, exit_code, signal,
            output_availability, output_truncated, tool_name, tool_call_id, parent_agent_id,
            child_agent_id, attempt, verification_name, verification_outcome, verification_passed,
            verification_failed, last_sequence
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, NULL, NULL, ?25, ?26, ?27, ?28, ?29, ?30
        )
        ON CONFLICT(record_id) DO UPDATE SET
            run_id = COALESCE(excluded.run_id, run_id),
            trace_id = COALESCE(excluded.trace_id, trace_id),
            span_id = COALESCE(excluded.span_id, span_id),
            operation_id = COALESCE(excluded.operation_id, operation_id),
            agent_id = COALESCE(excluded.agent_id, agent_id),
            seat_id = COALESCE(excluded.seat_id, seat_id),
            started_at = COALESCE(excluded.started_at, started_at),
            ended_at = COALESCE(excluded.ended_at, ended_at),
            duration_ms = COALESCE(excluded.duration_ms, duration_ms),
            status = CASE
                WHEN excluded.last_sequence > last_sequence
                 AND NOT (
                     status IN ('succeeded', 'failed', 'cancelled', 'incomplete')
                     AND excluded.status IN ('queued', 'running')
                 )
                THEN excluded.status ELSE status END,
            fidelity = CASE WHEN excluded.last_sequence > last_sequence THEN excluded.fidelity ELSE fidelity END,
            occurred_at = MAX(excluded.occurred_at, occurred_at),
            command_runtime_kind = COALESCE(excluded.command_runtime_kind, command_runtime_kind),
            redacted_display = COALESCE(excluded.redacted_display, redacted_display),
            cwd_display = COALESCE(excluded.cwd_display, cwd_display),
            exit_code = COALESCE(excluded.exit_code, exit_code),
            signal = COALESCE(excluded.signal, signal),
            output_availability = COALESCE(excluded.output_availability, output_availability),
            output_truncated = COALESCE(excluded.output_truncated, output_truncated),
            tool_name = COALESCE(excluded.tool_name, tool_name),
            tool_call_id = COALESCE(excluded.tool_call_id, tool_call_id),
            attempt = COALESCE(excluded.attempt, attempt),
            verification_name = COALESCE(excluded.verification_name, verification_name),
            verification_outcome = COALESCE(excluded.verification_outcome, verification_outcome),
            verification_passed = COALESCE(excluded.verification_passed, verification_passed),
            verification_failed = COALESCE(excluded.verification_failed, verification_failed),
            last_sequence = MAX(excluded.last_sequence, last_sequence)
        "#,
        params![
            update.record_id,
            update.record_kind.as_str(),
            update.session_id,
            update.run_id,
            update.trace_id,
            update.span_id,
            update.operation_id,
            update.agent_id,
            update.seat_id,
            update.started_at,
            update.ended_at,
            update.duration_ms.map(|value| value as i64),
            status_token(update.status),
            update.fidelity,
            update.occurred_at,
            update.command_runtime_kind,
            update.redacted_display,
            update.cwd_display,
            update.exit_code,
            update.signal,
            update.output_availability,
            update.output_truncated.map(|value| value as i64),
            update.tool_name,
            update.tool_call_id,
            update.attempt,
            update.verification_name,
            update.verification_outcome,
            update.verification_passed,
            update.verification_failed,
            update.sequence,
        ],
    )?;
    Ok(())
}
