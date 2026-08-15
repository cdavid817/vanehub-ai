use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, ContextQualityRepository,
};
use crate::contexts::agent_runtime::domain::{
    ContextAssessmentInvariants, ContextAssessmentMeasurementQuality, ContextAssessmentOutcome,
    ContextAssessmentPath, ContextAssessmentReason, ContextAssessmentTriggerSource,
    ContextQualityAssessment, ContextQualityAssessmentPage, ContextQualityAssessmentRecord,
    ContextQualitySummary,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension, Row};
use std::collections::BTreeMap;

const MAX_PAGE_SIZE: u32 = 100;

#[derive(Clone)]
pub(crate) struct SqliteContextQualityRepository {
    database: NativeDatabase,
}

impl SqliteContextQualityRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, AgentRuntimeApplicationError> {
        self.database.connection().map_err(storage_error)
    }
}

impl ContextQualityRepository for SqliteContextQualityRepository {
    fn append_and_prune(
        &self,
        record: &ContextQualityAssessmentRecord,
        retention_cutoff: &str,
        hard_limit: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if retention_cutoff.trim().is_empty() || hard_limit == 0 {
            return Err(validation_error(
                "invalid context quality retention boundary",
            ));
        }
        let hard_limit = checked_i64(hard_limit, "context quality hard limit")?;
        let assessment = &record.assessment;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                r#"
                INSERT OR IGNORE INTO context_quality_assessments (
                    attempt_id, session_correlation, decision_sequence, recorded_at, outcome,
                    path, reason, trigger_source, before_characters, after_characters,
                    saved_characters, before_tokens, after_tokens, saved_tokens,
                    measurement_quality, protocol_complete, protected_retained,
                    verbatim_retained, reinjection_complete, assessment_version,
                    context_policy_version, optimizer_version, verifier_version
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
                )
                "#,
                params![
                    assessment.attempt_id,
                    record.session_correlation,
                    checked_i64(assessment.decision_sequence, "decision sequence")?,
                    record.recorded_at,
                    assessment.outcome.as_str(),
                    assessment.path.map(ContextAssessmentPath::as_str),
                    assessment.reason.map(ContextAssessmentReason::as_str),
                    assessment
                        .trigger_source
                        .map(ContextAssessmentTriggerSource::as_str),
                    checked_i64(assessment.before_characters, "before characters")?,
                    checked_i64(assessment.after_characters, "after characters")?,
                    checked_i64(assessment.saved_characters, "saved characters")?,
                    optional_i64(assessment.before_tokens, "before tokens")?,
                    optional_i64(assessment.after_tokens, "after tokens")?,
                    optional_i64(assessment.saved_tokens, "saved tokens")?,
                    assessment.measurement_quality.as_str(),
                    assessment.invariants.map(|value| value.protocol_complete),
                    assessment.invariants.map(|value| value.protected_retained),
                    assessment.invariants.map(|value| value.verbatim_retained),
                    assessment
                        .invariants
                        .map(|value| value.reinjection_complete),
                    assessment.version,
                    assessment.context_policy_version,
                    assessment.optimizer_version,
                    assessment.verifier_version,
                ],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM context_quality_assessments WHERE recorded_at < ?1",
                [retention_cutoff],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"
                DELETE FROM context_quality_assessments
                WHERE attempt_id IN (
                    SELECT attempt_id FROM context_quality_assessments
                    ORDER BY recorded_at DESC, attempt_id DESC
                    LIMIT -1 OFFSET ?1
                )
                "#,
                [hard_limit],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn list(
        &self,
        since: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<ContextQualityAssessmentPage, AgentRuntimeApplicationError> {
        if since.trim().is_empty() || !(1..=MAX_PAGE_SIZE).contains(&limit) {
            return Err(validation_error("invalid context quality history query"));
        }
        let connection = self.connection()?;
        let cursor_key = cursor
            .map(|attempt_id| {
                connection
                    .query_row(
                        "SELECT recorded_at, attempt_id FROM context_quality_assessments WHERE attempt_id = ?1",
                        [attempt_id],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or_else(|| validation_error("invalid context quality cursor"))
            })
            .transpose()?;
        let query_limit = i64::from(limit) + 1;
        let rows = if let Some((recorded_at, attempt_id)) = cursor_key {
            let mut statement = connection
                .prepare(&format!(
                    "{} WHERE recorded_at >= ?1 AND (recorded_at < ?2 OR (recorded_at = ?2 AND attempt_id < ?3)) ORDER BY recorded_at DESC, attempt_id DESC LIMIT ?4",
                    assessment_select()
                ))
                .map_err(storage_error)?;
            let collected = statement
                .query_map(
                    params![since, recorded_at, attempt_id, query_limit],
                    RawAssessmentRow::read,
                )
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            collected
        } else {
            let mut statement = connection
                .prepare(&format!(
                    "{} WHERE recorded_at >= ?1 ORDER BY recorded_at DESC, attempt_id DESC LIMIT ?2",
                    assessment_select()
                ))
                .map_err(storage_error)?;
            let collected = statement
                .query_map(params![since, query_limit], RawAssessmentRow::read)
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            collected
        };
        let mut items = rows
            .into_iter()
            .map(RawAssessmentRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = (items.len() > limit as usize)
            .then(|| items[limit as usize - 1].assessment.attempt_id.clone());
        items.truncate(limit as usize);
        Ok(ContextQualityAssessmentPage { items, next_cursor })
    }

    fn summarize(
        &self,
        since: &str,
    ) -> Result<ContextQualitySummary, AgentRuntimeApplicationError> {
        if since.trim().is_empty() {
            return Err(validation_error("invalid context quality summary range"));
        }
        let connection = self.connection()?;
        let (evaluated, saved_characters, saved_tokens, token_measurement_count, earliest, latest) =
            connection
                .query_row(
                    r#"
                    SELECT COUNT(*), COALESCE(SUM(saved_characters), 0),
                           COALESCE(SUM(saved_tokens), 0), COUNT(saved_tokens),
                           MIN(recorded_at), MAX(recorded_at)
                    FROM context_quality_assessments WHERE recorded_at >= ?1
                    "#,
                    [since],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    },
                )
                .map_err(storage_error)?;
        Ok(ContextQualitySummary {
            evaluated: checked_u64(evaluated, "evaluated count")?,
            saved_characters: checked_u64(saved_characters, "saved characters")?,
            saved_tokens: checked_u64(saved_tokens, "saved tokens")?,
            token_measurement_count: checked_u64(
                token_measurement_count,
                "token measurement count",
            )?,
            outcomes: read_distribution(&connection, "outcome", since)?,
            paths: read_distribution(&connection, "path", since)?,
            qualities: read_distribution(&connection, "measurement_quality", since)?,
            reasons: read_distribution(&connection, "reason", since)?,
            policy_versions: read_distribution(&connection, "context_policy_version", since)?,
            earliest_recorded_at: earliest,
            latest_recorded_at: latest,
        })
    }
}

fn assessment_select() -> &'static str {
    r#"
    SELECT attempt_id, session_correlation, decision_sequence, recorded_at, outcome, path,
           reason, trigger_source, before_characters, after_characters, saved_characters,
           before_tokens, after_tokens, saved_tokens, measurement_quality, protocol_complete,
           protected_retained, verbatim_retained, reinjection_complete, assessment_version,
           context_policy_version, optimizer_version, verifier_version
    FROM context_quality_assessments
    "#
}

struct RawAssessmentRow {
    attempt_id: String,
    session_correlation: Option<String>,
    decision_sequence: i64,
    recorded_at: String,
    outcome: String,
    path: Option<String>,
    reason: Option<String>,
    trigger_source: Option<String>,
    before_characters: i64,
    after_characters: i64,
    saved_characters: i64,
    before_tokens: Option<i64>,
    after_tokens: Option<i64>,
    saved_tokens: Option<i64>,
    measurement_quality: String,
    protocol_complete: Option<bool>,
    protected_retained: Option<bool>,
    verbatim_retained: Option<bool>,
    reinjection_complete: Option<bool>,
    assessment_version: String,
    context_policy_version: String,
    optimizer_version: String,
    verifier_version: String,
}

impl RawAssessmentRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            attempt_id: row.get(0)?,
            session_correlation: row.get(1)?,
            decision_sequence: row.get(2)?,
            recorded_at: row.get(3)?,
            outcome: row.get(4)?,
            path: row.get(5)?,
            reason: row.get(6)?,
            trigger_source: row.get(7)?,
            before_characters: row.get(8)?,
            after_characters: row.get(9)?,
            saved_characters: row.get(10)?,
            before_tokens: row.get(11)?,
            after_tokens: row.get(12)?,
            saved_tokens: row.get(13)?,
            measurement_quality: row.get(14)?,
            protocol_complete: row.get(15)?,
            protected_retained: row.get(16)?,
            verbatim_retained: row.get(17)?,
            reinjection_complete: row.get(18)?,
            assessment_version: row.get(19)?,
            context_policy_version: row.get(20)?,
            optimizer_version: row.get(21)?,
            verifier_version: row.get(22)?,
        })
    }

    fn into_domain(self) -> Result<ContextQualityAssessmentRecord, AgentRuntimeApplicationError> {
        let invariants = match (
            self.protocol_complete,
            self.protected_retained,
            self.verbatim_retained,
            self.reinjection_complete,
        ) {
            (None, None, None, None) => None,
            (Some(protocol), Some(protected), Some(verbatim), Some(reinjection)) => {
                Some(ContextAssessmentInvariants {
                    protocol_complete: protocol,
                    protected_retained: protected,
                    verbatim_retained: verbatim,
                    reinjection_complete: reinjection,
                })
            }
            _ => return Err(storage_error("partial context quality invariants")),
        };
        Ok(ContextQualityAssessmentRecord {
            session_correlation: self.session_correlation,
            recorded_at: self.recorded_at,
            assessment: ContextQualityAssessment {
                version: self.assessment_version,
                attempt_id: self.attempt_id,
                decision_sequence: checked_u64(self.decision_sequence, "decision sequence")?,
                outcome: parse_value(&self.outcome, ContextAssessmentOutcome::parse, "outcome")?,
                path: parse_optional(self.path, ContextAssessmentPath::parse, "path")?,
                reason: parse_optional(self.reason, ContextAssessmentReason::parse, "reason")?,
                trigger_source: parse_optional(
                    self.trigger_source,
                    ContextAssessmentTriggerSource::parse,
                    "trigger source",
                )?,
                before_characters: checked_u64(self.before_characters, "before characters")?,
                after_characters: checked_u64(self.after_characters, "after characters")?,
                saved_characters: checked_u64(self.saved_characters, "saved characters")?,
                before_tokens: optional_u64(self.before_tokens, "before tokens")?,
                after_tokens: optional_u64(self.after_tokens, "after tokens")?,
                saved_tokens: optional_u64(self.saved_tokens, "saved tokens")?,
                measurement_quality: parse_value(
                    &self.measurement_quality,
                    ContextAssessmentMeasurementQuality::parse,
                    "measurement quality",
                )?,
                invariants,
                context_policy_version: self.context_policy_version,
                optimizer_version: self.optimizer_version,
                verifier_version: self.verifier_version,
            },
        })
    }
}

