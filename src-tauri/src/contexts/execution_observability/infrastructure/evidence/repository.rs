use super::cursor::{filter_fingerprint, RecordCursor};
use super::payload_row::to_stored;
use super::projection::{project_event, ProjectionUpdate};
use super::rows::{coverage_for_session, read_record_row, record_columns};
use crate::contexts::execution_observability::application::evidence::models::{
    EvidenceCorrelationCounts, EvidenceRecordPage, EvidenceSubscriptionBootstrap,
    ExecutionRecordDetailQuery, ExecutionRecordDetailView, ExecutionRecordProjection,
    ExecutionRecordQuery, UnownedSummarySource, WorkspaceEvidenceSummary,
    WorkspaceEvidenceSummaryQuery,
};
use crate::contexts::execution_observability::application::evidence::ports::{
    EvidenceAppendOutcome, EvidenceRepositoryPort, EvidenceRetentionSummary,
};
use crate::contexts::execution_observability::application::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::{
    fidelity_token, reason_codes, status_token, EvidenceCoverageState, EvidenceSeatId,
    EvidenceSessionId, ExecutionEvidenceEvent, ExecutionStatus, SafeReasonCode,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, ToSql};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SqliteEvidenceRepository {
    database: NativeDatabase,
    /// Evidence lost with no session to attribute it to.
    ///
    /// Shared with every clone, because this is a fact about the store as a whole rather than
    /// about one handle to it. While it is non-zero no session may report `complete`: the one that
    /// lost the evidence is among them, and nothing here can say which.
    unattributed_gaps: Arc<AtomicU32>,
}

impl SqliteEvidenceRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self {
            database,
            unattributed_gaps: Arc::new(AtomicU32::new(0)),
        }
    }

    pub(super) fn unattributed_gaps(&self) -> u32 {
        self.unattributed_gaps.load(Ordering::Relaxed)
    }

    fn connection(&self) -> Result<PooledSqlite, EvidenceApplicationError> {
        self.pooled_connection()
    }

    pub(super) fn pooled_connection(&self) -> Result<PooledSqlite, EvidenceApplicationError> {
        self.database
            .connection()
            .map_err(|error| EvidenceApplicationError::Storage(error.to_string()))
    }
}

fn storage<E: std::fmt::Display>(error: E) -> EvidenceApplicationError {
    EvidenceApplicationError::Storage(error.to_string())
}

