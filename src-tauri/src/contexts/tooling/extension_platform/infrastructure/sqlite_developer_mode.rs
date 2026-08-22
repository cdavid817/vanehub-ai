// Assembled in bootstrap with the settings surface in task 12; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapter for the Developer Mode switch and its audit trail.

use crate::contexts::tooling::extension_platform::application::{
    DeveloperModeAuditEntry, DeveloperModeAuditSink, DeveloperModeRepository, DeveloperModeView,
};
use crate::contexts::tooling::extension_platform::domain::{DeveloperMode, DeveloperModeError};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

pub(crate) struct SqliteDeveloperModeRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteDeveloperModeRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, DeveloperModeError> {
        self.database
            .connection()
            .map_err(|error| DeveloperModeError::Storage(error.to_string()))
    }
}

fn storage(error: rusqlite::Error) -> DeveloperModeError {
    DeveloperModeError::Storage(error.to_string())
}

impl DeveloperModeRepository for SqliteDeveloperModeRepository {
    fn load(&self) -> Result<DeveloperModeView, DeveloperModeError> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                "SELECT enabled, revision, updated_at, updated_by, reason \
                 FROM extension_platform_developer_mode WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? != 0,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)?;

        Ok(match row {
            Some((enabled, revision, updated_at, updated_by, reason)) => DeveloperModeView {
                mode: DeveloperMode::from_enabled(enabled),
                revision,
                updated_at: Some(updated_at),
                updated_by: Some(updated_by),
                reason,
            },
            None => DeveloperModeView {
                mode: DeveloperMode::Off,
                revision: 0,
                updated_at: None,
                updated_by: None,
                reason: None,
            },
        })
    }

    fn store(
        &self,
        mode: DeveloperMode,
        revision: i64,
        updated_at: &str,
        updated_by: &str,
        reason: Option<&str>,
    ) -> Result<DeveloperModeView, DeveloperModeError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO extension_platform_developer_mode \
                     (id, enabled, revision, updated_at, updated_by, reason) \
                 VALUES (1, ?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(id) DO UPDATE SET \
                     enabled = excluded.enabled, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at, \
                     updated_by = excluded.updated_by, \
                     reason = excluded.reason",
                params![
                    i64::from(mode.is_on()),
                    revision,
                    updated_at,
                    updated_by,
                    reason
                ],
            )
            .map_err(storage)?;
        Ok(DeveloperModeView {
            mode,
            revision,
            updated_at: Some(updated_at.to_string()),
            updated_by: Some(updated_by.to_string()),
            reason: reason.map(str::to_string),
        })
    }
}

pub(crate) struct SqliteDeveloperModeAuditSink {
    database: Arc<NativeDatabase>,
}

impl SqliteDeveloperModeAuditSink {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }
}

impl DeveloperModeAuditSink for SqliteDeveloperModeAuditSink {
    fn record(&self, entry: &DeveloperModeAuditEntry) -> Result<(), DeveloperModeError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| DeveloperModeError::Storage(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO extension_platform_developer_mode_audit \
                     (previous_enabled, new_enabled, revision, recorded_at, actor, reason) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
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
}
