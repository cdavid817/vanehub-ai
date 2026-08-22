use crate::contexts::execution_observability::application::evidence::models::{
    ExecutionRecordDetailFields, ExecutionRecordKind, ExecutionRecordProjection,
};
use crate::contexts::execution_observability::application::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::{
    parse_fidelity_token, parse_status_token, reason_codes, EvidenceCommandId,
    EvidenceCoverageState, EvidenceSeatId, EvidenceSessionId, EvidenceToolCallId, QueryCoverage,
};
use rusqlite::{params, Connection, Row};

pub(super) fn record_columns() -> &'static str {
    "record_id, record_kind, session_id, run_id, trace_id, span_id, operation_id, agent_id, \
     seat_id, started_at, ended_at, duration_ms, status, fidelity, occurred_at, \
     command_runtime_kind, redacted_display, cwd_display, exit_code, signal, output_availability, \
     output_truncated, tool_name, tool_call_id, parent_agent_id, child_agent_id, attempt, \
     verification_name, verification_outcome, verification_passed, verification_failed, \
     last_sequence"
}

/// Reads a projection row back through the domain's parsers.
///
/// A row whose status, fidelity, or kind this build does not recognise is an error rather than a
/// row with a guessed value: it means the database was written by something this binary does not
/// understand, and rendering it as the nearest familiar option would be a quiet lie about what ran.
pub(super) fn read_record_row(
    row: &Row<'_>,
) -> rusqlite::Result<Result<ExecutionRecordProjection, EvidenceApplicationError>> {
    let unreadable = |field: &str| {
        Err(EvidenceApplicationError::Storage(format!(
            "evidence record row carries an unreadable {field}"
        )))
    };

    let record_kind: String = row.get(1)?;
    let Some(kind) = ExecutionRecordKind::parse(&record_kind) else {
        return Ok(unreadable("record kind"));
    };
    let status_text: String = row.get(12)?;
    let Some(status) = parse_status_token(&status_text) else {
        return Ok(unreadable("status"));
    };
    let fidelity_text: String = row.get(13)?;
    let Some(fidelity) = parse_fidelity_token(&fidelity_text) else {
        return Ok(unreadable("fidelity"));
    };
    let session_text: String = row.get(2)?;
    let Ok(session_id) = EvidenceSessionId::parse(session_text) else {
        return Ok(unreadable("session id"));
    };
    let seat_id = match row.get::<_, Option<String>>(8)? {
        Some(value) => match EvidenceSeatId::parse(value) {
            Ok(seat) => Some(seat),
            Err(_) => return Ok(unreadable("seat id")),
        },
        None => None,
    };

    let detail = match kind {
        ExecutionRecordKind::Command => {
            let command_id = record_id_suffix(&row.get::<_, String>(0)?, "command:");
            let Some(command_id) =
                command_id.and_then(|value| EvidenceCommandId::parse(value).ok())
            else {
                return Ok(unreadable("command id"));
            };
            ExecutionRecordDetailFields::Command {
                command_id,
                runtime_kind: super::tokens::runtime_kind(
                    row.get::<_, Option<String>>(15)?.as_deref(),
                ),
                redacted_display: row.get(16)?,
                cwd_display: row.get(17)?,
                exit_code: row.get(18)?,
                signal: row.get(19)?,
                output_availability: super::tokens::output_availability(
                    row.get::<_, Option<String>>(20)?.as_deref(),
                ),
                output_truncated: row.get::<_, Option<i64>>(21)?.unwrap_or(0) != 0,
            }
        }
        ExecutionRecordKind::Tool => ExecutionRecordDetailFields::Tool {
            tool_call_id: row
                .get::<_, Option<String>>(23)?
                .and_then(|value| EvidenceToolCallId::parse(value).ok()),
            tool_name: row.get::<_, Option<String>>(22)?.unwrap_or_default(),
        },
        ExecutionRecordKind::Delegation => ExecutionRecordDetailFields::Delegation {
            parent_agent_id: row.get(24)?,
            child_agent_id: row.get(25)?,
            attempt: row.get::<_, Option<i64>>(26)?.map(|value| value as u32),
        },
        ExecutionRecordKind::Verification => ExecutionRecordDetailFields::Verification {
            name: row.get::<_, Option<String>>(27)?.unwrap_or_default(),
            outcome: super::tokens::verification_outcome(
                row.get::<_, Option<String>>(28)?.as_deref(),
            ),
            passed_count: row.get::<_, Option<i64>>(29)?.map(|value| value as u32),
            failed_count: row.get::<_, Option<i64>>(30)?.map(|value| value as u32),
        },
    };

    Ok(Ok(ExecutionRecordProjection {
        record_id: row.get(0)?,
        kind,
        session_id,
        run_id: row.get(3)?,
        trace_id: row.get(4)?,
        span_id: row.get(5)?,
        operation_id: row.get(6)?,
        agent_id: row.get(7)?,
        seat_id,
        started_at: row.get(9)?,
        ended_at: row.get(10)?,
        duration_ms: row.get::<_, Option<i64>>(11)?.map(|value| value as u64),
        status,
        fidelity,
        detail,
        last_sequence: row.get(31)?,
        occurred_at: row.get(14)?,
    }))
}

fn record_id_suffix(record_id: &str, prefix: &str) -> Option<String> {
    record_id.strip_prefix(prefix).map(str::to_string)
}

