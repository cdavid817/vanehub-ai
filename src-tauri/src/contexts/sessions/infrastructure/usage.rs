use super::SqliteSessionsRepository;
use crate::contexts::sessions::application::{
    EstimatedCharacterTotals, MessageUsageRecord, ReportedTokenTotals, SessionUsageAccountingKind,
    SessionUsageAgentBreakdown, SessionUsageCoverage, SessionUsagePoint, SessionUsageRepository,
    SessionUsageStatistics, SessionUsageSummary, SessionUsageUnit, SessionsApplicationError,
    UsageStatisticsRange,
};
use rusqlite::{params, params_from_iter, Connection, Row, Transaction};

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS usage_records (
            message_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            provider_id TEXT,
            model_id TEXT,
            input_count INTEGER NOT NULL DEFAULT 0 CHECK (input_count >= 0),
            output_count INTEGER NOT NULL DEFAULT 0 CHECK (output_count >= 0),
            cache_read_count INTEGER NOT NULL DEFAULT 0 CHECK (cache_read_count >= 0),
            cache_creation_count INTEGER NOT NULL DEFAULT 0 CHECK (cache_creation_count >= 0),
            accounting_kind TEXT NOT NULL CHECK (accounting_kind IN ('reported', 'estimated')),
            unit TEXT NOT NULL CHECK (unit IN ('tokens', 'characters')),
            source TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (agent_id) REFERENCES agents(id),
            CHECK (
                (accounting_kind = 'reported' AND unit = 'tokens') OR
                (accounting_kind = 'estimated' AND unit = 'characters')
            )
        );

        CREATE INDEX IF NOT EXISTS idx_usage_records_occurred_at
            ON usage_records(occurred_at);
        CREATE INDEX IF NOT EXISTS idx_usage_records_agent_occurred
            ON usage_records(agent_id, occurred_at);

        INSERT OR IGNORE INTO usage_records (
            message_id, session_id, agent_id, input_count, output_count,
            cache_read_count, cache_creation_count, accounting_kind, unit,
            source, occurred_at
        )
        SELECT
            messages.id,
            messages.session_id,
            sessions.agent_id,
            MAX(COALESCE(messages.token_input, 0), 0),
            MAX(COALESCE(messages.token_output, 0), 0),
            0,
            0,
            'estimated',
            'characters',
            'legacy-character-count',
            messages.created_at
        FROM messages
        INNER JOIN sessions ON sessions.id = messages.session_id
        WHERE messages.role = 'assistant'
          AND (
              COALESCE(messages.token_input, 0) > 0 OR
              COALESCE(messages.token_output, 0) > 0
          );
        "#,
    )?;
    Ok(())
}

impl SessionUsageRepository for SqliteSessionsRepository {
    fn statistics(
        &self,
        range: UsageStatisticsRange,
        range_start: Option<&str>,
        generated_at: &str,
    ) -> Result<SessionUsageStatistics, SessionsApplicationError> {
        let connection = self.connection()?;
        let filter = if range_start.is_some() {
            "WHERE occurred_at >= ?1"
        } else {
            ""
        };
        let summary_sql = format!(
            "SELECT {AGGREGATE_COLUMNS},
                COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN 1 ELSE 0 END), 0),
                COUNT(DISTINCT session_id)
             FROM usage_records
             {filter}"
        );
        let (
            reported,
            estimated,
            total_responses,
            reported_responses,
            estimated_responses,
            counted_sessions,
        ) = connection
            .query_row(&summary_sql, params_from_iter(range_start.iter()), |row| {
                let (reported, estimated, total_responses) = totals_from_row(row, 0)?;
                Ok((
                    reported,
                    estimated,
                    total_responses,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            })
            .map_err(repository_error)?;
        let reported_percent = if total_responses == 0 {
            0.0
        } else {
            (reported_responses as f64 / total_responses as f64) * 100.0
        };

        let daily_sql = format!(
            "SELECT date(occurred_at, 'localtime') AS local_date, {AGGREGATE_COLUMNS}
             FROM usage_records
             {filter}
             GROUP BY local_date
             ORDER BY local_date"
        );
        let mut daily_statement = connection.prepare(&daily_sql).map_err(repository_error)?;
        let daily = daily_statement
            .query_map(params_from_iter(range_start.iter()), |row| {
                let (reported, estimated, response_count) = totals_from_row(row, 1)?;
                Ok(SessionUsagePoint {
                    date: row.get(0)?,
                    reported,
                    estimated,
                    response_count,
                })
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;

        let agent_sql = format!(
            "SELECT agent_id, {AGGREGATE_COLUMNS}
             FROM usage_records
             {filter}
             GROUP BY agent_id
             ORDER BY COUNT(*) DESC, agent_id"
        );
        let mut agent_statement = connection.prepare(&agent_sql).map_err(repository_error)?;
        let by_agent = agent_statement
            .query_map(params_from_iter(range_start.iter()), |row| {
                let (reported, estimated, response_count) = totals_from_row(row, 1)?;
                Ok(SessionUsageAgentBreakdown {
                    agent_id: row.get(0)?,
                    reported,
                    estimated,
                    response_count,
                })
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;

        Ok(SessionUsageStatistics {
            range,
            reported,
            estimated,
            coverage: SessionUsageCoverage {
                reported_responses,
                estimated_responses,
                total_responses,
                reported_percent,
            },
            counted_sessions,
            daily,
            by_agent,
            generated_at: generated_at.to_string(),
        })
    }

    fn summary_for_session(
        &self,
        session_id: &str,
        generated_at: &str,
    ) -> Result<SessionUsageSummary, SessionsApplicationError> {
        let connection = self.connection()?;
        let (reported, estimated, response_count, reported_responses, estimated_responses) =
            connection
                .query_row(
                    &format!(
                        "SELECT {AGGREGATE_COLUMNS},
                            COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN 1 ELSE 0 END), 0),
                            COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN 1 ELSE 0 END), 0)
                         FROM usage_records
                         WHERE session_id = ?1"
                    ),
                    params![session_id],
                    |row| {
                        let (reported, estimated, response_count) = totals_from_row(row, 0)?;
                        Ok((
                            reported,
                            estimated,
                            response_count,
                            row.get(7)?,
                            row.get(8)?,
                        ))
                    },
                )
                .map_err(repository_error)?;
        let reported_percent = if response_count == 0 {
            0.0
        } else {
            (reported_responses as f64 / response_count as f64) * 100.0
        };
        Ok(SessionUsageSummary {
            session_id: session_id.to_string(),
            reported,
            estimated,
            coverage: SessionUsageCoverage {
                reported_responses,
                estimated_responses,
                total_responses: response_count,
                reported_percent,
            },
            response_count,
            generated_at: generated_at.to_string(),
        })
    }
}

