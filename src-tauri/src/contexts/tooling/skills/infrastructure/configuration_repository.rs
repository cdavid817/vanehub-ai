#![cfg_attr(not(test), allow(dead_code))]

use crate::contexts::tooling::skills::application::SkillApplicationError;
use crate::contexts::tooling::skills::domain::{
    SkillConfigDrift, SkillConfigRevision, SkillConfigScope, SkillConfigValue,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillConfigCleanupState {
    None,
    Pending,
    Failed,
}

impl SkillConfigCleanupState {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Pending => "pending",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "pending" => Self::Pending,
            "failed" => Self::Failed,
            _ => Self::None,
        }
    }
}

fn drift_as_str(drift: SkillConfigDrift) -> &'static str {
    drift.as_str()
}

fn parse_drift(value: &str) -> SkillConfigDrift {
    match value {
        "migration-required" => SkillConfigDrift::MigrationRequired,
        "invalid" => SkillConfigDrift::Invalid,
        _ => SkillConfigDrift::Compatible,
    }
}

fn parse_scope(value: &str) -> SkillConfigScope {
    match value {
        "project" => SkillConfigScope::Project,
        _ => SkillConfigScope::User,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredSkillConfiguration {
    pub(crate) skill_id: String,
    pub(crate) scope: SkillConfigScope,
    pub(crate) workspace_identity: String,
    pub(crate) schema_hash: String,
    pub(crate) base_revision: String,
    pub(crate) stored_revision: SkillConfigRevision,
    pub(crate) validation_state: SkillConfigDrift,
    pub(crate) values: Vec<(String, SkillConfigValue)>,
    pub(crate) secret_keys: Vec<String>,
    pub(crate) orphaned_at: Option<String>,
    pub(crate) cleanup_state: SkillConfigCleanupState,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigurationSave {
    pub(crate) skill_id: String,
    pub(crate) scope: SkillConfigScope,
    pub(crate) workspace_identity: String,
    pub(crate) schema_hash: String,
    pub(crate) base_revision: String,
    /// `None` asserts that no record exists yet. Any mismatch is stale, including the case where
    /// a record appeared between the caller's read and its write.
    pub(crate) expected_revision: Option<SkillConfigRevision>,
    pub(crate) values: Vec<(String, SkillConfigValue)>,
    pub(crate) secret_keys: Vec<String>,
    pub(crate) validation_state: SkillConfigDrift,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SkillConfigurationWrite {
    Saved(StoredSkillConfiguration),
    /// Carries the current record so the caller can refresh without a second read racing again.
    Stale(Option<StoredSkillConfiguration>),
}

#[derive(Clone)]
pub(crate) struct SqliteSkillConfigurationRepository {
    database: NativeDatabase,
}

impl SqliteSkillConfigurationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn load(
        &self,
        skill_id: &str,
        scope: SkillConfigScope,
        workspace_identity: &str,
    ) -> Result<Option<StoredSkillConfiguration>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        load_record(&connection, skill_id, scope, workspace_identity)
    }

    pub(crate) fn load_all_scopes(
        &self,
        skill_id: &str,
        workspace_identity: &str,
    ) -> Result<Vec<StoredSkillConfiguration>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut records = Vec::new();
        if let Some(user) = load_record(&connection, skill_id, SkillConfigScope::User, "")? {
            records.push(user);
        }
        if !workspace_identity.is_empty() {
            if let Some(project) = load_record(
                &connection,
                skill_id,
                SkillConfigScope::Project,
                workspace_identity,
            )? {
                records.push(project);
            }
        }
        Ok(records)
    }

    /// Compare-and-swap on the stored revision. The read, the comparison, and the write share one
    /// transaction, so a concurrent writer either loses the race and is told it is stale, or wins
    /// and leaves the loser's prior record fully intact — never half-applied.
    pub(crate) fn save(
        &self,
        request: &SkillConfigurationSave,
    ) -> Result<SkillConfigurationWrite, SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        // IMMEDIATE, not the default DEFERRED. A deferred transaction takes a read lock and only
        // upgrades to a write lock at the INSERT; two writers that both hold the read lock cannot
        // both upgrade, and SQLite returns "database is locked" at once rather than waiting,
        // because waiting could not resolve it. Taking the write lock up front makes the second
        // writer queue on the busy timeout and then observe the first one's revision, which is
        // what turns a lock error into the stale result the caller can act on.
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(app_error)?;
        let current = load_record(
            &transaction,
            &request.skill_id,
            request.scope,
            &request.workspace_identity,
        )?;
        let expected = request.expected_revision;
        let matches = match (&current, expected) {
            (None, None) => true,
            (Some(record), Some(revision)) => record.stored_revision == revision,
            _ => false,
        };
        if !matches {
            // Nothing was written, so the prior record stands exactly as it was.
            return Ok(SkillConfigurationWrite::Stale(current));
        }
        let next = current
            .as_ref()
            .map(|record| record.stored_revision)
            .unwrap_or(SkillConfigRevision::INITIAL)
            .next()
            .map_err(|error| SkillApplicationError::Validation(error.to_string()))?;
        let now = SystemClock.rfc3339();
        let values = encode_values(&request.values);
        let secrets = serde_json::to_string(&request.secret_keys)
            .map_err(|error| SkillApplicationError::Validation(error.to_string()))?;
        transaction
            .execute(
                r#"
                INSERT INTO skill_configuration_records
                    (skill_id, scope, workspace_identity, schema_hash, base_revision,
                     stored_revision, validation_state, values_json, secret_keys_json,
                     created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                ON CONFLICT (skill_id, scope, workspace_identity) DO UPDATE SET
                    schema_hash = excluded.schema_hash,
                    base_revision = excluded.base_revision,
                    stored_revision = excluded.stored_revision,
                    validation_state = excluded.validation_state,
                    values_json = excluded.values_json,
                    secret_keys_json = excluded.secret_keys_json,
                    updated_at = excluded.updated_at,
                    orphaned_at = NULL,
                    cleanup_state = 'none'
                "#,
                params![
                    request.skill_id,
                    request.scope.as_str(),
                    request.workspace_identity,
                    request.schema_hash,
                    request.base_revision,
                    next.value() as i64,
                    drift_as_str(request.validation_state),
                    values,
                    secrets,
                    now,
                ],
            )
            .map_err(app_error)?;
        let saved = load_record(
            &transaction,
            &request.skill_id,
            request.scope,
            &request.workspace_identity,
        )?
        .ok_or_else(|| {
            SkillApplicationError::Validation("saved configuration disappeared".to_string())
        })?;
        transaction.commit().map_err(app_error)?;
        Ok(SkillConfigurationWrite::Saved(saved))
    }

    /// Removes one property from one scope. The record survives with its other properties, so the
    /// reset property inherits from the next scope down rather than becoming globally unset.
    pub(crate) fn reset_property(
        &self,
        skill_id: &str,
        scope: SkillConfigScope,
        workspace_identity: &str,
        key: &str,
        expected_revision: SkillConfigRevision,
    ) -> Result<SkillConfigurationWrite, SkillApplicationError> {
        let Some(current) = self.load(skill_id, scope, workspace_identity)? else {
            return Ok(SkillConfigurationWrite::Stale(None));
        };
        if current.stored_revision != expected_revision {
            return Ok(SkillConfigurationWrite::Stale(Some(current)));
        }
        let values = current
            .values
            .iter()
            .filter(|(stored, _)| stored != key)
            .cloned()
            .collect::<Vec<_>>();
        let secret_keys = current
            .secret_keys
            .iter()
            .filter(|stored| stored.as_str() != key)
            .cloned()
            .collect::<Vec<_>>();
        self.save(&SkillConfigurationSave {
            skill_id: skill_id.to_string(),
            scope,
            workspace_identity: workspace_identity.to_string(),
            schema_hash: current.schema_hash.clone(),
            base_revision: current.base_revision.clone(),
            expected_revision: Some(expected_revision),
            values,
            secret_keys,
            validation_state: current.validation_state,
        })
    }

    /// Deletes the scope's record entirely. Absence and an empty record are not the same thing:
    /// the effective value must fall through to the next scope, not resolve to nothing.
    pub(crate) fn reset_scope(
        &self,
        skill_id: &str,
        scope: SkillConfigScope,
        workspace_identity: &str,
    ) -> Result<bool, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let removed = connection
            .execute(
                "DELETE FROM skill_configuration_records \
                 WHERE skill_id = ?1 AND scope = ?2 AND workspace_identity = ?3",
                params![skill_id, scope.as_str(), workspace_identity],
            )
            .map_err(app_error)?;
        Ok(removed > 0)
    }

    /// Retention, not deletion: a Skill that returns under the same identity finds its values
    /// again, and an explicit cleanup is what actually removes them.
    pub(crate) fn mark_orphaned(&self, skill_id: &str) -> Result<usize, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let now = SystemClock.rfc3339();
        connection
            .execute(
                "UPDATE skill_configuration_records \
                 SET orphaned_at = ?2, updated_at = ?2 WHERE skill_id = ?1 AND orphaned_at IS NULL",
                params![skill_id, now],
            )
            .map_err(app_error)
    }

    pub(crate) fn set_cleanup_state(
        &self,
        skill_id: &str,
        state: SkillConfigCleanupState,
    ) -> Result<usize, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                "UPDATE skill_configuration_records \
                 SET cleanup_state = ?2, updated_at = ?3 WHERE skill_id = ?1",
                params![skill_id, state.as_str(), SystemClock.rfc3339()],
            )
            .map_err(app_error)
    }

    pub(crate) fn purge(&self, skill_id: &str) -> Result<usize, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                "DELETE FROM skill_configuration_records WHERE skill_id = ?1",
                params![skill_id],
            )
            .map_err(app_error)
    }
}