/// Builds the coverage a query reports.
///
/// The important case is a session with no events at all. Group 3 ships the store; Group 4 wires
/// the producers, so before then an empty result means "nothing is capturing", not "nothing ran".
/// Reporting `complete` here would turn an unwired system into a confident claim of absence, which
/// is the single thing this capability must never do.
pub(super) fn coverage_for_session(
    connection: &Connection,
    session_id: Option<&EvidenceSessionId>,
) -> Result<QueryCoverage, EvidenceApplicationError> {
    let Some(session_id) = session_id else {
        return Ok(QueryCoverage::complete().degrade_to(
            EvidenceCoverageState::Partial,
            reason_codes::CAPTURE_NOT_INITIALIZED,
        ));
    };
    let storage = |error: rusqlite::Error| EvidenceApplicationError::Storage(error.to_string());

    let event_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM execution_evidence_events WHERE session_id = ?1",
            params![session_id.as_str()],
            |row| row.get(0),
        )
        .map_err(storage)?;

    let metadata: CoverageMetadata = connection
        .query_row(
            "SELECT dropped_count, conflict_count, retention_trimmed, oldest_available_at, \
             newest_available_at FROM execution_evidence_coverage WHERE session_id = ?1",
            params![session_id.as_str()],
            |row| {
                Ok(CoverageMetadata {
                    dropped: row.get(0)?,
                    conflicts: row.get(1)?,
                    trimmed: row.get(2)?,
                    oldest: row.get(3)?,
                    newest: row.get(4)?,
                })
            },
        )
        .optional_row()
        .map_err(storage)?
        .unwrap_or_default();
    let CoverageMetadata {
        dropped,
        conflicts,
        trimmed,
        oldest,
        newest,
    } = metadata;

    let mut coverage = QueryCoverage::complete().with_boundaries(oldest, newest);
    if let Some(indexed_through) = projection_lag(connection, session_id)? {
        // The journal holds a lifecycle event the projection has not applied. Every query answers
        // from the projection, so the honest answer is "still indexing" with the point it has
        // reached — reporting `complete` here would present a stale row set as the whole truth.
        coverage = coverage
            .degrade_to(
                EvidenceCoverageState::Indexing,
                reason_codes::PROJECTION_REBUILDING,
            )
            .with_indexed_through(indexed_through);
    }
    if event_count == 0 {
        coverage = coverage.degrade_to(
            EvidenceCoverageState::Partial,
            reason_codes::CAPTURE_NOT_INITIALIZED,
        );
    }
    if dropped > 0 {
        coverage = coverage
            .degrade_to(EvidenceCoverageState::Partial, reason_codes::DROPPED_EVENTS)
            .with_dropped_count(Some(dropped.max(0) as u32));
    }
    if conflicts > 0 {
        coverage = coverage.degrade_to(
            EvidenceCoverageState::Partial,
            reason_codes::CONFLICTING_SOURCE_EVENT,
        );
    }
    if trimmed > 0 {
        coverage = coverage.degrade_to(
            EvidenceCoverageState::Partial,
            reason_codes::RETENTION_EXPIRED,
        );
    }
    Ok(coverage)
}

#[derive(Default)]
struct CoverageMetadata {
    dropped: i64,
    conflicts: i64,
    trimmed: i64,
    oldest: Option<String>,
    newest: Option<String>,
}

/// The kinds that produce a projection row. An event of any other kind is recorded in the journal
/// and deliberately projects to nothing, so comparing raw sequence maxima would report a permanent
/// lag; only these are the ones a record is expected to exist for.
const PROJECTED_KINDS: &str = "'agent.delegated', 'agent.completed', 'tool.started', \
     'tool.completed', 'command.started', 'command.completed', 'verification.completed'";

/// How far the projection has caught up, or `None` when it is current.
///
/// Returns the newest occurrence the projection has applied, which is what a reader needs to know
/// how stale the answer is. A lag is normally momentary — the projection is written inside the
/// same transaction as the event — so this fires after an interrupted replay or a recovered crash.
fn projection_lag(
    connection: &Connection,
    session_id: &EvidenceSessionId,
) -> Result<Option<Option<String>>, EvidenceApplicationError> {
    let storage = |error: rusqlite::Error| EvidenceApplicationError::Storage(error.to_string());
    let journal_head: i64 = connection
        .query_row(
            &format!(
                "SELECT COALESCE(MAX(sequence), 0) FROM execution_evidence_events \
                 WHERE session_id = ?1 AND kind IN ({PROJECTED_KINDS})"
            ),
            params![session_id.as_str()],
            |row| row.get(0),
        )
        .map_err(storage)?;
    let projected: (i64, Option<String>) = connection
        .query_row(
            "SELECT COALESCE(MAX(last_sequence), 0), MAX(occurred_at) \
             FROM execution_evidence_records WHERE session_id = ?1",
            params![session_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(storage)?;
    if projected.0 >= journal_head {
        return Ok(None);
    }
    Ok(Some(projected.1))
}

/// `query_row` returns `QueryReturnedNoRows` for an absent row; treating that as `None` keeps the
/// "no coverage metadata yet" case out of the error path, where it would look like a failure.
trait OptionalRow<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRow<T> for rusqlite::Result<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
