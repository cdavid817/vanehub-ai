//! SQLite adapters for capability-gate desired state and audit.

use crate::contexts::tooling::extension_platform::application::{
    FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateClock, FeatureGateDegradationEntry,
    FeatureGateRepository, FeatureGateWrite, PersistedFeatureGate,
};
use crate::contexts::tooling::extension_platform::domain::{
    ExtensionPlatformFeature, FeatureGateError,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

pub(crate) struct SqliteFeatureGateRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteFeatureGateRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, FeatureGateError> {
        self.database
            .connection()
            .map_err(|error| FeatureGateError::Storage(error.to_string()))
    }
}

fn storage(error: rusqlite::Error) -> FeatureGateError {
    FeatureGateError::Storage(error.to_string())
}

impl FeatureGateRepository for SqliteFeatureGateRepository {
    fn load_all(&self) -> Result<Vec<PersistedFeatureGate>, FeatureGateError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT feature, desired_enabled, revision, updated_at, updated_by, reason \
                 FROM extension_platform_feature_gates",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? != 0,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;

        // A row whose key no longer maps to a known gate is dropped rather than failing the whole
        // read: a retired or not-yet-known gate name must not make every other gate unreadable,
        // and dropping it resolves that gate to disabled, which is the fail-closed answer.
        Ok(rows
            .into_iter()
            .filter_map(
                |(feature, desired_enabled, revision, updated_at, updated_by, reason)| {
                    ExtensionPlatformFeature::parse(&feature).map(|feature| PersistedFeatureGate {
                        feature,
                        desired_enabled,
                        revision,
                        updated_at,
                        updated_by,
                        reason,
                    })
                },
            )
            .collect())
    }

    fn upsert(&self, write: &FeatureGateWrite) -> Result<PersistedFeatureGate, FeatureGateError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection).map_err(storage)?;

        let current_revision: i64 = transaction
            .query_row(
                "SELECT revision FROM extension_platform_feature_gates WHERE feature = ?1",
                params![write.feature.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .unwrap_or(0);

        if current_revision != write.expected_revision {
            return Err(FeatureGateError::StaleRevision {
                feature: write.feature,
                expected: write.expected_revision,
                actual: current_revision,
            });
        }

        let revision = current_revision + 1;
        transaction
            .execute(
                "INSERT INTO extension_platform_feature_gates \
                     (feature, desired_enabled, revision, updated_at, updated_by, reason) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(feature) DO UPDATE SET \
                     desired_enabled = excluded.desired_enabled, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at, \
                     updated_by = excluded.updated_by, \
                     reason = excluded.reason",
                params![
                    write.feature.as_str(),
                    i64::from(write.desired_enabled),
                    revision,
                    write.updated_at,
                    write.updated_by,
                    write.reason,
                ],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;

        Ok(PersistedFeatureGate {
            feature: write.feature,
            desired_enabled: write.desired_enabled,
            revision,
            updated_at: write.updated_at.clone(),
            updated_by: write.updated_by.clone(),
            reason: write.reason.clone(),
        })
    }
}

pub(crate) struct SqliteFeatureGateAuditSink {
    database: Arc<NativeDatabase>,
}

impl SqliteFeatureGateAuditSink {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }
}

impl FeatureGateAuditSink for SqliteFeatureGateAuditSink {
    fn record(&self, entry: &FeatureGateAuditEntry) -> Result<(), FeatureGateError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| FeatureGateError::Storage(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO extension_platform_feature_gate_audit \
                     (feature, previous_enabled, new_enabled, revision, recorded_at, actor, reason) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    entry.feature.as_str(),
                    i64::from(entry.previous_enabled),
                    i64::from(entry.new_enabled),
                    entry.revision,
                    entry.recorded_at,
                    entry.actor,
                    entry.reason,
                ],
            )
            .map_err(storage)?;
        Ok(())
    }

    fn record_degradation(
        &self,
        entry: &FeatureGateDegradationEntry,
    ) -> Result<(), FeatureGateError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| FeatureGateError::Storage(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO extension_platform_feature_gate_degradations \
                     (degradation, code, recorded_at) \
                 VALUES (?1, ?2, ?3)",
                params![entry.degradation.as_str(), entry.code, entry.recorded_at],
            )
            .map_err(storage)?;
        Ok(())
    }
}

pub(crate) struct FeatureGateSystemClock;

impl FeatureGateClock for FeatureGateSystemClock {
    fn now_rfc3339(&self) -> String {
        SystemClock.rfc3339()
    }
}