fn load_record(
    connection: &Connection,
    skill_id: &str,
    scope: SkillConfigScope,
    workspace_identity: &str,
) -> Result<Option<StoredSkillConfiguration>, SkillApplicationError> {
    connection
        .query_row(
            "SELECT skill_id, scope, workspace_identity, schema_hash, base_revision, \
             stored_revision, validation_state, values_json, secret_keys_json, orphaned_at, \
             cleanup_state FROM skill_configuration_records \
             WHERE skill_id = ?1 AND scope = ?2 AND workspace_identity = ?3",
            params![skill_id, scope.as_str(), workspace_identity],
            |row| {
                Ok(StoredSkillConfiguration {
                    skill_id: row.get(0)?,
                    scope: parse_scope(&row.get::<_, String>(1)?),
                    workspace_identity: row.get(2)?,
                    schema_hash: row.get(3)?,
                    base_revision: row.get(4)?,
                    stored_revision: SkillConfigRevision::new(row.get::<_, i64>(5)?.max(0) as u64),
                    validation_state: parse_drift(&row.get::<_, String>(6)?),
                    values: decode_values(&row.get::<_, String>(7)?),
                    secret_keys: serde_json::from_str(&row.get::<_, String>(8)?)
                        .unwrap_or_default(),
                    orphaned_at: row.get(9)?,
                    cleanup_state: SkillConfigCleanupState::parse(&row.get::<_, String>(10)?),
                })
            },
        )
        .optional()
        .map_err(app_error)
}

