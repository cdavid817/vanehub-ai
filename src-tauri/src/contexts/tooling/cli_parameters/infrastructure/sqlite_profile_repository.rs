use crate::contexts::tooling::cli_parameters::application::error::CliParameterApplicationError;
use crate::contexts::tooling::cli_parameters::application::models::{
    PersistedCliParameterProfile, ReplaceCliParameterProfile,
};
use crate::contexts::tooling::cli_parameters::application::ports::CliParameterProfileRepository;
use crate::contexts::tooling::cli_parameters::domain::profile::{
    StoredCliParameterProfile, StoredSelectionRow, CURRENT_SELECTION_SCHEMA_VERSION,
};
use crate::contexts::tooling::cli_parameters::domain::selection::CliParameterSelectionMap;
use crate::platform::database::{
    begin_write_transaction, DatabaseError, NativeDatabase, PooledSqlite,
};
use rusqlite::{params, Connection, OptionalExtension};

/// Additive: the per-parameter table keeps its shape and rows, and profile metadata lands beside
/// it. No user row is deleted by the migration itself, and re-running it is a no-op.
pub(crate) fn apply_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_parameter_profiles (
            agent_id TEXT PRIMARY KEY,
            revision INTEGER NOT NULL DEFAULT 0,
            selection_schema_version INTEGER NOT NULL DEFAULT 1,
            catalog_version TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
        );
        INSERT OR IGNORE INTO cli_parameter_profiles
            (agent_id, revision, selection_schema_version, catalog_version, updated_at)
        SELECT DISTINCT agent_id, 0, 1, '', COALESCE(MAX(updated_at), '')
        FROM cli_parameter_settings
        GROUP BY agent_id;
        "#,
    )
    .map_err(|error| DatabaseError::Storage(error.to_string()))
}

fn repository_error(error: impl std::fmt::Display) -> CliParameterApplicationError {
    CliParameterApplicationError::Repository(error.to_string())
}

#[derive(Clone)]
pub(crate) struct SqliteCliParameterProfileRepository {
    database: NativeDatabase,
}

impl SqliteCliParameterProfileRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, CliParameterApplicationError> {
        self.database.connection().map_err(repository_error)
    }

    #[cfg(test)]
    pub(crate) fn raw_connection_for_tests(&self) -> PooledSqlite {
        self.database.connection().expect("test connection")
    }
}

struct ProfileMetadata {
    revision: i64,
    selection_schema_version: u32,
    catalog_version: String,
    updated_at: Option<String>,
}

fn read_metadata(conn: &Connection, agent_id: &str) -> Result<ProfileMetadata, rusqlite::Error> {
    let row = conn
        .query_row(
            "SELECT revision, selection_schema_version, catalog_version, updated_at
             FROM cli_parameter_profiles WHERE agent_id = ?1",
            params![agent_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    Ok(match row {
        Some((revision, schema_version, catalog_version, updated_at)) => ProfileMetadata {
            revision,
            selection_schema_version: schema_version.max(1) as u32,
            catalog_version,
            updated_at: (!updated_at.is_empty()).then_some(updated_at),
        },
        None => ProfileMetadata {
            revision: 0,
            selection_schema_version: 1,
            catalog_version: String::new(),
            updated_at: None,
        },
    })
}

fn write_rows(
    conn: &Connection,
    agent_id: &str,
    selections: &CliParameterSelectionMap,
    now: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM cli_parameter_settings WHERE agent_id = ?1",
        params![agent_id],
    )?;
    for (parameter_id, selection) in selections {
        // Inheritance is the absence of an override, so it is not stored as a row.
        if selection.is_inherit() {
            continue;
        }
        let value_json = serde_json::to_string(selection).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        conn.execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at)
             VALUES (?1, ?2, 1, ?3, ?4)",
            params![agent_id, parameter_id, value_json, now],
        )?;
    }
    Ok(())
}