fn read_distribution(
    connection: &rusqlite::Connection,
    column: &str,
    since: &str,
) -> Result<BTreeMap<String, u64>, AgentRuntimeApplicationError> {
    let allowed = [
        "outcome",
        "path",
        "measurement_quality",
        "reason",
        "context_policy_version",
    ];
    if !allowed.contains(&column) {
        return Err(validation_error("unsupported context quality distribution"));
    }
    let mut statement = connection
        .prepare(&format!(
            "SELECT {column}, COUNT(*) FROM context_quality_assessments WHERE recorded_at >= ?1 AND {column} IS NOT NULL GROUP BY {column} ORDER BY {column}"
        ))
        .map_err(storage_error)?;
    let distribution = statement
        .query_map([since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(storage_error)?
        .map(|row| {
            let (key, count) = row.map_err(storage_error)?;
            Ok((key, checked_u64(count, "distribution count")?))
        })
        .collect();
    distribution
}

fn parse_value<T>(
    value: &str,
    parse: impl FnOnce(&str) -> Option<T>,
    label: &str,
) -> Result<T, AgentRuntimeApplicationError> {
    parse(value).ok_or_else(|| storage_error(format!("invalid stored {label}")))
}

fn parse_optional<T>(
    value: Option<String>,
    parse: impl Fn(&str) -> Option<T>,
    label: &str,
) -> Result<Option<T>, AgentRuntimeApplicationError> {
    value
        .map(|value| parse_value(&value, &parse, label))
        .transpose()
}

fn checked_i64(value: u64, label: &str) -> Result<i64, AgentRuntimeApplicationError> {
    i64::try_from(value).map_err(|_| validation_error(format!("{label} exceeds SQLite range")))
}

fn checked_u64(value: i64, label: &str) -> Result<u64, AgentRuntimeApplicationError> {
    u64::try_from(value).map_err(|_| storage_error(format!("negative stored {label}")))
}

fn optional_i64(
    value: Option<u64>,
    label: &str,
) -> Result<Option<i64>, AgentRuntimeApplicationError> {
    value.map(|value| checked_i64(value, label)).transpose()
}

fn optional_u64(
    value: Option<i64>,
    label: &str,
) -> Result<Option<u64>, AgentRuntimeApplicationError> {
    value.map(|value| checked_u64(value, label)).transpose()
}

fn validation_error(message: impl Into<String>) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.into())
}

fn storage_error(error: impl ToString) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::ContextQuality(error.to_string())
}

#[cfg(test)]
#[path = "context_quality_repository_tests.rs"]
mod tests;
