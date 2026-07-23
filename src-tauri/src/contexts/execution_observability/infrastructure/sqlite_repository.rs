use super::storage_mapping::{
    attributes_json, capture_value, fidelity_value, source_parts, status_value, storage_error,
};
use crate::contexts::execution_observability::application::{
    ExecutionTelemetryError, ExecutionTelemetryPort,
};
use crate::contexts::execution_observability::domain::{
    ExecutionEvent, ExecutionRun, ExecutionRunId, ExecutionSpan, ExecutionStatus, SpanId,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection};

#[derive(Clone)]
pub(crate) struct SqliteExecutionTimelineRepository {
    database: NativeDatabase,
}

impl SqliteExecutionTimelineRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(super) fn connection(&self) -> Result<PooledSqlite, ExecutionTelemetryError> {
        self.database
            .connection()
            .map_err(|error| storage_error(error.to_string()))
    }

    fn insert_run(&self, run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
        run.validate()
            .map_err(|error| storage_error(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| storage_error(error.to_string()))?;
        let (source, source_id) = source_parts(&run.source);
        transaction
            .execute(
                r#"INSERT INTO execution_runs (
                    run_id, trace_id, root_span_id, source, source_id, status, capture_policy,
                    started_at, ended_at, error_classification, session_id, user_message_id,
                    assistant_message_id, operation_id, agent_id, provider_session_id, attributes_json
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
                ) ON CONFLICT(run_id) DO UPDATE SET
                    status = excluded.status,
                    ended_at = COALESCE(excluded.ended_at, execution_runs.ended_at),
                    assistant_message_id = COALESCE(excluded.assistant_message_id, execution_runs.assistant_message_id),
                    operation_id = COALESCE(excluded.operation_id, execution_runs.operation_id),
                    provider_session_id = COALESCE(excluded.provider_session_id, execution_runs.provider_session_id),
                    attributes_json = excluded.attributes_json"#,
                params![
                    run.context.run_id.as_str(),
                    run.context.trace_id.as_str(),
                    run.context.span_id.as_str(),
                    source,
                    source_id,
                    status_value(run.status),
                    capture_value(run.context.capture_policy),
                    run.started_at,
                    run.ended_at,
                    run.error_classification,
                    run.session_id,
                    run.user_message_id,
                    run.assistant_message_id,
                    run.operation_id,
                    run.agent_id,
                    run.provider_session_id,
                    attributes_json(&run.attributes)?,
                ],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .execute(
                "DELETE FROM execution_links WHERE run_id = ?1 AND span_id IS NULL",
                [run.context.run_id.as_str()],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        insert_links(&transaction, run.context.run_id.as_str(), None, &run.links)?;
        transaction
            .commit()
            .map_err(|error| storage_error(error.to_string()))
    }

    fn insert_span(&self, span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
        span.validate()
            .map_err(|error| storage_error(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .execute(
                r#"INSERT INTO execution_spans (
                    run_id, span_id, trace_id, parent_span_id, name, status, fidelity,
                    started_at, ended_at, error_classification, attributes_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(run_id, span_id) DO UPDATE SET
                    status = excluded.status,
                    ended_at = COALESCE(excluded.ended_at, execution_spans.ended_at),
                    error_classification = COALESCE(excluded.error_classification, execution_spans.error_classification),
                    attributes_json = excluded.attributes_json"#,
                params![
                    span.context.run_id.as_str(),
                    span.context.span_id.as_str(),
                    span.context.trace_id.as_str(),
                    span.parent_span_id.as_ref().map(SpanId::as_str),
                    span.name,
                    status_value(span.status),
                    fidelity_value(span.fidelity),
                    span.started_at,
                    span.ended_at,
                    span.error_classification,
                    attributes_json(&span.attributes)?,
                ],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .execute(
                "DELETE FROM execution_links WHERE run_id = ?1 AND span_id = ?2",
                params![span.context.run_id.as_str(), span.context.span_id.as_str()],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        insert_links(
            &transaction,
            span.context.run_id.as_str(),
            Some(span.context.span_id.as_str()),
            &span.links,
        )?;
        transaction
            .commit()
            .map_err(|error| storage_error(error.to_string()))
    }
}

impl ExecutionTelemetryPort for SqliteExecutionTimelineRepository {
    fn start_run(&self, run: &ExecutionRun) -> Result<(), ExecutionTelemetryError> {
        self.insert_run(run)
    }

    fn finish_run(
        &self,
        run_id: &ExecutionRunId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        let connection = self.connection()?;
        update_terminal(
            &connection,
            "execution_runs",
            run_id.as_str(),
            None,
            status,
            ended_at,
            error_classification,
        )
    }

    fn start_span(&self, span: &ExecutionSpan) -> Result<(), ExecutionTelemetryError> {
        self.insert_span(span)
    }

    fn finish_span(
        &self,
        run_id: &ExecutionRunId,
        span_id: &SpanId,
        status: ExecutionStatus,
        ended_at: &str,
        error_classification: Option<&str>,
    ) -> Result<(), ExecutionTelemetryError> {
        let connection = self.connection()?;
        update_terminal(
            &connection,
            "execution_spans",
            run_id.as_str(),
            Some(span_id.as_str()),
            status,
            ended_at,
            error_classification,
        )
    }

    fn record_event(&self, event: &ExecutionEvent) -> Result<(), ExecutionTelemetryError> {
        event
            .validate()
            .map_err(|error| storage_error(error.to_string()))?;
        self.connection()?
            .execute(
                r#"INSERT OR IGNORE INTO execution_events
                    (run_id, span_id, sequence, name, timestamp, attributes_json)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                params![
                    event.run_id.as_str(),
                    event.span_id.as_str(),
                    i64::try_from(event.sequence)
                        .map_err(|_| storage_error("event sequence exceeds SQLite range"))?,
                    event.name,
                    event.timestamp,
                    attributes_json(&event.attributes)?,
                ],
            )
            .map(|_| ())
            .map_err(|error| storage_error(error.to_string()))
    }

    fn add_metric(
        &self,
        _name: &'static str,
        _value: u64,
        _dimensions: &[(&'static str, &'static str)],
    ) -> Result<(), ExecutionTelemetryError> {
        Ok(())
    }
}

fn insert_links(
    connection: &Connection,
    run_id: &str,
    span_id: Option<&str>,
    links: &[crate::contexts::execution_observability::domain::ExecutionLink],
) -> Result<(), ExecutionTelemetryError> {
    for link in links {
        connection
            .execute(
                r#"INSERT OR IGNORE INTO execution_links
                    (run_id, span_id, linked_run_id, linked_trace_id, linked_span_id, relationship)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                params![
                    run_id,
                    span_id,
                    link.run_id.as_str(),
                    link.trace_id.as_str(),
                    link.span_id.as_ref().map(SpanId::as_str),
                    link.relationship,
                ],
            )
            .map_err(|error| storage_error(error.to_string()))?;
    }
    Ok(())
}

fn update_terminal(
    connection: &Connection,
    table: &str,
    run_id: &str,
    span_id: Option<&str>,
    status: ExecutionStatus,
    ended_at: &str,
    error_classification: Option<&str>,
) -> Result<(), ExecutionTelemetryError> {
    if !status.is_terminal() || ended_at.trim().is_empty() {
        return Err(storage_error(
            "terminal update requires terminal status and timestamp",
        ));
    }
    let (sql, changed) = if let Some(span_id) = span_id {
        let sql = format!(
            "UPDATE {table} SET status = ?1, ended_at = ?2, error_classification = ?3 WHERE run_id = ?4 AND span_id = ?5 AND status IN ('accepted', 'running')"
        );
        let changed = connection.execute(
            &sql,
            params![
                status_value(status),
                ended_at,
                error_classification,
                run_id,
                span_id
            ],
        );
        (sql, changed)
    } else {
        let sql = format!(
            "UPDATE {table} SET status = ?1, ended_at = ?2, error_classification = ?3 WHERE run_id = ?4 AND status IN ('accepted', 'running')"
        );
        let changed = connection.execute(
            &sql,
            params![status_value(status), ended_at, error_classification, run_id],
        );
        (sql, changed)
    };
    let changed = changed.map_err(|error| storage_error(format!("{sql}: {error}")))?;
    if changed == 0 {
        let exists = if let Some(span_id) = span_id {
            connection
                .query_row(
                    &format!("SELECT 1 FROM {table} WHERE run_id = ?1 AND span_id = ?2"),
                    params![run_id, span_id],
                    |_| Ok(()),
                )
                .is_ok()
        } else {
            connection
                .query_row(
                    &format!("SELECT 1 FROM {table} WHERE run_id = ?1"),
                    [run_id],
                    |_| Ok(()),
                )
                .is_ok()
        };
        if !exists {
            return Err(storage_error("execution record not found"));
        }
    }
    Ok(())
}
