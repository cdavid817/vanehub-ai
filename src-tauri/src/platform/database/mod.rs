//! App-owned SQLite location, connection setup, and migration orchestration.

mod migrations;

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub(crate) use migrations::{migrate, table_has_column};

const DATABASE_FILE_NAME: &str = "vanehub.sqlite";

#[derive(Debug, Error)]
pub(crate) enum DatabaseError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("storage error: {0}")]
    Storage(String),
}

#[derive(Clone)]
pub(crate) struct NativeDatabase {
    pub(crate) db_path: PathBuf,
}

impl NativeDatabase {
    pub(crate) fn new(data_dir: PathBuf) -> Result<Self, DatabaseError> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|error| DatabaseError::Storage(error.to_string()))?;
        Ok(Self {
            db_path: database_path(&data_dir),
        })
    }

    pub(crate) fn connection(&self) -> Result<Connection, DatabaseError> {
        let connection = Connection::open(&self.db_path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&connection)?;
        crate::contexts::agent_runtime::infrastructure::seed_registry(&connection)?;
        Ok(connection)
    }
}

fn database_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DATABASE_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use rusqlite::params;

    #[test]
    fn resolves_the_existing_app_owned_database_path() {
        let directory = TempDirectory::new("native-database-path");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");

        assert_eq!(database.db_path, directory.path().join(DATABASE_FILE_NAME));
        assert!(directory.path().is_dir());
    }

    #[test]
    fn connection_applies_all_migrations_foreign_keys_and_seeds() {
        let directory = TempDirectory::new("native-database-connection");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let connection = database.connection().expect("migrated connection");

        let migration_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count");
        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign key setting");
        let agent_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
            .expect("agent count");
        let skill_table_exists: i64 = connection
            .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
            .expect("Skill table query");

        assert_eq!(migration_count, 23);
        assert_eq!(foreign_keys, 1);
        assert_eq!(agent_count, 4);
        assert_eq!(skill_table_exists, 0);
    }

    #[test]
    fn reopening_is_idempotent_and_preserves_existing_records() {
        let directory = TempDirectory::new("native-database-reopen");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let first = database.connection().expect("first connection");
        first
            .execute(
                "INSERT OR REPLACE INTO settings (key, value, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?3)",
                params!["database-adapter-test", "preserved", "1700000000"],
            )
            .expect("insert setting");
        drop(first);

        let reopened = database.connection().expect("reopened connection");
        let value: String = reopened
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["database-adapter-test"],
                |row| row.get(0),
            )
            .expect("preserved setting");
        let migration_count: i64 = reopened
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count");

        assert_eq!(value, "preserved");
        assert_eq!(migration_count, 23);
    }
}
