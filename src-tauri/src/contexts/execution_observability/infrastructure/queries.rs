use super::rows::{link_from_parts, EventRow, RunRow, SpanRow};
use super::storage_mapping::storage_error;
use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    EventCoverage, ExecutionEvent, ExecutionLink, ExecutionRun, ExecutionRunId, ExecutionTimeline,
    Page, PageRequest,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const RUN_COLUMNS: &str = "run_id, trace_id, root_span_id, source, source_id, status, capture_policy, started_at, ended_at, error_classification, session_id, user_message_id, assistant_message_id, operation_id, agent_id, provider_session_id, attributes_json";
const SPAN_COLUMNS: &str = "spans.run_id, spans.span_id, spans.trace_id, spans.parent_span_id, spans.name, spans.status, spans.fidelity, spans.started_at, spans.ended_at, spans.error_classification, spans.attributes_json, runs.capture_policy";

/// How many events one timeline response carries.
///
/// Unchanged from the bound this response has always had. What changed is that exceeding it is now
/// reported instead of being silent, so a reader can tell a complete record from a clipped one.
pub(crate) const TIMELINE_EVENT_PAGE_SIZE: usize = 5000;

#[derive(Debug, Serialize, Deserialize)]
struct RunCursor {
    started_at: String,
    run_id: String,
}

/// Where an event page stopped, in the order events are read.
///
/// Carries all three ordering columns because any two of them can repeat: several events can share
/// a timestamp, and a span's sequence restarts per span. A cursor on fewer columns would either
/// skip events or repeat them at every page boundary.
#[derive(Debug, Serialize, Deserialize)]
struct EventCursor {
    timestamp: String,
    span_id: String,
    sequence: i64,
}

impl SqliteExecutionTimelineRepository {
    pub(crate) fn list_runs(
        &self,
        request: &PageRequest,
        session_id: Option<&str>,
    ) -> Result<Page<ExecutionRun>, ExecutionTelemetryError> {
        let connection = self.connection()?;
        let cursor = request
            .page_token
            .as_deref()
            .map(decode_cursor)
            .transpose()?;
        let cursor_time = cursor.as_ref().map(|value| value.started_at.as_str());
        let cursor_run = cursor.as_ref().map(|value| value.run_id.as_str());
        let sql = format!(
            "SELECT {RUN_COLUMNS} FROM execution_runs
             WHERE (?1 IS NULL OR session_id = ?1)
               AND (?2 IS NULL OR started_at < ?2 OR (started_at = ?2 AND run_id < ?3))
             ORDER BY started_at DESC, run_id DESC LIMIT ?4"
        );
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| storage_error(error.to_string()))?;
        let mut rows = statement
            .query_map(
                params![
                    session_id,
                    cursor_time,
                    cursor_run,
                    i64::from(request.limit) + 1
                ],
                RunRow::read,
            )
            .map_err(|error| storage_error(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| storage_error(error.to_string()))?;

        let has_more = rows.len() > usize::from(request.limit);
        if has_more {
            rows.truncate(usize::from(request.limit));
        }
        let next_page_token = if has_more {
            rows.last()
                .map(RunRow::cursor)
                .map(|(started_at, run_id)| encode_cursor(started_at, run_id))
                .transpose()?
        } else {
            None
        };
        let items = rows
            .into_iter()
            .map(|row| {
                let run_id = row.cursor().1.to_string();
                let links = load_links(&connection, &run_id, None)?;
                row.into_domain(links)
            })
            .collect::<Result<Vec<_>, ExecutionTelemetryError>>()?;
        Ok(Page {
            items,
            next_page_token,
        })
    }

