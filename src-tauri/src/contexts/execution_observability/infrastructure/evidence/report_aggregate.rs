//! Counting a session's work in SQL rather than in a consumer.
//!
//! Every query here is `GROUP BY` or an ordered offset over `execution_evidence_records`, scoped by
//! the same session/run/seat columns the page index already covers. None of them reads the journal
//! or a payload: a report needs counts, and a count taken from a projection cannot leak text that
//! the projection does not hold.
//!
//! Where a sum would be taken over rows that are not all measurable, the sum is dropped rather than
//! reported short. `SUM(duration_ms)` across a group where two records never recorded an end is a
//! smaller number that looks like a total, and no field in the answer would say so.

use super::rows::coverage_for_session;
use crate::contexts::execution_observability::application::evidence::report_models::{
    failure_codes, EvidenceCommandAggregate, EvidenceFailureAggregate, EvidenceLatencyAggregate,
    EvidenceReportAggregate, EvidenceReportQuery, EvidenceToolAggregate,
    EvidenceVerificationAggregate, MAX_EVIDENCE_TOOL_ROWS,
};
use crate::contexts::execution_observability::application::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::{EvidenceCoverageState, EvidenceSeatId};
use rusqlite::{params_from_iter, Connection, ToSql};

fn storage<E: std::fmt::Display>(error: E) -> EvidenceApplicationError {
    EvidenceApplicationError::Storage(error.to_string())
}

/// The scope predicate every query below shares, with its binds.
///
/// Built once because a report runs five statements over one scope, and five hand-written `WHERE`
/// clauses would eventually disagree about which one honours `seat_ids`.
struct Scope {
    sql: String,
    binds: Vec<Box<dyn ToSql>>,
}

fn scope_for(query: &EvidenceReportQuery) -> Scope {
    let mut sql = String::from(" WHERE session_id = ?");
    let mut binds: Vec<Box<dyn ToSql>> = vec![Box::new(query.session_id.as_str().to_string())];

    if !query.run_ids.is_empty() {
        sql.push_str(&format!(
            " AND run_id IN ({})",
            placeholders(query.run_ids.len())
        ));
        for run_id in &query.run_ids {
            binds.push(Box::new(run_id.clone()));
        }
    }
    if !query.seat_ids.is_empty() {
        sql.push_str(&format!(
            " AND seat_id IN ({})",
            placeholders(query.seat_ids.len())
        ));
        for seat_id in &query.seat_ids {
            binds.push(Box::new(EvidenceSeatId::as_str(seat_id).to_string()));
        }
    }
    // Compared as text against `occurred_at`, which is what the retention index is built on. Both
    // sides are RFC 3339 with the same offset by construction, so lexical order is chronological.
    if let Some(from) = &query.from {
        sql.push_str(" AND occurred_at >= ?");
        binds.push(Box::new(from.clone()));
    }
    if let Some(to) = &query.to {
        sql.push_str(" AND occurred_at <= ?");
        binds.push(Box::new(to.clone()));
    }
    Scope { sql, binds }
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}

fn binds(scope: &Scope) -> impl Iterator<Item = &dyn ToSql> {
    scope.binds.iter().map(|value| value.as_ref())
}

pub(super) fn report_aggregate(
    connection: &Connection,
    query: &EvidenceReportQuery,
    unattributed_gaps: u32,
) -> Result<EvidenceReportAggregate, EvidenceApplicationError> {
    let scope = scope_for(query);
    let coverage = coverage_for_session(connection, Some(&query.session_id), unattributed_gaps)?;

    let (tools, tool_tail_cut) = tool_rows(connection, &scope)?;
    let commands = command_row(connection, &scope)?;
    let verification = verification_row(connection, &scope)?;
    let failures = failure_rows(connection, &scope)?;

    Ok(EvidenceReportAggregate {
        tools,
        commands,
        verification,
        failures,
        // Two different shortfalls, one flag: the store lost or trimmed something, or the tool tail
        // did not fit. A consumer renders both as "partial", which is the honest reading of each.
        incomplete: coverage.state() != EvidenceCoverageState::Complete || tool_tail_cut,
    })
}

