use chrono::{DateTime, Days, Local, TimeZone, Utc};
use rusqlite::{params, params_from_iter, Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::{current_timestamp, AppError};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UsageStatisticsRange {
    Today,
    Last7Days,
    Last30Days,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum AccountingKind {
    Reported,
    Estimated,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum UsageUnit {
    Tokens,
    Characters,
}

impl UsageUnit {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::Characters => "characters",
        }
    }
}

impl AccountingKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reported => "reported",
            Self::Estimated => "estimated",
        }
    }
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportedTokenTotals {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cache_read_tokens: i64,
    pub(crate) cache_creation_tokens: i64,
    pub(crate) total_tokens: i64,
}

impl ReportedTokenTotals {
    fn from_parts(input: i64, output: i64, cache_read: i64, cache_creation: i64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_read,
            cache_creation_tokens: cache_creation,
            total_tokens: input + output + cache_read + cache_creation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EstimatedCharacterTotals {
    pub(crate) input_characters: i64,
    pub(crate) output_characters: i64,
    pub(crate) total_characters: i64,
}

impl EstimatedCharacterTotals {
    fn from_parts(input: i64, output: i64) -> Self {
        Self {
            input_characters: input,
            output_characters: output,
            total_characters: input + output,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageCoverage {
    pub(crate) reported_responses: i64,
    pub(crate) estimated_responses: i64,
    pub(crate) total_responses: i64,
    pub(crate) reported_percent: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageStatisticsPoint {
    pub(crate) date: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) response_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageAgentBreakdown {
    pub(crate) agent_id: String,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) response_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageStatistics {
    pub(crate) range: UsageStatisticsRange,
    pub(crate) reported: ReportedTokenTotals,
    pub(crate) estimated: EstimatedCharacterTotals,
    pub(crate) coverage: UsageCoverage,
    pub(crate) counted_sessions: i64,
    pub(crate) daily: Vec<UsageStatisticsPoint>,
    pub(crate) by_agent: Vec<UsageAgentBreakdown>,
    pub(crate) generated_at: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ReportedUsage {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cache_read_tokens: i64,
    pub(crate) cache_creation_tokens: i64,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) source: String,
}

impl ReportedUsage {
    pub(crate) fn total_tokens(&self) -> i64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_creation_tokens
    }

    pub(crate) fn has_usage(&self) -> bool {
        self.total_tokens() > 0
    }

    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if [
            self.input_tokens,
            self.output_tokens,
            self.cache_read_tokens,
            self.cache_creation_tokens,
        ]
        .iter()
        .any(|value| *value < 0)
        {
            return Err(AppError::Validation(
                "usage token counts must be non-negative".to_string(),
            ));
        }
        Ok(())
    }

    fn merge_max(&mut self, next: &Self) {
        self.input_tokens = self.input_tokens.max(next.input_tokens);
        self.output_tokens = self.output_tokens.max(next.output_tokens);
        self.cache_read_tokens = self.cache_read_tokens.max(next.cache_read_tokens);
        self.cache_creation_tokens = self.cache_creation_tokens.max(next.cache_creation_tokens);
        if next.provider_id.is_some() {
            self.provider_id.clone_from(&next.provider_id);
        }
        if next.model_id.is_some() {
            self.model_id.clone_from(&next.model_id);
        }
        if !next.source.is_empty() {
            self.source.clone_from(&next.source);
        }
    }

    fn saturating_delta(&self, baseline: &Self) -> Self {
        Self {
            input_tokens: self.input_tokens.saturating_sub(baseline.input_tokens),
            output_tokens: self.output_tokens.saturating_sub(baseline.output_tokens),
            cache_read_tokens: self
                .cache_read_tokens
                .saturating_sub(baseline.cache_read_tokens),
            cache_creation_tokens: self
                .cache_creation_tokens
                .saturating_sub(baseline.cache_creation_tokens),
            provider_id: self.provider_id.clone(),
            model_id: self.model_id.clone(),
            source: self.source.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum UsageEventMode {
    Snapshot,
    Cumulative,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UsageEvent {
    pub(crate) observation_id: Option<String>,
    pub(crate) usage: ReportedUsage,
    pub(crate) mode: UsageEventMode,
}

#[derive(Debug, Default)]
pub(crate) struct UsageAccumulator {
    keyed: HashMap<String, ReportedUsage>,
    anonymous: Option<ReportedUsage>,
    cumulative_baseline: Option<ReportedUsage>,
}

impl UsageAccumulator {
    pub(crate) fn observe(&mut self, event: UsageEvent) {
        let usage = match event.mode {
            UsageEventMode::Snapshot => event.usage,
            UsageEventMode::Cumulative => {
                let Some(baseline) = self.cumulative_baseline.as_ref() else {
                    self.cumulative_baseline = Some(event.usage);
                    return;
                };
                event.usage.saturating_delta(baseline)
            }
        };
        if !usage.has_usage() {
            return;
        }
        if let Some(id) = event.observation_id {
            self.keyed
                .entry(id)
                .and_modify(|current| current.merge_max(&usage))
                .or_insert(usage);
        } else if let Some(current) = self.anonymous.as_mut() {
            current.merge_max(&usage);
        } else {
            self.anonymous = Some(usage);
        }
    }

    pub(crate) fn finish(self) -> Option<ReportedUsage> {
        let mut items = self.keyed.into_values().collect::<Vec<_>>();
        if let Some(anonymous) = self.anonymous {
            items.push(anonymous);
        }
        if items.is_empty() {
            return None;
        }
        let mut total = ReportedUsage::default();
        for item in items {
            total.input_tokens += item.input_tokens;
            total.output_tokens += item.output_tokens;
            total.cache_read_tokens += item.cache_read_tokens;
            total.cache_creation_tokens += item.cache_creation_tokens;
            if item.provider_id.is_some() {
                total.provider_id = item.provider_id;
            }
            if item.model_id.is_some() {
                total.model_id = item.model_id;
            }
            if !item.source.is_empty() {
                total.source = item.source;
            }
        }
        total.has_usage().then_some(total)
    }
}

fn count_at(value: &Value, pointers: &[&str]) -> i64 {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer))
        .and_then(|item| {
            item.as_i64().or_else(|| {
                item.as_u64()
                    .map(|number| number.min(i64::MAX as u64) as i64)
            })
        })
        .unwrap_or(0)
        .max(0)
}

fn string_at(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .map(str::to_string)
        .filter(|item| !item.trim().is_empty())
}

fn usage_from_counts(
    source: &str,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_creation: i64,
    provider_id: Option<String>,
    model_id: Option<String>,
    input_includes_cache: bool,
) -> ReportedUsage {
    ReportedUsage {
        input_tokens: if input_includes_cache {
            input
                .saturating_sub(cache_read)
                .saturating_sub(cache_creation)
        } else {
            input
        },
        output_tokens: output,
        cache_read_tokens: cache_read,
        cache_creation_tokens: cache_creation,
        provider_id,
        model_id,
        source: source.to_string(),
    }
}

pub(crate) fn looks_like_usage(value: &Value) -> bool {
    value.get("usage").is_some()
        || value.get("usageMetadata").is_some()
        || value.get("tokens").is_some()
        || value.pointer("/message/usage").is_some()
        || value.pointer("/response/usage").is_some()
        || value.pointer("/payload/info/last_token_usage").is_some()
        || value.pointer("/payload/info/total_token_usage").is_some()
}

pub(crate) fn parse_usage_event(agent_id: &str, value: &Value) -> Option<UsageEvent> {
    let observation_id = string_at(
        value,
        &[
            "/message/id",
            "/response/id",
            "/part/id",
            "/id",
            "/messageID",
        ],
    );
    let provider_id = string_at(
        value,
        &[
            "/providerID",
            "/provider_id",
            "/provider/id",
            "/metadata/provider_id",
        ],
    );
    let model_id = string_at(
        value,
        &[
            "/message/model",
            "/response/model",
            "/modelVersion",
            "/modelID",
            "/model",
            "/payload/model",
            "/payload/info/model",
            "/payload/info/model_name",
        ],
    );

    if agent_id == "codex-cli" {
        if let Some(info) = value
            .pointer("/payload/info")
            .or_else(|| value.pointer("/info"))
        {
            if let Some(last) = info.get("last_token_usage") {
                let cache_read =
                    count_at(last, &["/cached_input_tokens", "/cache_read_input_tokens"]);
                let usage = usage_from_counts(
                    agent_id,
                    count_at(last, &["/input_tokens"]),
                    count_at(last, &["/output_tokens"]),
                    cache_read,
                    0,
                    provider_id.clone(),
                    model_id.clone(),
                    true,
                );
                return usage.has_usage().then_some(UsageEvent {
                    observation_id: observation_id.clone(),
                    usage,
                    mode: UsageEventMode::Snapshot,
                });
            }
            if let Some(total) = info.get("total_token_usage") {
                let cache_read =
                    count_at(total, &["/cached_input_tokens", "/cache_read_input_tokens"]);
                let usage = usage_from_counts(
                    agent_id,
                    count_at(total, &["/input_tokens"]),
                    count_at(total, &["/output_tokens"]),
                    cache_read,
                    0,
                    provider_id.clone(),
                    model_id.clone(),
                    true,
                );
                return usage.has_usage().then_some(UsageEvent {
                    observation_id: observation_id.clone(),
                    usage,
                    mode: UsageEventMode::Cumulative,
                });
            }
        }
    }

    if agent_id == "gemini-cli" {
        if let Some(metadata) = value
            .get("usageMetadata")
            .or_else(|| value.pointer("/response/usageMetadata"))
        {
            let input = count_at(metadata, &["/promptTokenCount"]);
            let cache_read = count_at(metadata, &["/cachedContentTokenCount"]);
            let candidates = count_at(metadata, &["/candidatesTokenCount"]);
            let thoughts = count_at(metadata, &["/thoughtsTokenCount"]);
            let total = count_at(metadata, &["/totalTokenCount"]);
            let output = if candidates + thoughts > 0 {
                candidates + thoughts
            } else {
                total.saturating_sub(input)
            };
            let usage = usage_from_counts(
                agent_id,
                input,
                output,
                cache_read,
                0,
                provider_id.clone(),
                model_id.clone(),
                true,
            );
            return usage.has_usage().then_some(UsageEvent {
                observation_id: observation_id.clone(),
                usage,
                mode: UsageEventMode::Snapshot,
            });
        }
        if let Some(tokens) = value
            .get("tokens")
            .or_else(|| value.pointer("/data/tokens"))
        {
            let usage = usage_from_counts(
                agent_id,
                count_at(tokens, &["/input"]),
                count_at(tokens, &["/output"]) + count_at(tokens, &["/thoughts"]),
                count_at(tokens, &["/cached"]),
                0,
                provider_id.clone(),
                model_id.clone(),
                false,
            );
            return usage.has_usage().then_some(UsageEvent {
                observation_id: observation_id.clone(),
                usage,
                mode: UsageEventMode::Snapshot,
            });
        }
    }

    if agent_id == "opencode" {
        if let Some(tokens) = value
            .get("tokens")
            .or_else(|| value.pointer("/data/tokens"))
            .or_else(|| value.pointer("/part/tokens"))
        {
            let usage = usage_from_counts(
                agent_id,
                count_at(tokens, &["/input"]),
                count_at(tokens, &["/output"]) + count_at(tokens, &["/reasoning"]),
                count_at(tokens, &["/cache/read"]),
                count_at(tokens, &["/cache/write"]),
                provider_id.clone(),
                model_id.clone(),
                false,
            );
            return usage.has_usage().then_some(UsageEvent {
                observation_id: observation_id.clone(),
                usage,
                mode: UsageEventMode::Snapshot,
            });
        }
    }

    let usage_value = value
        .pointer("/message/usage")
        .or_else(|| value.pointer("/response/usage"))
        .or_else(|| value.get("usage"))?;
    let cache_read = count_at(
        usage_value,
        &[
            "/cache_read_input_tokens",
            "/cached_input_tokens",
            "/input_tokens_details/cached_tokens",
            "/prompt_tokens_details/cached_tokens",
        ],
    );
    let cache_creation = count_at(
        usage_value,
        &[
            "/cache_creation_input_tokens",
            "/input_tokens_details/cache_write_tokens",
        ],
    );
    let input = count_at(usage_value, &["/input_tokens", "/prompt_tokens"]);
    let output = count_at(usage_value, &["/output_tokens", "/completion_tokens"]);
    let usage = usage_from_counts(
        agent_id,
        input,
        output,
        cache_read,
        cache_creation,
        provider_id,
        model_id,
        agent_id == "codex-cli",
    );
    usage.has_usage().then_some(UsageEvent {
        observation_id,
        usage,
        mode: UsageEventMode::Snapshot,
    })
}

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
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

pub(crate) fn upsert_reported_usage(
    conn: &Connection,
    message_id: &str,
    session_id: &str,
    agent_id: &str,
    usage: &ReportedUsage,
    occurred_at: &str,
) -> Result<(), AppError> {
    usage.validate()?;
    if !usage.has_usage() {
        return Ok(());
    }
    let accounting_kind = AccountingKind::Reported.as_str();
    let unit = UsageUnit::Tokens.as_str();
    conn.execute(
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
        "#,
        params![
            message_id,
            session_id,
            agent_id,
            usage.provider_id,
            usage.model_id,
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_tokens,
            usage.cache_creation_tokens,
            accounting_kind,
            unit,
            usage.source,
            occurred_at,
        ],
    )?;
    Ok(())
}

pub(crate) fn upsert_estimated_usage(
    conn: &Connection,
    message_id: &str,
    session_id: &str,
    agent_id: &str,
    input_characters: i64,
    output_characters: i64,
    source: &str,
    occurred_at: &str,
) -> Result<(), AppError> {
    if input_characters < 0 || output_characters < 0 {
        return Err(AppError::Validation(
            "estimated character counts must be non-negative".to_string(),
        ));
    }
    if input_characters == 0 && output_characters == 0 {
        return Ok(());
    }
    let accounting_kind = AccountingKind::Estimated.as_str();
    let unit = UsageUnit::Characters.as_str();
    conn.execute(
        r#"
        INSERT INTO usage_records (
            message_id, session_id, agent_id, input_count, output_count,
            cache_read_count, cache_creation_count, accounting_kind, unit,
            source, occurred_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?7, ?8, ?9)
        ON CONFLICT(message_id) DO UPDATE SET
            session_id = excluded.session_id,
            agent_id = excluded.agent_id,
            input_count = excluded.input_count,
            output_count = excluded.output_count,
            source = excluded.source,
            occurred_at = excluded.occurred_at
        WHERE usage_records.accounting_kind != 'reported'
        "#,
        params![
            message_id,
            session_id,
            agent_id,
            input_characters,
            output_characters,
            accounting_kind,
            unit,
            source,
            occurred_at,
        ],
    )?;
    Ok(())
}

fn local_midnight(date_time: DateTime<Local>) -> Result<DateTime<Utc>, AppError> {
    let naive = date_time
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::Validation("invalid local date boundary".to_string()))?;
    let local = Local
        .from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| AppError::Validation("unresolvable local date boundary".to_string()))?;
    Ok(local.with_timezone(&Utc))
}

fn range_start_at(
    range: UsageStatisticsRange,
    now: DateTime<Local>,
) -> Result<Option<String>, AppError> {
    let days_back = match range {
        UsageStatisticsRange::All => return Ok(None),
        UsageStatisticsRange::Today => 0,
        UsageStatisticsRange::Last7Days => 6,
        UsageStatisticsRange::Last30Days => 29,
    };
    let date = now
        .checked_sub_days(Days::new(days_back))
        .ok_or_else(|| AppError::Validation("usage range is out of bounds".to_string()))?;
    Ok(Some(local_midnight(date)?.to_rfc3339()))
}

fn totals_from_row(
    row: &Row<'_>,
    start: usize,
) -> rusqlite::Result<(ReportedTokenTotals, EstimatedCharacterTotals, i64)> {
    let reported = ReportedTokenTotals::from_parts(
        row.get(start)?,
        row.get(start + 1)?,
        row.get(start + 2)?,
        row.get(start + 3)?,
    );
    let estimated = EstimatedCharacterTotals::from_parts(row.get(start + 4)?, row.get(start + 5)?);
    Ok((reported, estimated, row.get(start + 6)?))
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

fn usage_filter_clause(start: Option<&str>) -> &'static str {
    if start.is_some() {
        "WHERE occurred_at >= ?1"
    } else {
        ""
    }
}

pub(crate) fn aggregate_usage_statistics(
    conn: &Connection,
    range: UsageStatisticsRange,
) -> Result<UsageStatistics, AppError> {
    aggregate_usage_statistics_at(conn, range, Local::now())
}

fn aggregate_usage_statistics_at(
    conn: &Connection,
    range: UsageStatisticsRange,
    now: DateTime<Local>,
) -> Result<UsageStatistics, AppError> {
    let start = range_start_at(range, now)?;
    let filter = usage_filter_clause(start.as_deref());
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
    ) = conn.query_row(&summary_sql, params_from_iter(start.iter()), |row| {
        let (reported, estimated, total_responses) = totals_from_row(row, 0)?;
        Ok((
            reported,
            estimated,
            total_responses,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
        ))
    })?;

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
    let mut daily_stmt = conn.prepare(&daily_sql)?;
    let daily = daily_stmt
        .query_map(params_from_iter(start.iter()), |row| {
            let (reported, estimated, response_count) = totals_from_row(row, 1)?;
            Ok(UsageStatisticsPoint {
                date: row.get(0)?,
                reported,
                estimated,
                response_count,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let agent_sql = format!(
        "SELECT agent_id, {AGGREGATE_COLUMNS}
         FROM usage_records
         {filter}
         GROUP BY agent_id
         ORDER BY COUNT(*) DESC, agent_id"
    );
    let mut agent_stmt = conn.prepare(&agent_sql)?;
    let by_agent = agent_stmt
        .query_map(params_from_iter(start.iter()), |row| {
            let (reported, estimated, response_count) = totals_from_row(row, 1)?;
            Ok(UsageAgentBreakdown {
                agent_id: row.get(0)?,
                reported,
                estimated,
                response_count,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(UsageStatistics {
        range,
        reported,
        estimated,
        coverage: UsageCoverage {
            reported_responses,
            estimated_responses,
            total_responses,
            reported_percent,
        },
        counted_sessions,
        daily,
        by_agent,
        generated_at: current_timestamp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ParityFixture {
        now: String,
        range: UsageStatisticsRange,
        records: Vec<ParityRecord>,
        expected: Value,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ParityRecord {
        message_id: String,
        session_id: String,
        agent_id: String,
        accounting_kind: String,
        input_count: i64,
        output_count: i64,
        cache_read_count: i64,
        cache_creation_count: i64,
        occurred_at: String,
    }

    fn schema_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys");
        conn.execute_batch(
            r#"
            CREATE TABLE agents (id TEXT PRIMARY KEY);
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                FOREIGN KEY (agent_id) REFERENCES agents(id)
            );
            CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                token_input INTEGER,
                token_output INTEGER,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );
            INSERT INTO agents (id) VALUES ('codex-cli'), ('claude-code');
            INSERT INTO sessions (id, agent_id) VALUES
                ('session-1', 'codex-cli'),
                ('session-2', 'claude-code');
            "#,
        )
        .expect("base schema");
        conn
    }

    #[test]
    fn schema_backfills_positive_legacy_usage_idempotently() {
        let conn = schema_conn();
        conn.execute_batch(
            r#"
            INSERT INTO messages VALUES
                ('message-1', 'session-1', 'assistant', 12, 7, '2026-07-01T01:00:00Z'),
                ('message-2', 'session-1', 'assistant', 0, 0, '2026-07-01T02:00:00Z'),
                ('message-3', 'session-2', 'user', 4, 5, '2026-07-01T03:00:00Z');
            "#,
        )
        .expect("legacy messages");

        apply_schema(&conn).expect("first migration");
        apply_schema(&conn).expect("repeated migration");

        let record: (i64, i64, String, String, String) = conn
            .query_row(
                "SELECT input_count, output_count, accounting_kind, unit, agent_id FROM usage_records",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .expect("backfilled record");
        assert_eq!(
            record,
            (
                12,
                7,
                "estimated".into(),
                "characters".into(),
                "codex-cli".into()
            )
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_records", [], |row| row.get(0))
            .expect("record count");
        assert_eq!(count, 1);
    }

    #[test]
    fn reported_usage_replaces_estimate_and_estimate_cannot_downgrade_it() {
        let conn = schema_conn();
        conn.execute(
            "INSERT INTO messages VALUES (?1, ?2, 'assistant', 0, 0, ?3)",
            params!["message-1", "session-1", "2026-07-01T01:00:00Z"],
        )
        .expect("message");
        apply_schema(&conn).expect("usage schema");
        upsert_estimated_usage(
            &conn,
            "message-1",
            "session-1",
            "codex-cli",
            100,
            50,
            "character-count",
            "2026-07-01T01:00:00Z",
        )
        .expect("estimate");
        let reported = ReportedUsage {
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 3,
            cache_creation_tokens: 0,
            source: "codex-cli".into(),
            ..ReportedUsage::default()
        };
        upsert_reported_usage(
            &conn,
            "message-1",
            "session-1",
            "codex-cli",
            &reported,
            "2026-07-01T01:00:00Z",
        )
        .expect("reported");
        upsert_estimated_usage(
            &conn,
            "message-1",
            "session-1",
            "codex-cli",
            999,
            999,
            "character-count",
            "2026-07-01T01:00:00Z",
        )
        .expect("ignored estimate");

        let record: (String, String, i64, i64) = conn
            .query_row(
                "SELECT accounting_kind, unit, input_count, cache_read_count FROM usage_records",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("usage record");
        assert_eq!(record, ("reported".into(), "tokens".into(), 10, 3));
    }

    #[test]
    fn negative_usage_is_rejected() {
        let usage = ReportedUsage {
            input_tokens: -1,
            source: "fixture".into(),
            ..ReportedUsage::default()
        };
        assert!(usage.validate().is_err());
    }

    #[test]
    fn claude_usage_keeps_cache_categories_and_deduplicates_message_updates() {
        let first = serde_json::json!({
            "type": "assistant",
            "message": {
                "id": "msg-1",
                "model": "claude-sonnet-4-5",
                "usage": {
                    "input_tokens": 3,
                    "output_tokens": 20,
                    "cache_read_input_tokens": 50,
                    "cache_creation_input_tokens": 7
                }
            }
        });
        let final_update = serde_json::json!({
            "type": "assistant",
            "message": {
                "id": "msg-1",
                "model": "claude-sonnet-4-5",
                "usage": {
                    "input_tokens": 3,
                    "output_tokens": 30,
                    "cache_read_input_tokens": 50,
                    "cache_creation_input_tokens": 7
                }
            }
        });
        let mut accumulator = UsageAccumulator::default();
        accumulator.observe(parse_usage_event("claude-code", &first).expect("first usage"));
        accumulator.observe(parse_usage_event("claude-code", &final_update).expect("final usage"));
        let usage = accumulator.finish().expect("accumulated usage");
        assert_eq!(usage.input_tokens, 3);
        assert_eq!(usage.output_tokens, 30);
        assert_eq!(usage.cache_read_tokens, 50);
        assert_eq!(usage.cache_creation_tokens, 7);
        assert_eq!(usage.model_id.as_deref(), Some("claude-sonnet-4-5"));
    }

    #[test]
    fn codex_last_usage_separates_cached_input() {
        let value = serde_json::json!({
            "type": "token_count",
            "payload": {
                "info": {
                    "model": "gpt-5.4",
                    "last_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 40,
                        "output_tokens": 25
                    }
                }
            }
        });
        let event = parse_usage_event("codex-cli", &value).expect("codex usage");
        assert_eq!(event.mode, UsageEventMode::Snapshot);
        assert_eq!(event.usage.input_tokens, 60);
        assert_eq!(event.usage.cache_read_tokens, 40);
        assert_eq!(event.usage.output_tokens, 25);
        assert_eq!(event.usage.model_id.as_deref(), Some("gpt-5.4"));
    }

    #[test]
    fn codex_cumulative_usage_keeps_initial_baseline_across_updates() {
        let baseline = serde_json::json!({
            "payload": { "info": { "total_token_usage": {
                "input_tokens": 100,
                "cached_input_tokens": 40,
                "output_tokens": 20
            }}}
        });
        let next = serde_json::json!({
            "payload": { "info": { "total_token_usage": {
                "input_tokens": 150,
                "cached_input_tokens": 70,
                "output_tokens": 35
            }}}
        });
        let final_update = serde_json::json!({
            "payload": { "info": { "total_token_usage": {
                "input_tokens": 190,
                "cached_input_tokens": 90,
                "output_tokens": 50
            }}}
        });
        let regressed = serde_json::json!({
            "payload": { "info": { "total_token_usage": {
                "input_tokens": 10,
                "cached_input_tokens": 5,
                "output_tokens": 1
            }}}
        });
        let mut accumulator = UsageAccumulator::default();
        accumulator.observe(parse_usage_event("codex-cli", &baseline).expect("baseline"));
        accumulator.observe(parse_usage_event("codex-cli", &next).expect("next"));
        accumulator.observe(parse_usage_event("codex-cli", &final_update).expect("final update"));
        accumulator
            .observe(parse_usage_event("codex-cli", &final_update).expect("duplicate terminal"));
        accumulator.observe(parse_usage_event("codex-cli", &regressed).expect("regressed"));
        let usage = accumulator.finish().expect("delta usage");
        assert_eq!(usage.input_tokens, 40);
        assert_eq!(usage.cache_read_tokens, 50);
        assert_eq!(usage.output_tokens, 30);
    }

    #[test]
    fn gemini_usage_includes_thoughts_and_separates_cached_prompt() {
        let value = serde_json::json!({
            "modelVersion": "gemini-3-pro",
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 20,
                "thoughtsTokenCount": 5,
                "cachedContentTokenCount": 30,
                "totalTokenCount": 125
            }
        });
        let usage = parse_usage_event("gemini-cli", &value)
            .expect("gemini usage")
            .usage;
        assert_eq!(usage.input_tokens, 70);
        assert_eq!(usage.output_tokens, 25);
        assert_eq!(usage.cache_read_tokens, 30);
        assert_eq!(usage.model_id.as_deref(), Some("gemini-3-pro"));
    }

    #[test]
    fn opencode_usage_includes_reasoning_and_cache_write() {
        let value = serde_json::json!({
            "id": "part-1",
            "providerID": "openrouter",
            "modelID": "deepseek-v4",
            "tokens": {
                "input": 12,
                "output": 8,
                "reasoning": 3,
                "cache": { "read": 40, "write": 5 }
            }
        });
        let usage = parse_usage_event("opencode", &value)
            .expect("opencode usage")
            .usage;
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 11);
        assert_eq!(usage.cache_read_tokens, 40);
        assert_eq!(usage.cache_creation_tokens, 5);
        assert_eq!(usage.provider_id.as_deref(), Some("openrouter"));
        assert_eq!(usage.model_id.as_deref(), Some("deepseek-v4"));
    }

    #[test]
    fn zero_and_malformed_usage_are_not_reported() {
        let zero = serde_json::json!({"usage": {"input_tokens": 0, "output_tokens": 0}});
        let malformed = serde_json::json!({"usage": {"input_tokens": "secret-value"}});
        assert!(parse_usage_event("claude-code", &zero).is_none());
        assert!(parse_usage_event("claude-code", &malformed).is_none());
        assert!(looks_like_usage(&malformed));

        let agent_fixtures = [
            (
                "codex-cli",
                serde_json::json!({"payload":{"info":{"last_token_usage":{"input_tokens":"bad"}}}}),
            ),
            (
                "gemini-cli",
                serde_json::json!({"usageMetadata":{"promptTokenCount":"bad"}}),
            ),
            (
                "opencode",
                serde_json::json!({"tokens":{"input":"bad","cache":{"read":"secret"}}}),
            ),
        ];
        for (agent_id, fixture) in agent_fixtures {
            assert!(parse_usage_event(agent_id, &fixture).is_none());
            assert!(looks_like_usage(&fixture));
        }
    }

    #[test]
    fn aggregate_keeps_reported_tokens_and_estimated_characters_separate() {
        let conn = schema_conn();
        conn.execute_batch(
            r#"
            INSERT INTO messages VALUES
                ('message-1', 'session-1', 'assistant', 0, 0, '2026-07-01T01:00:00Z'),
                ('message-2', 'session-2', 'assistant', 0, 0, '2026-07-01T02:00:00Z');
            "#,
        )
        .expect("messages");
        apply_schema(&conn).expect("usage schema");
        upsert_reported_usage(
            &conn,
            "message-1",
            "session-1",
            "codex-cli",
            &ReportedUsage {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: 3,
                cache_creation_tokens: 2,
                source: "fixture".into(),
                ..ReportedUsage::default()
            },
            "2026-07-01T01:00:00Z",
        )
        .expect("reported");
        upsert_estimated_usage(
            &conn,
            "message-2",
            "session-2",
            "claude-code",
            100,
            50,
            "fixture",
            "2026-07-01T02:00:00Z",
        )
        .expect("estimated");

        let stats = aggregate_usage_statistics_at(&conn, UsageStatisticsRange::All, Local::now())
            .expect("statistics");
        assert_eq!(stats.reported.total_tokens, 20);
        assert_eq!(stats.estimated.total_characters, 150);
        assert_eq!(stats.coverage.reported_responses, 1);
        assert_eq!(stats.coverage.estimated_responses, 1);
        assert_eq!(stats.coverage.reported_percent, 50.0);
        assert_eq!(stats.counted_sessions, 2);
        assert_eq!(stats.by_agent.len(), 2);
    }

    #[test]
    fn desktop_aggregation_matches_shared_web_parity_fixture() {
        let fixture: ParityFixture = serde_json::from_str(include_str!(
            "../../tests/fixtures/usage-statistics-parity.json"
        ))
        .expect("shared parity fixture");
        let conn = schema_conn();
        apply_schema(&conn).expect("usage schema");

        for record in &fixture.records {
            conn.execute(
                "INSERT INTO messages VALUES (?1, ?2, 'assistant', 0, 0, ?3)",
                params![record.message_id, record.session_id, record.occurred_at],
            )
            .expect("fixture message");
            if record.accounting_kind == "reported" {
                upsert_reported_usage(
                    &conn,
                    &record.message_id,
                    &record.session_id,
                    &record.agent_id,
                    &ReportedUsage {
                        input_tokens: record.input_count,
                        output_tokens: record.output_count,
                        cache_read_tokens: record.cache_read_count,
                        cache_creation_tokens: record.cache_creation_count,
                        source: "shared-fixture".into(),
                        ..ReportedUsage::default()
                    },
                    &record.occurred_at,
                )
                .expect("reported fixture usage");
            } else {
                upsert_estimated_usage(
                    &conn,
                    &record.message_id,
                    &record.session_id,
                    &record.agent_id,
                    record.input_count,
                    record.output_count,
                    "shared-fixture",
                    &record.occurred_at,
                )
                .expect("estimated fixture usage");
            }
        }

        let now = DateTime::parse_from_rfc3339(&fixture.now)
            .expect("fixture now")
            .with_timezone(&Local);
        let stats =
            aggregate_usage_statistics_at(&conn, fixture.range, now).expect("fixture statistics");
        let mut actual = serde_json::to_value(stats).expect("serialized statistics");
        actual
            .as_object_mut()
            .expect("statistics object")
            .remove("generatedAt");

        assert_eq!(actual, fixture.expected);
    }

    #[test]
    fn local_ranges_start_at_runtime_local_midnight() {
        let now = Local
            .with_ymd_and_hms(2026, 7, 17, 12, 30, 0)
            .single()
            .expect("local fixture time");
        let today = range_start_at(UsageStatisticsRange::Today, now)
            .expect("today boundary")
            .expect("today start");
        let last_seven = range_start_at(UsageStatisticsRange::Last7Days, now)
            .expect("seven day boundary")
            .expect("seven day start");
        let today_local = DateTime::parse_from_rfc3339(&today)
            .expect("today timestamp")
            .with_timezone(&Local);
        let seven_local = DateTime::parse_from_rfc3339(&last_seven)
            .expect("seven day timestamp")
            .with_timezone(&Local);

        assert_eq!(today_local.date_naive().to_string(), "2026-07-17");
        assert_eq!(today_local.time().to_string(), "00:00:00");
        assert_eq!(seven_local.date_naive().to_string(), "2026-07-11");
        assert!(range_start_at(UsageStatisticsRange::All, now)
            .expect("all range")
            .is_none());
    }

    #[test]
    fn empty_aggregation_has_zero_totals_and_no_breakdowns() {
        let conn = schema_conn();
        apply_schema(&conn).expect("usage schema");
        let stats = aggregate_usage_statistics_at(&conn, UsageStatisticsRange::All, Local::now())
            .expect("statistics");

        assert_eq!(stats.reported.total_tokens, 0);
        assert_eq!(stats.estimated.total_characters, 0);
        assert_eq!(stats.coverage.total_responses, 0);
        assert_eq!(stats.coverage.reported_percent, 0.0);
        assert!(stats.daily.is_empty());
        assert!(stats.by_agent.is_empty());
    }

    #[test]
    fn bounded_aggregation_uses_occurrence_index_range_search() {
        let conn = schema_conn();
        apply_schema(&conn).expect("usage schema");
        let sql = format!(
            "EXPLAIN QUERY PLAN SELECT {AGGREGATE_COLUMNS},
                COALESCE(SUM(CASE WHEN accounting_kind = 'reported' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN accounting_kind = 'estimated' THEN 1 ELSE 0 END), 0),
                COUNT(DISTINCT session_id)
             FROM usage_records {}",
            usage_filter_clause(Some("2026-07-01T00:00:00Z")),
        );
        let details = conn
            .prepare(&sql)
            .expect("query plan")
            .query_map(params!["2026-07-01T00:00:00Z"], |row| {
                row.get::<_, String>(3)
            })
            .expect("plan rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("plan details")
            .join("\n");

        assert!(details.contains("SEARCH usage_records"), "{details}");
        assert!(
            details.contains("idx_usage_records_occurred_at"),
            "{details}"
        );
        assert_eq!(usage_filter_clause(None), "");
    }

    #[test]
    fn deleting_a_session_cascades_usage_records() {
        let conn = schema_conn();
        conn.execute(
            "INSERT INTO messages VALUES (?1, ?2, 'assistant', 0, 0, ?3)",
            params!["message-1", "session-1", "2026-07-01T01:00:00Z"],
        )
        .expect("message");
        apply_schema(&conn).expect("usage schema");
        upsert_estimated_usage(
            &conn,
            "message-1",
            "session-1",
            "codex-cli",
            10,
            20,
            "fixture",
            "2026-07-01T01:00:00Z",
        )
        .expect("usage");

        conn.execute("DELETE FROM sessions WHERE id = 'session-1'", [])
            .expect("delete session");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_records", [], |row| row.get(0))
            .expect("usage count");
        assert_eq!(count, 0);
    }
}