impl EvidenceRepositoryPort for SqliteEvidenceRepository {
    /// One transaction covers the journal insert, the projection update, and the coverage
    /// metadata. A projection failure rolls the event back with it: a journal row whose projection
    /// never landed would be invisible to every query while still counting toward completeness,
    /// which is worse than not having recorded it.
    ///
    /// Nothing is published here. The caller publishes after this returns, so a failed notice can
    /// never roll back a committed event.
    fn append(
        &self,
        event: &ExecutionEvidenceEvent,
        fingerprint: &str,
        recorded_at: &str,
    ) -> Result<EvidenceAppendOutcome, EvidenceApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage)?;

        let existing: Option<(i64, String)> = transaction
            .query_row(
                "SELECT sequence, content_fingerprint FROM execution_evidence_events \
                 WHERE source_context = ?1 AND source_event_id = ?2",
                params![
                    event.source_context().as_str(),
                    event.source_event_id().as_str()
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;

        if let Some((sequence, stored_fingerprint)) = existing {
            // One indexed lookup decides retry from conflict. The original row always wins: it is
            // the assertion that was actually accepted, and rewriting it would silently change
            // history a reader may already have paged through.
            if stored_fingerprint == fingerprint {
                transaction.commit().map_err(storage)?;
                return Ok(EvidenceAppendOutcome::IdenticalDuplicate { sequence });
            }
            mark_conflict(&transaction, event, recorded_at)?;
            transaction.commit().map_err(storage)?;
            return Ok(EvidenceAppendOutcome::Conflict);
        }

        let payload_json = serde_json::to_string(&to_stored(event.payload())).map_err(storage)?;
        let redaction_rules = serde_json::to_string(
            &event
                .redaction()
                .rule_ids()
                .iter()
                .map(SafeReasonCode::as_str)
                .collect::<Vec<_>>(),
        )
        .map_err(storage)?;
        let correlation = event.correlation();
        let session_id = correlation
            .session()
            .ok_or(EvidenceApplicationError::Storage(
                "evidence event reached storage without a session".to_string(),
            ))?
            .as_str()
            .to_string();

        transaction
            .execute(
                r#"INSERT INTO execution_evidence_events (
                    event_id, source_context, source_event_id, schema_version, content_fingerprint,
                    session_id, run_id, trace_id, span_id, parent_span_id, operation_id, agent_id,
                    seat_id, tool_call_id, command_id, file_mutation_id, kind, status, fidelity,
                    occurred_at, safe_payload_json, redaction_applied, redaction_rule_ids_json,
                    created_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20, ?21, ?22, ?23, ?24
                )"#,
                params![
                    event.event_id().as_str(),
                    event.source_context().as_str(),
                    event.source_event_id().as_str(),
                    event.schema_version(),
                    fingerprint,
                    session_id,
                    correlation.run_id.as_ref().map(|value| value.as_str()),
                    correlation.trace_id.as_ref().map(|value| value.as_str()),
                    correlation.span_id.as_ref().map(|value| value.as_str()),
                    correlation
                        .parent_span_id
                        .as_ref()
                        .map(|value| value.as_str()),
                    correlation
                        .operation_id
                        .as_ref()
                        .map(|value| value.as_str()),
                    correlation.agent_id.as_ref().map(|value| value.as_str()),
                    correlation.seat_id.as_ref().map(|value| value.as_str()),
                    correlation
                        .tool_call_id
                        .as_ref()
                        .map(|value| value.as_str()),
                    correlation.command_id.as_ref().map(|value| value.as_str()),
                    correlation
                        .file_mutation_id
                        .as_ref()
                        .map(|value| value.as_str()),
                    event.kind().as_str(),
                    event.status().map(status_token),
                    fidelity_token(event.fidelity()),
                    event.occurred_at(),
                    payload_json,
                    event.redaction().is_applied() as i64,
                    redaction_rules,
                    recorded_at,
                ],
            )
            .map_err(storage)?;
        let sequence = transaction.last_insert_rowid();

        if let Some(update) = project_event(event, sequence) {
            apply_projection(&transaction, &update)?;
        }
        touch_coverage(&transaction, &session_id, event.occurred_at(), recorded_at)?;

        transaction.commit().map_err(storage)?;
        Ok(EvidenceAppendOutcome::Appended { sequence })
    }

    fn list_records(
        &self,
        query: &ExecutionRecordQuery,
    ) -> Result<EvidenceRecordPage, EvidenceApplicationError> {
        let fingerprint = filter_fingerprint(query);
        let cursor = query
            .cursor
            .as_deref()
            .map(|encoded| RecordCursor::decode(encoded, &fingerprint))
            .transpose()?;

        let mut sql = format!(
            "SELECT {} FROM execution_evidence_records WHERE 1 = 1",
            record_columns()
        );
        let mut binds: Vec<Box<dyn ToSql>> = Vec::new();
        push_scope(&mut sql, &mut binds, query);
        push_filters(&mut sql, &mut binds, query);

        // The keyset predicate uses exactly the pair the ordering uses, so a row appended after
        // the cursor was issued sorts ahead of the boundary and is simply not part of this page.
        if let Some(cursor) = &cursor {
            sql.push_str(" AND (occurred_at < ?  OR (occurred_at = ? AND record_id < ?))");
            binds.push(Box::new(cursor.occurred_at.clone()));
            binds.push(Box::new(cursor.occurred_at.clone()));
            binds.push(Box::new(cursor.record_id.clone()));
        }
        sql.push_str(" ORDER BY occurred_at DESC, record_id DESC LIMIT ?");
        // One extra row tells us whether a further page exists without a second count query.
        binds.push(Box::new((query.limit + 1) as i64));

        let connection = self.connection()?;
        let mut statement = connection.prepare(&sql).map_err(storage)?;
        let mut rows = statement
            .query_map(
                params_from_iter(binds.iter().map(|value| value.as_ref())),
                read_record_row,
            )
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .collect::<Result<Vec<ExecutionRecordProjection>, EvidenceApplicationError>>()?;

        let has_more = rows.len() > query.limit;
        rows.truncate(query.limit);
        let next_cursor = has_more.then(|| rows.last()).flatten().map(|record| {
            RecordCursor {
                occurred_at: record.occurred_at.clone(),
                record_id: record.record_id.clone(),
                filter_fingerprint: fingerprint.clone(),
            }
            .encode()
        });

        let session = query.scope.session_id.as_ref();
        let coverage = coverage_for_session(&connection, session, self.unattributed_gaps())?
            .with_truncated(has_more);
        Ok(EvidenceRecordPage {
            items: rows,
            next_cursor,
            coverage,
        })
    }

    fn record_detail(
        &self,
        query: &ExecutionRecordDetailQuery,
    ) -> Result<ExecutionRecordDetailView, EvidenceApplicationError> {
        let connection = self.connection()?;
        let sql = format!(
            "SELECT {} FROM execution_evidence_records WHERE session_id = ?1 AND record_id = ?2",
            record_columns()
        );
        let record = connection
            .query_row(
                &sql,
                params![query.session_id.as_str(), query.record_id],
                read_record_row,
            )
            .optional()
            .map_err(storage)?
            .transpose()?
            .ok_or(EvidenceApplicationError::RecordNotFound)?;

        let counts = self.correlation_counts(&query.session_id, record.run_id.as_deref())?;
        let error_reason_code = matches!(record.status, ExecutionStatus::Failed)
            .then(|| "execution_failed".to_string());
        Ok(ExecutionRecordDetailView {
            record,
            counts,
            error_reason_code,
        })
    }

    fn summary(
        &self,
        query: &WorkspaceEvidenceSummaryQuery,
    ) -> Result<WorkspaceEvidenceSummary, EvidenceApplicationError> {
        let connection = self.connection()?;
        let coverage = coverage_for_session(
            &connection,
            Some(&query.session_id),
            self.unattributed_gaps(),
        )?;

        let (running, failed): (i64, i64) = connection
            .query_row(
                "SELECT \
                 COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0), \
                 COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) \
                 FROM execution_evidence_records WHERE session_id = ?1",
                params![query.session_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(storage)?;

        let (passed, failed_checks): (i64, i64) = connection
            .query_row(
                "SELECT COALESCE(SUM(verification_passed), 0), COALESCE(SUM(verification_failed), 0) \
                 FROM execution_evidence_records \
                 WHERE session_id = ?1 AND record_kind = 'verification'",
                params![query.session_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(storage)?;

        let latest_run: Option<(String, String, String)> = connection
            .query_row(
                "SELECT run_id, status, occurred_at FROM execution_evidence_records \
                 WHERE session_id = ?1 AND run_id IS NOT NULL \
                 ORDER BY occurred_at DESC, record_id DESC LIMIT 1",
                params![query.session_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?;

        Ok(WorkspaceEvidenceSummary {
            session_id: query.session_id.clone(),
            generated_at: coverage
                .newest_available_at()
                .unwrap_or_default()
                .to_string(),
            coverage,
            run_status: latest_run.as_ref().and_then(|(_, status, _)| {
                crate::contexts::execution_observability::domain::parse_status_token(status)
            }),
            run_id: latest_run.as_ref().map(|(run_id, _, _)| run_id.clone()),
            run_started_at: latest_run.map(|(_, _, occurred_at)| occurred_at),
            running_records: running.max(0) as u32,
            failed_records: failed.max(0) as u32,
            verification_passed: passed.max(0) as u32,
            verification_failed: failed_checks.max(0) as u32,
            // Logs, Shells, changes, review progress, and usage belong to other contexts. Group 3
            // owns no port to them, so they are reported as not-owned rather than as zero — a
            // definitive zero here would render as "nothing happened" in a badge.
            unowned_sources: unowned_sources(),
        })
    }

    fn correlation_counts(
        &self,
        session_id: &EvidenceSessionId,
        run_id: Option<&str>,
    ) -> Result<EvidenceCorrelationCounts, EvidenceApplicationError> {
        let connection = self.connection()?;
        let mut counts = EvidenceCorrelationCounts::default();
        let mut statement = connection
            .prepare(
                "SELECT record_kind, COUNT(*) FROM execution_evidence_records \
                 WHERE session_id = ?1 AND (?2 IS NULL OR run_id = ?2) GROUP BY record_kind",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![session_id.as_str(), run_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(storage)?;
        for row in rows {
            let (kind, count) = row.map_err(storage)?;
            let count = count.max(0) as u32;
            match kind.as_str() {
                "command" => counts.commands = count,
                "tool" => counts.tools = count,
                "delegation" => counts.delegations = count,
                "verification" => counts.verifications = count,
                _ => {}
            }
        }

        let (files, usage): (i64, i64) = connection
            .query_row(
                "SELECT \
                 COALESCE(SUM(CASE WHEN kind = 'file.mutation.observed' THEN 1 ELSE 0 END), 0), \
                 COALESCE(SUM(CASE WHEN kind = 'usage.observed' THEN 1 ELSE 0 END), 0) \
                 FROM execution_evidence_events \
                 WHERE session_id = ?1 AND (?2 IS NULL OR run_id = ?2)",
                params![session_id.as_str(), run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(storage)?;
        counts.file_mutations = files.max(0) as u32;
        counts.usage_observations = usage.max(0) as u32;
        Ok(counts)
    }

    /// The sequence the store has committed through. A subscriber that starts from it applies only
    /// notices newer than the page it already fetched, which is what closes the race between the
    /// listener being registered and the first page arriving.
    fn subscription_bootstrap(
        &self,
        session_id: &EvidenceSessionId,
    ) -> Result<EvidenceSubscriptionBootstrap, EvidenceApplicationError> {
        let connection = self.connection()?;
        let watermark: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) FROM execution_evidence_events WHERE session_id = ?1",
                params![session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        Ok(EvidenceSubscriptionBootstrap {
            session_id: session_id.clone(),
            watermark_sequence: watermark,
            coverage: coverage_for_session(
                &connection,
                Some(session_id),
                self.unattributed_gaps(),
            )?,
        })
    }

    /// Two indexed aggregates over the whole store, deliberately not a scan.
    ///
    /// `MAX(sequence)` is served by the primary key and the projected-kind filter narrows it;
    /// `MAX(last_sequence)` reads one column of the projection. A current projection therefore
    /// costs two index lookups at startup rather than a pass over every event ever recorded.
    fn report_unattributed_gap(&self, count: u32) {
        // Saturating: a wrapped count would read as fewer losses, and what matters is that a
        // non-zero count degrades every session's coverage.
        self.unattributed_gaps
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_add(count))
            })
            .ok();
    }

    fn projection_is_stale(&self) -> Result<bool, EvidenceApplicationError> {
        let connection = self.connection()?;
        let journal_head: i64 = connection
            .query_row(
                &format!(
                    "SELECT COALESCE(MAX(sequence), 0) FROM execution_evidence_events \
                     WHERE kind IN ({})",
                    super::rows::PROJECTED_KINDS
                ),
                [],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let projected_head: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(last_sequence), 0) FROM execution_evidence_records",
                [],
                |row| row.get(0),
            )
            .map_err(storage)?;
        // A missing or emptied projection reads as 0, so "projection absent behind a non-empty
        // journal" needs no separate check — and a projection-shape change ships as a migration
        // that clears the table, which lands the same way rather than needing a stored revision.
        Ok(projected_head < journal_head)
    }

    fn replay_projections(
        &self,
        session_id: Option<&EvidenceSessionId>,
    ) -> Result<usize, EvidenceApplicationError> {
        SqliteEvidenceRepository::replay_projections(self, session_id)
    }

    fn maintain_retention(
        &self,
        cutoff: &str,
        now: &str,
    ) -> Result<EvidenceRetentionSummary, EvidenceApplicationError> {
        let outcome = SqliteEvidenceRepository::maintain_retention(self, cutoff, now)?;
        Ok(EvidenceRetentionSummary {
            deleted_events: outcome.deleted_events,
            deleted_records: outcome.deleted_records,
        })
    }
}

pub(super) fn unowned_sources() -> Vec<UnownedSummarySource> {
    ["logs", "shells", "changes", "review", "usage"]
        .into_iter()
        .map(|source| UnownedSummarySource {
            source,
            coverage_state: EvidenceCoverageState::Unavailable,
            reason_code: reason_codes::SOURCE_NOT_OWNED,
        })
        .collect()
}

fn push_scope(sql: &mut String, binds: &mut Vec<Box<dyn ToSql>>, query: &ExecutionRecordQuery) {
    let scope = &query.scope;
    if let Some(session_id) = &scope.session_id {
        sql.push_str(" AND session_id = ?");
        binds.push(Box::new(session_id.as_str().to_string()));
    }
    if let Some(seat_id) = &scope.seat_id {
        sql.push_str(" AND seat_id = ?");
        binds.push(Box::new(EvidenceSeatId::as_str(seat_id).to_string()));
    }
    for (column, value) in [
        ("run_id", scope.run_id.as_deref()),
        ("trace_id", scope.trace_id.as_deref()),
        ("span_id", scope.span_id.as_deref()),
        ("operation_id", scope.operation_id.as_deref()),
    ] {
        if let Some(value) = value {
            sql.push_str(&format!(" AND {column} = ?"));
            binds.push(Box::new(value.to_string()));
        }
    }
    if let Some(command_id) = &scope.command_id {
        sql.push_str(" AND record_id = ?");
        binds.push(Box::new(format!("command:{command_id}")));
    }
}

fn push_filters(sql: &mut String, binds: &mut Vec<Box<dyn ToSql>>, query: &ExecutionRecordQuery) {
    let filters = &query.filters;
    if !filters.kinds.is_empty() {
        sql.push_str(&format!(
            " AND record_kind IN ({})",
            placeholders(filters.kinds.len())
        ));
        for kind in &filters.kinds {
            binds.push(Box::new(kind.as_str().to_string()));
        }
    }
    if !filters.statuses.is_empty() {
        sql.push_str(&format!(
            " AND status IN ({})",
            placeholders(filters.statuses.len())
        ));
        for status in &filters.statuses {
            binds.push(Box::new(status_token(*status).to_string()));
        }
    }
    if !filters.fidelities.is_empty() {
        sql.push_str(&format!(
            " AND fidelity IN ({})",
            placeholders(filters.fidelities.len())
        ));
        for fidelity in &filters.fidelities {
            binds.push(Box::new(fidelity_token(*fidelity).to_string()));
        }
    }
    if let Some(search) = filters
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        // Matches only the already-redacted display fields the projection holds; the journal
        // payload is never scanned for a page.
        //
        // `ESCAPE` is not decoration. The pattern below escapes `%` and `_` so a term cannot smuggle
        // a wildcard in, and without declaring the escape character SQLite reads the backslash as a
        // literal — so `read_file` searched for `read\_file` and matched nothing. Tool names are
        // overwhelmingly snake_case, which made that the common case rather than the edge one.
        sql.push_str(
            " AND (COALESCE(redacted_display, '') LIKE ? ESCAPE '\\' \
             OR COALESCE(tool_name, '') LIKE ? ESCAPE '\\' \
             OR COALESCE(verification_name, '') LIKE ? ESCAPE '\\')",
        );
        let pattern = format!("%{}%", search.replace('%', "\\%").replace('_', "\\_"));
        for _ in 0..3 {
            binds.push(Box::new(pattern.clone()));
        }
    }
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Applies a projection update behind a monotonic guard.
///
/// `last_sequence` is what keeps a late-arriving start from overwriting the completion that
/// already landed. Out-of-order delivery is normal — producers publish through a bounded queue —
/// so the guard is the difference between a record that settles on its terminal state and one that
/// flickers back to running.
fn apply_projection(
    connection: &Connection,
    update: &ProjectionUpdate,
) -> Result<(), EvidenceApplicationError> {
    super::projection::upsert(connection, update).map_err(storage)
}

fn touch_coverage(
    connection: &Connection,
    session_id: &str,
    occurred_at: &str,
    recorded_at: &str,
) -> Result<(), EvidenceApplicationError> {
    connection
        .execute(
            "INSERT INTO execution_evidence_coverage (
                session_id, oldest_available_at, newest_available_at, updated_at
             ) VALUES (?1, ?2, ?2, ?3)
             ON CONFLICT(session_id) DO UPDATE SET
                oldest_available_at = MIN(COALESCE(oldest_available_at, ?2), ?2),
                newest_available_at = MAX(COALESCE(newest_available_at, ?2), ?2),
                updated_at = ?3",
            params![session_id, occurred_at, recorded_at],
        )
        .map_err(storage)?;
    Ok(())
}

/// Records that one version of an event was refused.
///
/// The counter is what turns a conflict into visible partial coverage. Nothing about either
/// payload is stored: a conflict is usually a producer bug, and writing both versions or a diff
/// between them would put the content the journal declined into a second place.
fn mark_conflict(
    connection: &Connection,
    event: &ExecutionEvidenceEvent,
    recorded_at: &str,
) -> Result<(), EvidenceApplicationError> {
    let session_id = event
        .correlation()
        .session()
        .map(|session| session.as_str().to_string())
        .unwrap_or_default();
    connection
        .execute(
            "INSERT INTO execution_evidence_coverage (session_id, conflict_count, updated_at)
             VALUES (?1, 1, ?2)
             ON CONFLICT(session_id) DO UPDATE SET
                conflict_count = conflict_count + 1,
                updated_at = ?2",
            params![session_id, recorded_at],
        )
        .map_err(storage)?;
    Ok(())
}