/// The heaviest tools first, and whether anything was left behind.
///
/// One extra row is requested so the tail can be detected without a second `COUNT(DISTINCT …)`.
fn tool_rows(
    connection: &Connection,
    scope: &Scope,
) -> Result<(Vec<EvidenceToolAggregate>, bool), EvidenceApplicationError> {
    let sql = format!(
        "SELECT tool_name, COUNT(*), \
         COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0), \
         COALESCE(SUM(duration_ms), 0), COUNT(*) - COUNT(duration_ms) \
         FROM execution_evidence_records{} AND record_kind = 'tool' AND tool_name IS NOT NULL \
         GROUP BY tool_name ORDER BY COUNT(*) DESC, tool_name ASC LIMIT {}",
        scope.sql,
        MAX_EVIDENCE_TOOL_ROWS + 1
    );
    let mut statement = connection.prepare(&sql).map_err(storage)?;
    let mut rows = statement
        .query_map(params_from_iter(binds(scope)), |row| {
            let unmeasured: i64 = row.get(4)?;
            Ok(EvidenceToolAggregate {
                tool_name: row.get(0)?,
                invocations: row.get::<_, i64>(1)?.max(0) as u32,
                failures: row.get::<_, i64>(2)?.max(0) as u32,
                // Dropped entirely when any invocation in the group went unmeasured. A partial sum
                // presented as the group's total understates it by an amount nobody can recover.
                duration_ms: (unmeasured == 0)
                    .then(|| row.get::<_, i64>(3))
                    .transpose()?
                    .map(|total| total.max(0) as u64),
            })
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;

    let tail_cut = rows.len() > MAX_EVIDENCE_TOOL_ROWS;
    rows.truncate(MAX_EVIDENCE_TOOL_ROWS);
    Ok((rows, tail_cut))
}

fn command_row(
    connection: &Connection,
    scope: &Scope,
) -> Result<EvidenceCommandAggregate, EvidenceApplicationError> {
    let sql = format!(
        "SELECT COUNT(*), \
         COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0), \
         COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0), \
         COALESCE(SUM(duration_ms), 0), COUNT(*) - COUNT(duration_ms) \
         FROM execution_evidence_records{} AND record_kind = 'command'",
        scope.sql
    );
    connection
        .query_row(&sql, params_from_iter(binds(scope)), |row| {
            let unmeasured: i64 = row.get(4)?;
            Ok(EvidenceCommandAggregate {
                total: row.get::<_, i64>(0)?.max(0) as u32,
                failed: row.get::<_, i64>(1)?.max(0) as u32,
                running: row.get::<_, i64>(2)?.max(0) as u32,
                // A session with a command still running has no total duration, which is correct:
                // it is not over, and a total that excluded it would read as though it were.
                duration_ms: (unmeasured == 0)
                    .then(|| row.get::<_, i64>(3))
                    .transpose()?
                    .map(|total| total.max(0) as u64),
            })
        })
        .map_err(storage)
}

fn verification_row(
    connection: &Connection,
    scope: &Scope,
) -> Result<EvidenceVerificationAggregate, EvidenceApplicationError> {
    let sql = format!(
        "SELECT COALESCE(SUM(verification_passed), 0), COALESCE(SUM(verification_failed), 0), \
         COALESCE(SUM(CASE WHEN verification_outcome = 'skipped' THEN 1 ELSE 0 END), 0) \
         FROM execution_evidence_records{} AND record_kind = 'verification'",
        scope.sql
    );
    connection
        .query_row(&sql, params_from_iter(binds(scope)), |row| {
            Ok(EvidenceVerificationAggregate {
                passed: row.get::<_, i64>(0)?.max(0) as u32,
                failed: row.get::<_, i64>(1)?.max(0) as u32,
                skipped: row.get::<_, i64>(2)?.max(0) as u32,
            })
        })
        .map_err(storage)
}

/// Failures under stable codes, counted from the projection's own columns.
///
/// A command that failed is split by how: a non-zero exit is the program's verdict on itself, a
/// signal is the platform killing it, and the two lead a reader to different places. Everything
/// else is counted by record kind, because the kind is the only classification the projection holds
/// that is not producer text.
fn failure_rows(
    connection: &Connection,
    scope: &Scope,
) -> Result<Vec<EvidenceFailureAggregate>, EvidenceApplicationError> {
    let sql = format!(
        "SELECT CASE \
           WHEN record_kind = 'command' AND signal IS NOT NULL THEN '{signal}' \
           WHEN record_kind = 'command' AND exit_code IS NOT NULL THEN '{exit}' \
           WHEN record_kind = 'command' THEN '{unknown}' \
           WHEN record_kind = 'tool' THEN '{tool}' \
           WHEN record_kind = 'delegation' THEN '{delegation}' \
           ELSE '{verification}' END AS reason_code, \
         COUNT(*) \
         FROM execution_evidence_records{scope} AND status = 'failed' \
         GROUP BY reason_code ORDER BY COUNT(*) DESC, reason_code ASC",
        signal = failure_codes::COMMAND_SIGNAL,
        exit = failure_codes::COMMAND_EXIT,
        unknown = failure_codes::COMMAND_UNKNOWN,
        tool = failure_codes::TOOL,
        delegation = failure_codes::DELEGATION,
        verification = failure_codes::VERIFICATION,
        scope = scope.sql,
    );
    let mut statement = connection.prepare(&sql).map_err(storage)?;
    let rows = statement
        .query_map(params_from_iter(binds(scope)), |row| {
            Ok(EvidenceFailureAggregate {
                reason_code: row.get(0)?,
                count: row.get::<_, i64>(1)?.max(0) as u32,
            })
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    Ok(rows)
}

/// Percentiles by ordered offset rather than by reading the durations into memory.
///
/// SQLite has no percentile function, and the alternative — selecting every duration and sorting in
/// Rust — is unbounded in exactly the dimension a report grows. Two extra single-row queries cost
/// an index seek each.
pub(super) fn report_latency(
    connection: &Connection,
    query: &EvidenceReportQuery,
    unattributed_gaps: u32,
) -> Result<EvidenceLatencyAggregate, EvidenceApplicationError> {
    let scope = scope_for(query);
    let coverage = coverage_for_session(connection, Some(&query.session_id), unattributed_gaps)?;

    let measured: i64 = connection
        .query_row(
            &format!(
                "SELECT COUNT(duration_ms) FROM execution_evidence_records{}",
                scope.sql
            ),
            params_from_iter(binds(&scope)),
            |row| row.get(0),
        )
        .map_err(storage)?;

    if measured <= 0 {
        // Nothing finished, so there is no distribution. Absent rather than zero: a p50 of zero
        // reports a session where every call returned instantly.
        return Ok(EvidenceLatencyAggregate {
            incomplete: coverage.state() != EvidenceCoverageState::Complete,
            ..EvidenceLatencyAggregate::default()
        });
    }

    let percentile = |fraction: i64| -> Result<Option<u64>, EvidenceApplicationError> {
        // Nearest-rank, clamped to the last element: the p95 of three samples is the third, not an
        // offset past the end that would return no row and read as "no measurement".
        let offset = ((measured * fraction) / 100).min(measured - 1).max(0);
        let sql = format!(
            "SELECT duration_ms FROM execution_evidence_records{} AND duration_ms IS NOT NULL \
             ORDER BY duration_ms ASC LIMIT 1 OFFSET {offset}",
            scope.sql
        );
        let value: Option<i64> = connection
            .query_row(&sql, params_from_iter(binds(&scope)), |row| row.get(0))
            .map_err(storage)?;
        Ok(value.map(|value| value.max(0) as u64))
    };

    let slowest: Option<i64> = connection
        .query_row(
            &format!(
                "SELECT MAX(duration_ms) FROM execution_evidence_records{}",
                scope.sql
            ),
            params_from_iter(binds(&scope)),
            |row| row.get(0),
        )
        .map_err(storage)?;

    Ok(EvidenceLatencyAggregate {
        p50_ms: percentile(50)?,
        p95_ms: percentile(95)?,
        slowest_record_duration_ms: slowest.map(|value| value.max(0) as u64),
        incomplete: coverage.state() != EvidenceCoverageState::Complete,
    })
}