    /// A timeline with the default event bound and no continuation.
    pub(crate) fn timeline(
        &self,
        run_id: &ExecutionRunId,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError> {
        self.timeline_page(run_id, TIMELINE_EVENT_PAGE_SIZE, None)
    }

    /// A timeline whose event page is bounded and resumable.
    ///
    /// The bound is a parameter rather than a constant read inside so a test can exercise the
    /// paging itself without inserting tens of thousands of rows to reach the production bound.
    /// The logic under test is identical at any bound; only the row count changes.
    pub(crate) fn timeline_page(
        &self,
        run_id: &ExecutionRunId,
        event_limit: usize,
        event_page_token: Option<&str>,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError> {
        let connection = self.connection()?;
        let sql = format!("SELECT {RUN_COLUMNS} FROM execution_runs WHERE run_id = ?1");
        let run = connection
            .query_row(&sql, [run_id.as_str()], RunRow::read)
            .optional()
            .map_err(|error| storage_error(error.to_string()))?;
        let Some(run) = run else {
            return Ok(None);
        };
        let run_links = load_links(&connection, run_id.as_str(), None)?;
        let run = run.into_domain(run_links)?;

        let sql = format!(
            "SELECT {SPAN_COLUMNS} FROM execution_spans spans
             JOIN execution_runs runs ON runs.run_id = spans.run_id
             WHERE spans.run_id = ?1 ORDER BY spans.started_at, spans.span_id"
        );
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| storage_error(error.to_string()))?;
        let span_rows = statement
            .query_map([run_id.as_str()], SpanRow::read)
            .map_err(|error| storage_error(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| storage_error(error.to_string()))?;
        let spans = span_rows
            .into_iter()
            .map(|row| {
                let links = load_links(&connection, run_id.as_str(), Some(row.span_id()))?;
                row.into_domain(links)
            })
            .collect::<Result<Vec<_>, ExecutionTelemetryError>>()?;

        let cursor = event_page_token.map(decode_event_cursor).transpose()?;
        let (cursor_timestamp, cursor_span, cursor_sequence) = match cursor.as_ref() {
            Some(value) => (
                Some(value.timestamp.as_str()),
                Some(value.span_id.as_str()),
                Some(value.sequence),
            ),
            None => (None, None, None),
        };
        // One row past the bound, purely to learn whether another exists. Asking for exactly the
        // bound cannot distinguish a run with that many events from one with more -- which was the
        // defect: the query returned 5000 rows and no way for a reader to tell the two apart.
        // Two statements rather than one with an `?2 IS NULL OR ...` guard, because that guard is
        // not free: a bound parameter inside an OR cannot be folded into an index range, so SQLite
        // seeks on `run_id` alone and re-walks the run from its first event on every page. Measured
        // over 60k events the guarded form costs 165ms against 75ms for these two. The plan text is
        // where the difference shows -- `(run_id=?)` versus
        // `(run_id=? AND (timestamp,span_id,sequence)>(?,?,?))`.
        let over_limit = i64::try_from(event_limit.saturating_add(1))
            .map_err(|_| storage_error("event page size exceeds SQLite range"))?;
        let mut statement = connection
            .prepare(if cursor.is_some() {
                "SELECT run_id, span_id, sequence, name, timestamp, attributes_json
                 FROM execution_events WHERE run_id = ?1
                   AND (?2, ?3, ?4) < (timestamp, span_id, sequence)
                 ORDER BY timestamp, span_id, sequence LIMIT ?5"
            } else {
                "SELECT run_id, span_id, sequence, name, timestamp, attributes_json
                 FROM execution_events WHERE run_id = ?1
                 ORDER BY timestamp, span_id, sequence LIMIT ?2"
            })
            .map_err(|error| storage_error(error.to_string()))?;
        let rows = if cursor.is_some() {
            statement.query_map(
                params![
                    run_id.as_str(),
                    cursor_timestamp,
                    cursor_span,
                    cursor_sequence,
                    over_limit
                ],
                EventRow::read,
            )
        } else {
            statement.query_map(params![run_id.as_str(), over_limit], EventRow::read)
        };
        let mut events = rows
            .map_err(|error| storage_error(error.to_string()))?
            .map(|row| {
                row.map_err(|error| storage_error(error.to_string()))?
                    .into_domain()
            })
            .collect::<Result<Vec<_>, ExecutionTelemetryError>>()?;

        let event_coverage = if events.len() > event_limit {
            // The cursor is the last row that survives truncation, read before truncating so the
            // index is unambiguous.
            let token = event_limit
                .checked_sub(1)
                .and_then(|index| events.get(index))
                .map(encode_event_cursor)
                .transpose()?;
            events.truncate(event_limit);
            match token {
                Some(token) => EventCoverage::truncated_at(token),
                // A zero bound: rows exist, none fit, and there is no last-returned row to resume
                // after. Reporting `complete` here would be the exact false claim this type exists
                // to prevent -- an empty list presented as the whole record. The bound is a caller
                // parameter, so this is reachable by asking for a page that can hold nothing.
                None => {
                    return Err(storage_error(
                        "event page size must leave room for at least one event",
                    ))
                }
            }
        } else {
            EventCoverage::complete()
        };
        Ok(Some(ExecutionTimeline {
            run,
            spans,
            events,
            event_coverage,
        }))
    }
}

fn load_links(
    connection: &Connection,
    run_id: &str,
    span_id: Option<&str>,
) -> Result<Vec<ExecutionLink>, ExecutionTelemetryError> {
    let mut statement = connection
        .prepare(
            "SELECT linked_run_id, linked_trace_id, linked_span_id, relationship
             FROM execution_links
             WHERE run_id = ?1 AND ((?2 IS NULL AND span_id IS NULL) OR span_id = ?2)
             ORDER BY relationship, linked_run_id, linked_span_id",
        )
        .map_err(|error| storage_error(error.to_string()))?;
    let links = statement
        .query_map(params![run_id, span_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|error| storage_error(error.to_string()))?
        .map(|row| {
            let (run_id, trace_id, span_id, relationship) =
                row.map_err(|error| storage_error(error.to_string()))?;
            link_from_parts(run_id, trace_id, span_id, relationship)
        })
        .collect();
    links
}

fn encode_cursor(started_at: &str, run_id: &str) -> Result<String, ExecutionTelemetryError> {
    let bytes = serde_json::to_vec(&RunCursor {
        started_at: started_at.to_string(),
        run_id: run_id.to_string(),
    })
    .map_err(|error| storage_error(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(value: &str) -> Result<RunCursor, ExecutionTelemetryError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| storage_error("invalid execution page token"))?;
    serde_json::from_slice(&bytes).map_err(|_| storage_error("invalid execution page token"))
}

fn encode_event_cursor(event: &ExecutionEvent) -> Result<String, ExecutionTelemetryError> {
    let bytes = serde_json::to_vec(&EventCursor {
        timestamp: event.timestamp.clone(),
        span_id: event.span_id.as_str().to_string(),
        sequence: i64::try_from(event.sequence)
            .map_err(|_| storage_error("event sequence exceeds SQLite range"))?,
    })
    .map_err(|error| storage_error(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_event_cursor(value: &str) -> Result<EventCursor, ExecutionTelemetryError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| storage_error("invalid execution event page token"))?;
    serde_json::from_slice(&bytes).map_err(|_| storage_error("invalid execution event page token"))
}
