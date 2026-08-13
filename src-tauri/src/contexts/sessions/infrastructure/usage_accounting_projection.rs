#![allow(dead_code)]

use super::usage_accounting::storage;
use crate::contexts::sessions::application::{
    SessionsApplicationError, UsageAccountingSummary, UsageBreakdown, UsageBreakdownDimension,
    UsageBreakdownEntry, UsageDailyAggregate, UsageEntityCounts, UsageMeasureAggregate,
    UsageQualityAggregate, UsageSummaryQuery,
};
use crate::contexts::sessions::domain::{
    AccountingUnit, MeasurementQuality, TokenDimensions, TokenOverlap, UsagePurpose, UsageStatus,
};
use rusqlite::{params, Connection, Row};
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn project_usage_summary(
    connection: &Connection,
    query: &UsageSummaryQuery,
) -> Result<UsageAccountingSummary, SessionsApplicationError> {
    let purpose = query.purpose.map(storage).transpose()?;
    let quality = query.quality.map(storage).transpose()?;
    let status = query.status.map(storage).transpose()?;
    let mut statement = connection
        .prepare(PROJECTION_SQL)
        .map_err(repository_error)?;
    let rows = statement
        .query_map(
            params![
                query.session_id,
                query.message_id,
                query.generation_id,
                query.agent_id,
                query.provider_id,
                query.model_id,
                purpose,
                quality,
                status,
                query.range_start,
                query.range_end,
            ],
            projection_row,
        )
        .map_err(repository_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(repository_error)?;
    aggregate_rows(
        rows,
        query.breakdown_limit.clamp(1, 50),
        &query.generated_at,
    )
}

const PROJECTION_SQL: &str = r#"
    SELECT observation.quality, observation.unit,
           observation.input_count, observation.output_count,
           observation.cached_input_count, observation.cache_write_input_count,
           observation.reasoning_output_count, observation.provider_total_count,
           observation.cache_overlap, observation.reasoning_overlap,
           invocation.id, invocation.generation_id, invocation.session_id,
           invocation.agent_id, COALESCE(invocation.provider_id, 'unknown'),
           COALESCE(invocation.model_id, 'unknown'), invocation.purpose,
           invocation.status,
           COALESCE(date(COALESCE(observation.event_at, observation.observed_at), 'localtime'),
                    'unknown')
    FROM token_usage_observations observation
    INNER JOIN model_invocations invocation ON invocation.id = observation.invocation_id
    WHERE observation.superseded_by_observation_id IS NULL
      AND (?1 IS NULL OR invocation.session_id = ?1)
      AND (?2 IS NULL OR invocation.message_id = ?2)
      AND (?3 IS NULL OR invocation.generation_id = ?3)
      AND (?4 IS NULL OR invocation.agent_id = ?4)
      AND (?5 IS NULL OR COALESCE(invocation.provider_id, 'unknown') = ?5)
      AND (?6 IS NULL OR COALESCE(invocation.model_id, 'unknown') = ?6)
      AND (?7 IS NULL OR invocation.purpose = ?7)
      AND (?8 IS NULL OR observation.quality = ?8)
      AND (?9 IS NULL OR invocation.status = ?9)
      AND (?10 IS NULL OR COALESCE(observation.event_at, observation.observed_at) >= ?10)
      AND (?11 IS NULL OR COALESCE(observation.event_at, observation.observed_at) < ?11)
    ORDER BY COALESCE(observation.event_at, observation.observed_at), observation.id
"#;

#[derive(Debug)]
struct ProjectionRow {
    quality: MeasurementQuality,
    unit: AccountingUnit,
    dimensions: TokenDimensions,
    cache_overlap: TokenOverlap,
    reasoning_overlap: TokenOverlap,
    invocation_id: String,
    generation_id: Option<String>,
    session_id: String,
    agent_id: String,
    provider_id: String,
    model_id: String,
    purpose: UsagePurpose,
    status: UsageStatus,
    local_date: String,
}

fn projection_row(row: &Row<'_>) -> rusqlite::Result<ProjectionRow> {
    Ok(ProjectionRow {
        quality: parse(row.get(0)?)?,
        unit: parse(row.get(1)?)?,
        dimensions: TokenDimensions {
            input: row.get(2)?,
            output: row.get(3)?,
            cached_input: row.get(4)?,
            cache_write_input: row.get(5)?,
            reasoning_output: row.get(6)?,
            provider_total: row.get(7)?,
        },
        cache_overlap: parse(row.get(8)?)?,
        reasoning_overlap: parse(row.get(9)?)?,
        invocation_id: row.get(10)?,
        generation_id: row.get(11)?,
        session_id: row.get(12)?,
        agent_id: row.get(13)?,
        provider_id: row.get(14)?,
        model_id: row.get(15)?,
        purpose: parse(row.get(16)?)?,
        status: parse(row.get(17)?)?,
        local_date: row.get(18)?,
    })
}

#[derive(Default)]
struct SummaryAccumulator {
    reported: MeasureAccumulator,
    reported_derived: MeasureAccumulator,
    estimated: MeasureAccumulator,
    calls: BTreeSet<String>,
    generations: BTreeSet<String>,
    sessions: BTreeSet<String>,
}

#[derive(Default)]
struct MeasureAccumulator {
    dimensions: TokenDimensions,
    provider_total_complete: bool,
    headline_total: Option<i64>,
    calls: BTreeSet<String>,
    observations: i64,
}

impl SummaryAccumulator {
    fn add(&mut self, row: &ProjectionRow) -> Result<(), SessionsApplicationError> {
        self.calls.insert(row.invocation_id.clone());
        if let Some(generation_id) = &row.generation_id {
            self.generations.insert(generation_id.clone());
        }
        self.sessions.insert(row.session_id.clone());
        self.measure_mut(row.quality).add(row)
    }

    fn measure_mut(&mut self, quality: MeasurementQuality) -> &mut MeasureAccumulator {
        match quality {
            MeasurementQuality::Reported => &mut self.reported,
            MeasurementQuality::ReportedDerived => &mut self.reported_derived,
            MeasurementQuality::Estimated => &mut self.estimated,
        }
    }

    fn finish(
        self,
    ) -> Result<(UsageQualityAggregate, UsageEntityCounts), SessionsApplicationError> {
        Ok((
            UsageQualityAggregate {
                reported: self.reported.finish(AccountingUnit::Tokens)?,
                reported_derived: self.reported_derived.finish(AccountingUnit::Tokens)?,
                estimated: self.estimated.finish(AccountingUnit::Characters)?,
            },
            UsageEntityCounts {
                calls: count(self.calls.len())?,
                generations: count(self.generations.len())?,
                sessions: count(self.sessions.len())?,
            },
        ))
    }
}

impl MeasureAccumulator {
    fn add(&mut self, row: &ProjectionRow) -> Result<(), SessionsApplicationError> {
        if row.unit
            != match row.quality {
                MeasurementQuality::Estimated => AccountingUnit::Characters,
                MeasurementQuality::Reported | MeasurementQuality::ReportedDerived => {
                    AccountingUnit::Tokens
                }
            }
        {
            return Err(SessionsApplicationError::Validation(
                "persisted usage quality and unit are inconsistent".to_string(),
            ));
        }
        self.dimensions.input = add(self.dimensions.input, row.dimensions.input)?;
        self.dimensions.output = add(self.dimensions.output, row.dimensions.output)?;
        self.dimensions.cached_input =
            add(self.dimensions.cached_input, row.dimensions.cached_input)?;
        self.dimensions.cache_write_input = add(
            self.dimensions.cache_write_input,
            row.dimensions.cache_write_input,
        )?;
        self.dimensions.reasoning_output = add(
            self.dimensions.reasoning_output,
            row.dimensions.reasoning_output,
        )?;
        match (
            self.observations,
            self.dimensions.provider_total,
            row.dimensions.provider_total,
        ) {
            (0, _, Some(total)) => {
                self.dimensions.provider_total = Some(total);
                self.provider_total_complete = true;
            }
            (_, Some(current), Some(total)) if self.provider_total_complete => {
                self.dimensions.provider_total = Some(add(current, total)?);
            }
            _ => {
                self.dimensions.provider_total = None;
                self.provider_total_complete = false;
            }
        }
        let headline = row
            .dimensions
            .headline_total(row.cache_overlap, row.reasoning_overlap);
        self.headline_total = match (self.observations, self.headline_total, headline) {
            (0, _, value) => value,
            (_, Some(current), Some(value)) => Some(add(current, value)?),
            _ => None,
        };
        self.calls.insert(row.invocation_id.clone());
        self.observations = add(self.observations, 1)?;
        Ok(())
    }

    fn finish(
        self,
        unit: AccountingUnit,
    ) -> Result<UsageMeasureAggregate, SessionsApplicationError> {
        Ok(UsageMeasureAggregate {
            unit,
            dimensions: self.dimensions,
            headline_total: if self.observations == 0 {
                Some(0)
            } else {
                self.headline_total
            },
            call_count: count(self.calls.len())?,
            observation_count: self.observations,
        })
    }
}

fn aggregate_rows(
    rows: Vec<ProjectionRow>,
    breakdown_limit: usize,
    generated_at: &str,
) -> Result<UsageAccountingSummary, SessionsApplicationError> {
    let mut total = SummaryAccumulator::default();
    let mut user_response = SummaryAccumulator::default();
    let mut internal = SummaryAccumulator::default();
    let mut daily = BTreeMap::<String, SummaryAccumulator>::new();
    let mut breakdowns = breakdown_maps();
    for row in &rows {
        total.add(row)?;
        if is_internal(row.purpose) {
            internal.add(row)?;
        } else {
            user_response.add(row)?;
        }
        daily.entry(row.local_date.clone()).or_default().add(row)?;
        for (dimension, key) in breakdown_keys(row) {
            breakdowns
                .entry(dimension)
                .or_default()
                .entry(key)
                .or_default()
                .add(row)?;
        }
    }
    let (totals, counts) = total.finish()?;
    Ok(UsageAccountingSummary {
        totals,
        user_response: user_response.finish()?.0,
        internal: internal.finish()?.0,
        counts,
        daily: daily
            .into_iter()
            .map(|(local_date, accumulator)| {
                let (totals, counts) = accumulator.finish()?;
                Ok(UsageDailyAggregate {
                    local_date,
                    totals,
                    counts,
                })
            })
            .collect::<Result<Vec<_>, SessionsApplicationError>>()?,
        breakdowns: finish_breakdowns(breakdowns, breakdown_limit)?,
        generated_at: generated_at.to_string(),
    })
}

type BreakdownMaps = BTreeMap<UsageBreakdownDimension, BTreeMap<String, SummaryAccumulator>>;

fn breakdown_maps() -> BreakdownMaps {
    [
        UsageBreakdownDimension::Agent,
        UsageBreakdownDimension::Provider,
        UsageBreakdownDimension::Model,
        UsageBreakdownDimension::Purpose,
        UsageBreakdownDimension::Quality,
        UsageBreakdownDimension::Status,
    ]
    .into_iter()
    .map(|dimension| (dimension, BTreeMap::new()))
    .collect()
}

fn breakdown_keys(row: &ProjectionRow) -> [(UsageBreakdownDimension, String); 6] {
    [
        (UsageBreakdownDimension::Agent, row.agent_id.clone()),
        (UsageBreakdownDimension::Provider, row.provider_id.clone()),
        (UsageBreakdownDimension::Model, row.model_id.clone()),
        (
            UsageBreakdownDimension::Purpose,
            purpose_key(row.purpose).to_string(),
        ),
        (
            UsageBreakdownDimension::Quality,
            quality_key(row.quality).to_string(),
        ),
        (
            UsageBreakdownDimension::Status,
            status_key(row.status).to_string(),
        ),
    ]
}

fn finish_breakdowns(
    maps: BreakdownMaps,
    limit: usize,
) -> Result<Vec<UsageBreakdown>, SessionsApplicationError> {
    maps.into_iter()
        .map(|(dimension, entries)| {
            let mut entries = entries
                .into_iter()
                .map(|(key, accumulator)| {
                    let (totals, counts) = accumulator.finish()?;
                    Ok(UsageBreakdownEntry {
                        key,
                        totals,
                        counts,
                    })
                })
                .collect::<Result<Vec<_>, SessionsApplicationError>>()?;
            entries.sort_by(|left, right| {
                right
                    .counts
                    .calls
                    .cmp(&left.counts.calls)
                    .then_with(|| left.key.cmp(&right.key))
            });
            entries.truncate(limit);
            Ok(UsageBreakdown { dimension, entries })
        })
        .collect()
}

fn is_internal(purpose: UsagePurpose) -> bool {
    matches!(
        purpose,
        UsagePurpose::ContextCompaction | UsagePurpose::MemoryExtraction
    )
}

fn purpose_key(purpose: UsagePurpose) -> &'static str {
    match purpose {
        UsagePurpose::AssistantInitial => "assistant-initial",
        UsagePurpose::ToolContinuation => "tool-continuation",
        UsagePurpose::ContextCompaction => "context-compaction",
        UsagePurpose::MemoryExtraction => "memory-extraction",
        UsagePurpose::Retry => "retry",
        UsagePurpose::TerminalInterval => "terminal-interval",
    }
}

fn quality_key(quality: MeasurementQuality) -> &'static str {
    match quality {
        MeasurementQuality::Reported => "reported",
        MeasurementQuality::ReportedDerived => "reported-derived",
        MeasurementQuality::Estimated => "estimated",
    }
}

fn status_key(status: UsageStatus) -> &'static str {
    match status {
        UsageStatus::Running => "running",
        UsageStatus::Succeeded => "succeeded",
        UsageStatus::Failed => "failed",
        UsageStatus::Cancelled => "cancelled",
    }
}

fn add(left: i64, right: i64) -> Result<i64, SessionsApplicationError> {
    left.checked_add(right).ok_or_else(|| {
        SessionsApplicationError::Validation("usage aggregate exceeds i64 range".to_string())
    })
}

fn count(value: usize) -> Result<i64, SessionsApplicationError> {
    i64::try_from(value).map_err(|_| {
        SessionsApplicationError::Validation("usage entity count exceeds i64 range".to_string())
    })
}

fn parse<T: DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value)).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}