pub(super) fn upsert_usage(
    transaction: &Transaction<'_>,
    usage: &MessageUsageRecord,
) -> Result<(), SessionsApplicationError> {
    let accounting_kind = match usage.accounting_kind {
        SessionUsageAccountingKind::Reported => "reported",
        SessionUsageAccountingKind::Estimated => "estimated",
    };
    let unit = match usage.unit {
        SessionUsageUnit::Tokens => "tokens",
        SessionUsageUnit::Characters => "characters",
    };
    let update_guard = if usage.accounting_kind == SessionUsageAccountingKind::Estimated {
        "WHERE usage_records.accounting_kind != 'reported'"
    } else {
        ""
    };
    transaction
        .execute(
            &format!(
                r#"
                INSERT INTO usage_records (
                    message_id, session_id, agent_id, provider_id, model_id,
                    input_count, output_count, cache_read_count, cache_creation_count,
                    accounting_kind, unit, source, occurred_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(message_id) DO UPDATE SET
                    session_id = excluded.session_id,
                    agent_id = excluded.agent_id,
                    provider_id = excluded.provider_id,
                    model_id = excluded.model_id,
                    input_count = excluded.input_count,
                    output_count = excluded.output_count,
                    cache_read_count = excluded.cache_read_count,
                    cache_creation_count = excluded.cache_creation_count,
                    accounting_kind = excluded.accounting_kind,
                    unit = excluded.unit,
                    source = excluded.source,
                    occurred_at = excluded.occurred_at
                {update_guard}
                "#
            ),
            params![
                usage.message_id,
                usage.session_id,
                usage.agent_id,
                usage.provider_id,
                usage.model_id,
                usage.input_count,
                usage.output_count,
                usage.cache_read_count,
                usage.cache_creation_count,
                accounting_kind,
                unit,
                usage.source,
                usage.occurred_at,
            ],
        )
        .map_err(repository_error)?;
    Ok(())
}

const AGGREGATE_COLUMNS: &str = r#"
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN input_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN output_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN cache_read_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN cache_creation_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN input_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN output_count ELSE 0 END), 0),
    COUNT(*)
"#;

fn totals_from_row(
    row: &Row<'_>,
    start: usize,
) -> rusqlite::Result<(ReportedTokenTotals, EstimatedCharacterTotals, i64)> {
    let input_tokens = row.get(start)?;
    let output_tokens = row.get(start + 1)?;
    let cache_read_tokens = row.get(start + 2)?;
    let cache_creation_tokens = row.get(start + 3)?;
    let input_characters = row.get(start + 4)?;
    let output_characters = row.get(start + 5)?;
    Ok((
        ReportedTokenTotals {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            total_tokens: input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens,
        },
        EstimatedCharacterTotals {
            input_characters,
            output_characters,
            total_characters: input_characters + output_characters,
        },
        row.get(start + 6)?,
    ))
}

fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}