/// Values are tagged rather than written as bare JSON. `3` and `3.0` are the same JSON number, and
/// drift classification has to be able to tell an integer property that was retyped to a number
/// from one that was not.
fn encode_values(values: &[(String, SkillConfigValue)]) -> String {
    let mut map = Map::new();
    for (key, value) in values {
        map.insert(key.clone(), encode_value(value));
    }
    Value::Object(map).to_string()
}

fn encode_value(value: &SkillConfigValue) -> Value {
    match value {
        SkillConfigValue::Text(text) => json!({ "t": "s", "v": text }),
        SkillConfigValue::Integer(number) => json!({ "t": "i", "v": number }),
        SkillConfigValue::Number(number) => json!({ "t": "n", "v": number }),
        SkillConfigValue::Boolean(flag) => json!({ "t": "b", "v": flag }),
        SkillConfigValue::List(items) => {
            json!({ "t": "l", "v": items.iter().map(encode_value).collect::<Vec<_>>() })
        }
    }
}

fn decode_values(raw: &str) -> Vec<(String, SkillConfigValue)> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) else {
        // A corrupt document reads as no stored values rather than propagating a parse error:
        // the record still carries its revision witness, so a save can replace it, and drift
        // classification reports the record as invalid rather than the Skill as broken.
        return Vec::new();
    };
    let mut values = map
        .into_iter()
        .filter_map(|(key, value)| decode_value(&value).map(|value| (key, value)))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.0.cmp(&right.0));
    values
}

fn decode_value(value: &Value) -> Option<SkillConfigValue> {
    match value.get("t")?.as_str()? {
        "s" => Some(SkillConfigValue::Text(
            value.get("v")?.as_str()?.to_string(),
        )),
        "i" => Some(SkillConfigValue::Integer(value.get("v")?.as_i64()?)),
        "n" => Some(SkillConfigValue::Number(value.get("v")?.as_f64()?)),
        "b" => Some(SkillConfigValue::Boolean(value.get("v")?.as_bool()?)),
        "l" => Some(SkillConfigValue::List(
            value
                .get("v")?
                .as_array()?
                .iter()
                .filter_map(decode_value)
                .collect(),
        )),
        _ => None,
    }
}

fn app_error(error: impl std::fmt::Display) -> SkillApplicationError {
    SkillApplicationError::Repository(error.to_string())
}

#[cfg(test)]
#[path = "configuration_repository_tests.rs"]
mod tests;
