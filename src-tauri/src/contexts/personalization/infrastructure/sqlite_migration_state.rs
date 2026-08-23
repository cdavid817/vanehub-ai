use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::params;

use crate::contexts::personalization::application::{
    MigrationStatePort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::MigrationState;
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_timestamp(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    DateTime::parse_from_rfc3339(&value)
        .map(|parsed| Some(parsed.with_timezone(&Utc)))
        .map_err(|error| {
            PersonalizationApplicationError::Storage(format!(
                "personalization migration state holds an unreadable timestamp: {error}"
            ))
        })
}

/// The singleton row that says whether stored personalization data is trustworthy yet.
#[derive(Clone)]
pub(crate) struct SqliteMigrationState {
    database: NativeDatabase,
}

impl SqliteMigrationState {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }
}

impl MigrationStatePort for SqliteMigrationState {
    fn load(&self) -> Result<MigrationState> {
        let conn = self.connection()?;
        let (generation, started_at, completed_at, last_error_code, repair_required): (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
        ) = conn
            .query_row(
                "SELECT generation, started_at, completed_at, last_error_code, repair_required \
                 FROM personalization_migration_state WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(storage)?;

        Ok(MigrationState {
            generation: u64::try_from(generation).unwrap_or_default(),
            started_at: parse_timestamp(started_at)?,
            completed_at: parse_timestamp(completed_at)?,
            last_error_code,
            repair_required: repair_required != 0,
        })
    }

    fn save(&self, state: &MigrationState) -> Result<()> {
        let conn = self.connection()?;
        // The row is seeded by the migration, so this is always an update. Using UPDATE rather
        // than an upsert keeps the `id = 1` check meaningful: a second row would mean two answers
        // to "is memory safe to use".
        let changed = conn
            .execute(
                "UPDATE personalization_migration_state
                 SET generation = ?1, started_at = ?2, completed_at = ?3, last_error_code = ?4,
                     repair_required = ?5
                 WHERE id = 1",
                params![
                    i64::try_from(state.generation).unwrap_or(i64::MAX),
                    state.started_at.map(timestamp),
                    state.completed_at.map(timestamp),
                    state.last_error_code.as_deref(),
                    i64::from(state.repair_required),
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(PersonalizationApplicationError::Storage(
                "the personalization migration-state row is missing".to_string(),
            ));
        }
        Ok(())
    }
}
