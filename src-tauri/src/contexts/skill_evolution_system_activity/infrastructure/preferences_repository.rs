use rusqlite::{params, OptionalExtension, Row, Transaction};
use serde::de::DeserializeOwned;

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityPreferenceUpdateOutcome {
    Updated(EvolutionActivityPreferences),
    Conflict(EvolutionActivityPreferences),
}

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn preferences(
        &self,
        scope_kind: ActivityScopeKind,
        canonical_scope_id: &str,
    ) -> Result<Option<EvolutionActivityPreferences>, ActivityProjectionRepositoryError> {
        sanitize_text(canonical_scope_id, "preferences.canonical_scope_id", 160)
            .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        load_preferences(self.connection, scope_kind, canonical_scope_id)
    }

    pub(crate) fn update_preferences(
        &self,
        requested: &EvolutionActivityPreferences,
        updated_at_ms: i64,
    ) -> Result<ActivityPreferenceUpdateOutcome, ActivityProjectionRepositoryError> {
        requested
            .validate()
            .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        if updated_at_ms < 0 {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let transaction = self.connection.unchecked_transaction()?;
        let changed = persist_preferences(&transaction, requested, updated_at_ms)?;
        if changed != 1 {
            let current = load_preferences(
                &transaction,
                requested.scope_kind,
                &requested.canonical_scope_id,
            )?
            .ok_or(ActivityProjectionRepositoryError::Conflict)?;
            return Ok(ActivityPreferenceUpdateOutcome::Conflict(current));
        }
        transaction.execute(
            "UPDATE evolution_system_activity_sessions
             SET preference_revision=?1 WHERE scope_kind=?2 AND canonical_scope_id=?3",
            params![
                to_i64(requested.revision.saturating_add(1))?,
                enum_text(requested.scope_kind)?,
                requested.canonical_scope_id,
            ],
        )?;
        let current = load_preferences(
            &transaction,
            requested.scope_kind,
            &requested.canonical_scope_id,
        )?
        .ok_or(ActivityProjectionRepositoryError::Storage)?;
        transaction.commit()?;
        Ok(ActivityPreferenceUpdateOutcome::Updated(current))
    }
}

fn persist_preferences(
    transaction: &Transaction<'_>,
    requested: &EvolutionActivityPreferences,
    updated_at_ms: i64,
) -> Result<usize, ActivityProjectionRepositoryError> {
    if requested.revision == 0 {
        return transaction
            .execute(
                "INSERT OR IGNORE INTO evolution_activity_preferences
                 (scope_kind,canonical_scope_id,visible,minimum_timeline_severity,
                  notification_threshold,digest_cadence,read_retention_days,
                 detail_retention_days,export_item_limit,export_size_limit_bytes,revision,updated_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,1,?11)",
                params![
                    enum_text(requested.scope_kind)?,
                    requested.canonical_scope_id,
                    requested.visible,
                    enum_text(requested.minimum_timeline_severity)?,
                    enum_text(requested.notification_threshold)?,
                    enum_text(requested.digest_cadence)?,
                    i64::from(requested.read_retention_days),
                    i64::from(requested.detail_retention_days),
                    i64::from(requested.export_item_limit),
                    to_i64(requested.export_size_limit_bytes)?,
                    updated_at_ms,
                ],
            )
            .map_err(Into::into);
    }
    transaction
        .execute(
            "UPDATE evolution_activity_preferences SET visible=?3,minimum_timeline_severity=?4,
             notification_threshold=?5,digest_cadence=?6,read_retention_days=?7,
             detail_retention_days=?8,export_item_limit=?9,export_size_limit_bytes=?10,
             revision=revision+1,updated_at_ms=?11
             WHERE scope_kind=?1 AND canonical_scope_id=?2 AND revision=?12",
            params![
                enum_text(requested.scope_kind)?,
                requested.canonical_scope_id,
                requested.visible,
                enum_text(requested.minimum_timeline_severity)?,
                enum_text(requested.notification_threshold)?,
                enum_text(requested.digest_cadence)?,
                i64::from(requested.read_retention_days),
                i64::from(requested.detail_retention_days),
                i64::from(requested.export_item_limit),
                to_i64(requested.export_size_limit_bytes)?,
                updated_at_ms,
                to_i64(requested.revision)?,
            ],
        )
        .map_err(Into::into)
}

fn load_preferences(
    connection: &rusqlite::Connection,
    scope_kind: ActivityScopeKind,
    canonical_scope_id: &str,
) -> Result<Option<EvolutionActivityPreferences>, ActivityProjectionRepositoryError> {
    connection
        .query_row(
            "SELECT scope_kind,canonical_scope_id,visible,minimum_timeline_severity,
             notification_threshold,digest_cadence,read_retention_days,detail_retention_days,
             export_item_limit,export_size_limit_bytes,revision
             FROM evolution_activity_preferences WHERE scope_kind=?1 AND canonical_scope_id=?2",
            params![enum_text(scope_kind)?, canonical_scope_id],
            map_preferences,
        )
        .optional()
        .map_err(Into::into)
}

fn map_preferences(row: &Row<'_>) -> rusqlite::Result<EvolutionActivityPreferences> {
    Ok(EvolutionActivityPreferences {
        scope_kind: parse_enum(&row.get::<_, String>(0)?)?,
        canonical_scope_id: row.get(1)?,
        visible: row.get(2)?,
        minimum_timeline_severity: parse_enum(&row.get::<_, String>(3)?)?,
        notification_threshold: parse_enum(&row.get::<_, String>(4)?)?,
        digest_cadence: parse_enum(&row.get::<_, String>(5)?)?,
        read_retention_days: from_i64(row.get(6)?)?,
        detail_retention_days: from_i64(row.get(7)?)?,
        export_item_limit: from_i64(row.get(8)?)?,
        export_size_limit_bytes: from_i64(row.get(9)?)?,
        revision: from_i64(row.get(10)?)?,
    })
}

fn parse_enum<T: DeserializeOwned>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn enum_text(value: impl serde::Serialize) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn to_i64<T: TryInto<i64>>(value: T) -> Result<i64, ActivityProjectionRepositoryError> {
    value
        .try_into()
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn from_i64<T: TryFrom<i64>>(value: i64) -> rusqlite::Result<T> {
    T::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, value))
}
