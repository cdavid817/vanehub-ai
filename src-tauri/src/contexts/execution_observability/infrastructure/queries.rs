use super::rows::{link_from_parts, EventRow, RunRow, SpanRow};
use super::storage_mapping::storage_error;
use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    ExecutionLink, ExecutionRun, ExecutionRunId, ExecutionTimeline, Page, PageRequest,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const RUN_COLUMNS: &str = "run_id, trace_id, root_span_id, source, source_id, status, capture_policy, started_at, ended_at, error_classification, session_id, user_message_id, assistant_message_id, operation_id, agent_id, provider_session_id, attributes_json";
const SPAN_COLUMNS: &str = "spans.run_id, spans.span_id, spans.trace_id, spans.parent_span_id, spans.name, spans.status, spans.fidelity, spans.started_at, spans.ended_at, spans.error_classification, spans.attributes_json, runs.capture_policy";

#[derive(Debug, Serialize, Deserialize)]
struct RunCursor {
    started_at: String,
    run_id: String,
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

    pub(crate) fn timeline(
        &self,
        run_id: &ExecutionRunId,
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

        let mut statement = connection
            .prepare(
                "SELECT run_id, span_id, sequence, name, timestamp, attributes_json
                 FROM execution_events WHERE run_id = ?1
                 ORDER BY timestamp, span_id, sequence LIMIT 5000",
            )
            .map_err(|error| storage_error(error.to_string()))?;
        let events = statement
            .query_map([run_id.as_str()], EventRow::read)
            .map_err(|error| storage_error(error.to_string()))?
            .map(|row| {
                row.map_err(|error| storage_error(error.to_string()))?
                    .into_domain()
            })
            .collect::<Result<Vec<_>, ExecutionTelemetryError>>()?;
        Ok(Some(ExecutionTimeline { run, spans, events }))
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