fn commit_metadata(
    conn: &Connection,
    agent_id: &str,
    revision: i64,
    catalog_version: &str,
    now: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO cli_parameter_profiles
            (agent_id, revision, selection_schema_version, catalog_version, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(agent_id) DO UPDATE SET
            revision = excluded.revision,
            selection_schema_version = excluded.selection_schema_version,
            catalog_version = excluded.catalog_version,
            updated_at = excluded.updated_at",
        params![
            agent_id,
            revision,
            CURRENT_SELECTION_SCHEMA_VERSION as i64,
            catalog_version,
            now
        ],
    )?;
    Ok(())
}

impl CliParameterProfileRepository for SqliteCliParameterProfileRepository {
    fn load(
        &self,
        agent_id: &str,
    ) -> Result<StoredCliParameterProfile, CliParameterApplicationError> {
        let conn = self.connection()?;
        let metadata = read_metadata(&conn, agent_id).map_err(repository_error)?;
        let mut statement = conn
            .prepare(
                "SELECT parameter_id, value_json FROM cli_parameter_settings
                 WHERE agent_id = ?1 AND enabled = 1 ORDER BY parameter_id",
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(params![agent_id], |row| {
                Ok(StoredSelectionRow {
                    parameter_id: row.get(0)?,
                    value_json: row.get(1)?,
                })
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        Ok(StoredCliParameterProfile {
            agent_id: agent_id.to_string(),
            revision: metadata.revision,
            selection_schema_version: metadata.selection_schema_version,
            catalog_version: metadata.catalog_version,
            updated_at: metadata.updated_at,
            rows,
        })
    }

    fn replace_if_revision(
        &self,
        mutation: ReplaceCliParameterProfile,
    ) -> Result<PersistedCliParameterProfile, CliParameterApplicationError> {
        let conn = self.connection()?;
        let now = chrono::Utc::now().to_rfc3339();
        // Compare-and-swap: the revision read here decides whether the write below happens, so the
        // two must be one atomic step. A deferred transaction would take a shared lock at the read
        // and then have to upgrade it at the write, which SQLite refuses outright without ever
        // consulting `busy_timeout` -- so two concurrent saves would fail immediately rather than
        // one waiting for the other.
        let transaction = begin_write_transaction(&conn).map_err(repository_error)?;
        let metadata = read_metadata(&transaction, &mutation.agent_id).map_err(repository_error)?;
        if metadata.revision != mutation.expected_revision {
            return Err(CliParameterApplicationError::RevisionConflict {
                agent_id: mutation.agent_id,
                expected_revision: mutation.expected_revision,
                actual_revision: metadata.revision,
            });
        }
        let revision = metadata.revision + 1;
        write_rows(&transaction, &mutation.agent_id, &mutation.selections, &now)
            .map_err(repository_error)?;
        commit_metadata(
            &transaction,
            &mutation.agent_id,
            revision,
            &mutation.catalog_version,
            &now,
        )
        .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(PersistedCliParameterProfile {
            agent_id: mutation.agent_id,
            revision,
            catalog_version: mutation.catalog_version,
            updated_at: now,
        })
    }

    fn reset_if_revision(
        &self,
        agent_id: &str,
        expected_revision: i64,
        catalog_version: &str,
    ) -> Result<PersistedCliParameterProfile, CliParameterApplicationError> {
        let conn = self.connection()?;
        let now = chrono::Utc::now().to_rfc3339();
        // Same compare-and-swap shape as `replace_if_revision`, and the same reason for taking the
        // write lock up front rather than upgrading into it.
        let transaction = begin_write_transaction(&conn).map_err(repository_error)?;
        let metadata = read_metadata(&transaction, agent_id).map_err(repository_error)?;
        if metadata.revision != expected_revision {
            return Err(CliParameterApplicationError::RevisionConflict {
                agent_id: agent_id.to_string(),
                expected_revision,
                actual_revision: metadata.revision,
            });
        }
        let revision = metadata.revision + 1;
        write_rows(
            &transaction,
            agent_id,
            &CliParameterSelectionMap::new(),
            &now,
        )
        .map_err(repository_error)?;
        commit_metadata(&transaction, agent_id, revision, catalog_version, &now)
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(PersistedCliParameterProfile {
            agent_id: agent_id.to_string(),
            revision,
            catalog_version: catalog_version.to_string(),
            updated_at: now,
        })
    }
}
