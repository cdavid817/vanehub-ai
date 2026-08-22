use super::payload_row::{from_stored, StoredPayload};
use super::projection::{project_event, upsert};
use super::repository::SqliteEvidenceRepository;
use crate::contexts::execution_observability::application::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::{
    parse_fidelity_token, parse_status_token, EvidenceCommandId, EvidenceCorrelation,
    EvidenceEventId, EvidenceFileMutationId, EvidenceOperationId, EvidenceSeatId,
    EvidenceSessionId, EvidenceSourceContext, EvidenceToolCallId, ExecutionEvidenceEvent,
    ExecutionEvidenceEventInput, ExecutionRunId, RedactionReceipt, SafeReasonCode, SourceEventId,
    SpanId, TraceId,
};
use rusqlite::{params, Connection, Transaction};

/// How many journal rows one maintenance pass touches.
///
/// Bounded so retention never turns into a full-table scan on a write path. The pass repeats until
/// it clears fewer rows than the batch, which is how a large backlog drains without any single
/// call holding the database.
const RETENTION_BATCH: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EvidenceRetentionOutcome {
    pub(crate) deleted_events: usize,
    pub(crate) deleted_records: usize,
}

impl SqliteEvidenceRepository {
    /// Deterministically rebuilds projections from the retained journal.
    ///
    /// Replay is a pure function of the journal: it appends no event, publishes no notice, and
    /// produces the same projection for the same input every time. That is what makes the
    /// projection disposable — if it is ever wrong, it can be discarded and rebuilt rather than
    /// patched, and the journal stays the only thing that has to be right.
    pub(crate) fn replay_projections(
        &self,
        session_id: Option<&EvidenceSessionId>,
    ) -> Result<usize, EvidenceApplicationError> {
        let mut connection = self.connection_for_maintenance()?;
        let transaction = connection.transaction().map_err(storage)?;

        match session_id {
            Some(session_id) => transaction
                .execute(
                    "DELETE FROM execution_evidence_records WHERE session_id = ?1",
                    params![session_id.as_str()],
                )
                .map_err(storage)?,
            None => transaction
                .execute("DELETE FROM execution_evidence_records", [])
                .map_err(storage)?,
        };

        // Ordered by sequence so the replayed projection sees events in the order the journal
        // accepted them, which is what makes the monotonic guard reach the same result as live
        // capture rather than a different one.
        let mut replayed = 0usize;
        let events = read_events(&transaction, session_id)?;
        for (event, sequence) in events {
            if let Some(update) = project_event(&event, sequence) {
                upsert(&transaction, &update).map_err(storage)?;
                replayed += 1;
            }
        }

        transaction.commit().map_err(storage)?;
        Ok(replayed)
    }

    /// Removes evidence older than the cutoff in bounded batches.
    ///
    /// Projections whose backing events are all gone go with them: a record left behind would
    /// claim an observation the journal can no longer substantiate. A record whose lifecycle
    /// straddles the cutoff survives, because its newest event is still retained and dropping it
    /// would lose work that is still inside the window.
    pub(crate) fn maintain_retention(
        &self,
        cutoff: &str,
        now: &str,
    ) -> Result<EvidenceRetentionOutcome, EvidenceApplicationError> {
        let mut connection = self.connection_for_maintenance()?;
        let transaction = connection.transaction().map_err(storage)?;

        let sessions: Vec<String> = {
            let mut statement = transaction
                .prepare(
                    "SELECT DISTINCT session_id FROM execution_evidence_events \
                     WHERE occurred_at < ?1 LIMIT ?2",
                )
                .map_err(storage)?;
            let collected = statement
                .query_map(params![cutoff, RETENTION_BATCH as i64], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(storage)?
                .collect::<Result<Vec<String>, _>>()
                .map_err(storage)?;
            collected
        };

        let deleted_events = transaction
            .execute(
                "DELETE FROM execution_evidence_events WHERE sequence IN (
                    SELECT sequence FROM execution_evidence_events
                    WHERE occurred_at < ?1 ORDER BY occurred_at, sequence LIMIT ?2
                 )",
                params![cutoff, RETENTION_BATCH as i64],
            )
            .map_err(storage)?;

        // A record survives while any retained event still backs it. `record_id` is derived from
        // correlation, so the check is a correlation lookup rather than a stored back-reference.
        let deleted_records = transaction
            .execute(
                "DELETE FROM execution_evidence_records
                 WHERE occurred_at < ?1
                   AND NOT EXISTS (
                     SELECT 1 FROM execution_evidence_events event
                     WHERE event.session_id = execution_evidence_records.session_id
                       AND (
                         event.command_id = SUBSTR(execution_evidence_records.record_id, 9)
                         OR event.tool_call_id = SUBSTR(execution_evidence_records.record_id, 6)
                         OR event.agent_id = execution_evidence_records.agent_id
                       )
                   )",
                params![cutoff],
            )
            .map_err(storage)?;

        for session_id in sessions {
            let oldest: Option<String> = transaction
                .query_row(
                    "SELECT MIN(occurred_at) FROM execution_evidence_events WHERE session_id = ?1",
                    params![session_id],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            transaction
                .execute(
                    "INSERT INTO execution_evidence_coverage (
                        session_id, retention_trimmed, oldest_available_at, updated_at
                     ) VALUES (?1, 1, ?2, ?3)
                     ON CONFLICT(session_id) DO UPDATE SET
                        retention_trimmed = 1,
                        oldest_available_at = ?2,
                        updated_at = ?3",
                    params![session_id, oldest, now],
                )
                .map_err(storage)?;
        }

        transaction.commit().map_err(storage)?;
        Ok(EvidenceRetentionOutcome {
            deleted_events,
            deleted_records,
        })
    }
}

