use super::SqliteSessionsRepository;
use crate::contexts::sessions::application::{
    EstimatedCharacterTotals, ReportedTokenTotals, SessionUsageAgentBreakdown,
    SessionUsageCoverage, SessionUsagePoint, SessionUsageRepository, SessionUsageStatistics,
    SessionUsageSummary, SessionsApplicationError, UsageStatisticsRange,
};
use rusqlite::{params, params_from_iter, Connection, Row};

/// Migration 22 is intentionally retained as a no-op so pre-release migration numbering remains
/// dense. Fine-grained accounting starts at migration 62 and deliberately imports no old data.
pub(crate) fn apply_schema(
    _connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
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
            "{ACTIVE_USAGE_CTE}
             SELECT {AGGREGATE_COLUMNS},
                COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN 1 ELSE 0 END), 0),
                COUNT(DISTINCT session_id)
             FROM active_usage {filter}"
        );
        let (reported, estimated, responses, reported_responses, estimated_responses, sessions) =
            connection
                .query_row(&summary_sql, params_from_iter(range_start.iter()), |row| {
                    let (reported, estimated, responses) = totals_from_row(row, 0)?;
                    Ok((
                        reported,
                        estimated,
                        responses,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                })
                .map_err(repository_error)?;
        let daily_sql = format!(
            "{ACTIVE_USAGE_CTE}
             SELECT date(occurred_at, 'localtime'), {AGGREGATE_COLUMNS}
             FROM active_usage {filter}
             GROUP BY date(occurred_at, 'localtime')
             ORDER BY date(occurred_at, 'localtime')"
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
            "{ACTIVE_USAGE_CTE}
             SELECT agent_id, {AGGREGATE_COLUMNS}
             FROM active_usage {filter}
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
            coverage: coverage(reported_responses, estimated_responses, responses),
            counted_sessions: sessions,
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
        let sql = format!(
            "{ACTIVE_USAGE_CTE}
             SELECT {AGGREGATE_COLUMNS},
                COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN 1 ELSE 0 END), 0)
             FROM active_usage WHERE session_id = ?1"
        );
        let (reported, estimated, responses, reported_responses, estimated_responses) = connection
            .query_row(&sql, params![session_id], |row| {
                let (reported, estimated, responses) = totals_from_row(row, 0)?;
                Ok((reported, estimated, responses, row.get(7)?, row.get(8)?))
            })
            .map_err(repository_error)?;
        Ok(SessionUsageSummary {
            session_id: session_id.to_string(),
            reported,
            estimated,
            coverage: coverage(reported_responses, estimated_responses, responses),
            response_count: responses,
            generated_at: generated_at.to_string(),
        })
    }
}

const ACTIVE_USAGE_CTE: &str = r#"
    WITH active_usage AS (
        SELECT
            invocation.session_id,
            invocation.agent_id,
            CASE WHEN observation.quality = 'estimated' THEN 'estimated' ELSE 'reported' END
                AS accounting_kind,
            observation.input_count,
            observation.output_count,
            observation.cached_input_count AS cache_read_count,
            observation.cache_write_input_count AS cache_creation_count,
            COALESCE(observation.event_at, observation.observed_at) AS occurred_at
        FROM token_usage_observations observation
        INNER JOIN model_invocations invocation ON invocation.id = observation.invocation_id
        WHERE observation.superseded_by_observation_id IS NULL
    )
"#;

const AGGREGATE_COLUMNS: &str = r#"
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN input_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN output_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN cache_read_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN cache_creation_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN input_count ELSE 0 END), 0),
    COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN output_count ELSE 0 END), 0),
    COUNT(*)
"#;

fn coverage(reported: i64, estimated: i64, total: i64) -> SessionUsageCoverage {
    SessionUsageCoverage {
        reported_responses: reported,
        estimated_responses: estimated,
        total_responses: total,
        reported_percent: if total == 0 {
            0.0
        } else {
            reported as f64 / total as f64 * 100.0
        },
    }
}

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