fn storage<E: std::fmt::Display>(error: E) -> EvidenceApplicationError {
    EvidenceApplicationError::Storage(error.to_string())
}

/// Reads journal rows back into domain events.
///
/// Every value goes through its domain constructor, so a row this build cannot validate stops the
/// replay instead of producing a projection from evidence that would be rejected today.
fn read_events(
    transaction: &Transaction<'_>,
    session_id: Option<&EvidenceSessionId>,
) -> Result<Vec<(ExecutionEvidenceEvent, i64)>, EvidenceApplicationError> {
    let sql = "SELECT sequence, event_id, source_context, source_event_id, schema_version, \
               session_id, run_id, trace_id, span_id, parent_span_id, operation_id, agent_id, \
               seat_id, tool_call_id, command_id, file_mutation_id, status, fidelity, \
               occurred_at, safe_payload_json, redaction_rule_ids_json \
               FROM execution_evidence_events \
               WHERE (?1 IS NULL OR session_id = ?1) ORDER BY sequence ASC";
    let mut statement = transaction.prepare(sql).map_err(storage)?;
    let rows = statement
        .query_map(params![session_id.map(EvidenceSessionId::as_str)], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, u16>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, String>(17)?,
                row.get::<_, String>(18)?,
                row.get::<_, String>(19)?,
                row.get::<_, String>(20)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;

    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let (
            sequence,
            event_id,
            source_context,
            source_event_id,
            schema_version,
            session,
            run_id,
            trace_id,
            span_id,
            parent_span_id,
            operation_id,
            agent_id,
            seat_id,
            tool_call_id,
            command_id,
            file_mutation_id,
            status,
            fidelity,
            occurred_at,
            payload_json,
            redaction_json,
        ) = row;

        let stored: StoredPayload = serde_json::from_str(&payload_json).map_err(storage)?;
        let payload = from_stored(stored).map_err(storage)?;
        let rule_ids: Vec<String> = serde_json::from_str(&redaction_json).map_err(storage)?;
        let redaction = if rule_ids.is_empty() {
            RedactionReceipt::none()
        } else {
            RedactionReceipt::applied(
                rule_ids
                    .into_iter()
                    .map(SafeReasonCode::parse)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage)?,
            )
            .map_err(storage)?
        };

        let correlation = EvidenceCorrelation {
            session_id: Some(EvidenceSessionId::parse(session).map_err(storage)?),
            run_id: run_id
                .map(ExecutionRunId::parse)
                .transpose()
                .map_err(storage)?,
            trace_id: trace_id.map(TraceId::parse).transpose().map_err(storage)?,
            span_id: span_id.map(SpanId::parse).transpose().map_err(storage)?,
            parent_span_id: parent_span_id
                .map(SpanId::parse)
                .transpose()
                .map_err(storage)?,
            operation_id: operation_id
                .map(EvidenceOperationId::parse)
                .transpose()
                .map_err(storage)?,
            agent_id: agent_id
                .map(crate::contexts::execution_observability::domain::EvidenceAgentId::parse)
                .transpose()
                .map_err(storage)?,
            seat_id: seat_id
                .map(EvidenceSeatId::parse)
                .transpose()
                .map_err(storage)?,
            tool_call_id: tool_call_id
                .map(EvidenceToolCallId::parse)
                .transpose()
                .map_err(storage)?,
            command_id: command_id
                .map(EvidenceCommandId::parse)
                .transpose()
                .map_err(storage)?,
            file_mutation_id: file_mutation_id
                .map(EvidenceFileMutationId::parse)
                .transpose()
                .map_err(storage)?,
        };

        let event = ExecutionEvidenceEvent::new(ExecutionEvidenceEventInput {
            event_id: EvidenceEventId::parse(event_id).map_err(storage)?,
            source_context: EvidenceSourceContext::parse(&source_context).ok_or_else(|| {
                EvidenceApplicationError::Storage(
                    "journal row names a source context this build does not know".to_string(),
                )
            })?,
            source_event_id: SourceEventId::parse(source_event_id).map_err(storage)?,
            schema_version,
            occurred_at,
            correlation,
            status: status.as_deref().and_then(parse_status_token),
            fidelity: parse_fidelity_token(&fidelity).ok_or_else(|| {
                EvidenceApplicationError::Storage(
                    "journal row names a fidelity this build does not know".to_string(),
                )
            })?,
            payload,
            redaction,
        })
        .map_err(storage)?;
        events.push((event, sequence));
    }
    Ok(events)
}

impl SqliteEvidenceRepository {
    fn connection_for_maintenance(
        &self,
    ) -> Result<crate::platform::database::PooledSqlite, EvidenceApplicationError> {
        self.pooled_connection()
    }
}

/// Test-only reset. Removing the projection without touching the journal is the controlled repair
/// the replay test needs; production code never calls it, because a projection is only ever
/// rebuilt from the journal that still holds every event.
#[cfg(test)]
pub(super) fn reset_projections(connection: &Connection) -> Result<(), EvidenceApplicationError> {
    connection
        .execute("DELETE FROM execution_evidence_records", [])
        .map_err(storage)?;
    Ok(())
}
