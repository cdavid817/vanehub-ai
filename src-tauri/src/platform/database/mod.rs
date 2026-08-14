//! App-owned SQLite location, pooled connections, and migration orchestration.

mod legacy_plan_schema;
mod migrations;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

/// How long a connection waits for a competing writer before surfacing `SQLITE_BUSY`.
/// Writers are serialized by SQLite, so brief contention is expected and should block
/// rather than fail (commands run on Tauri's blocking worker pool).
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Upper bound on live SQLite connections. WAL supports many concurrent readers against
/// a single writer, so a small pool sized near the command worker-thread count is ample.
const MAX_POOL_SIZE: u32 = 12;

/// How long a caller waits for a free pooled connection before failing, rather than
/// opening an unbounded number of handles.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// FULL makes SQLite synchronize the WAL at each recovery-critical commit. The guarantee starts
/// after SQLite reports commit success; physical media destruction remains outside this boundary.
const SQLITE_SYNCHRONOUS_FULL: i64 = 2;

pub(crate) use migrations::{migrate, table_has_column};

const DATABASE_FILE_NAME: &str = "vanehub.sqlite";

/// A checked-out pooled connection. Dereferences to `rusqlite::Connection`, so existing
/// call sites keep using `prepare` / `execute` / `transaction` unchanged.
pub(crate) type PooledSqlite = PooledConnection<SqliteConnectionManager>;

#[derive(Debug, Error)]
pub(crate) enum DatabaseError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("storage error: {0}")]
    Storage(String),
}

impl From<r2d2::Error> for DatabaseError {
    fn from(error: r2d2::Error) -> Self {
        DatabaseError::Storage(error.to_string())
    }
}

#[derive(Clone)]
pub(crate) struct NativeDatabase {
    pub(crate) db_path: PathBuf,
    pool: Pool<SqliteConnectionManager>,
}

impl NativeDatabase {
    pub(crate) fn new(data_dir: PathBuf) -> Result<Self, DatabaseError> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|error| DatabaseError::Storage(error.to_string()))?;
        let db_path = database_path(&data_dir);
        // Every physical connection is configured once here instead of on every checkout:
        // WAL lets readers proceed without blocking the writer, and the busy-timeout makes
        // contended access wait rather than fail immediately.
        let manager = SqliteConnectionManager::file(&db_path).with_init(|connection| {
            connection.busy_timeout(BUSY_TIMEOUT)?;
            connection.query_row("PRAGMA journal_mode=WAL", [], |_row| Ok(()))?;
            connection.pragma_update(None, "foreign_keys", "ON")?;
            connection.pragma_update(None, "synchronous", "FULL")?;
            let synchronous =
                connection.query_row("PRAGMA synchronous", [], |row| row.get::<_, i64>(0))?;
            if synchronous != SQLITE_SYNCHRONOUS_FULL {
                return Err(rusqlite::Error::InvalidQuery);
            }
            Ok(())
        });
        let pool = Pool::builder()
            .max_size(MAX_POOL_SIZE)
            .min_idle(Some(1))
            .connection_timeout(CONNECTION_TIMEOUT)
            .build(manager)?;
        // Migration and seeding are one-time work. `new` runs once during bootstrap,
        // before the pool is shared, so this happens exactly once for the database.
        let connection = pool.get()?;
        migrate(&connection)?;
        crate::contexts::agent_runtime::infrastructure::seed_registry(&connection)?;
        drop(connection);
        Ok(Self { db_path, pool })
    }

    pub(crate) fn connection(&self) -> Result<PooledSqlite, DatabaseError> {
        Ok(self.pool.get()?)
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
        let synchronous: i64 = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .expect("synchronous setting");
        let agent_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
            .expect("agent count");
        let skill_table_exists: i64 = connection
            .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
            .expect("Skill table query");
        let cli_config_tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('cli_config_profiles', 'cli_config_applied_state')",
                [],
                |row| row.get(0),
            )
            .expect("CLI configuration tables");
        let cli_config_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 35",
                [],
                |row| row.get(0),
            )
            .expect("CLI configuration migration");
        let skill_reliability_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 37",
                [],
                |row| row.get(0),
            )
            .expect("Skill reliability migration");
        let plan_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 49",
                [],
                |row| row.get(0),
            )
            .expect("plan migration");
        let code_index_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 50",
                [],
                |row| row.get(0),
            )
            .expect("code index migration");
        let code_index_mode_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 51",
                [],
                |row| row.get(0),
            )
            .expect("code index mode migration");
        let automatic_code_index_mode_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 52",
                [],
                |row| row.get(0),
            )
            .expect("automatic code index mode migration");
        let reconciliation_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 53",
                [],
                |row| row.get(0),
            )
            .expect("plan and code index reconciliation migration");
        let stable_participant_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 59",
                [],
                |row| row.get(0),
            )
            .expect("stable participant migration");
        let plan_agent_loop_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 62",
                [],
                |row| row.get(0),
            )
            .expect("OnePiece Plan Agent loop migration");
        let managed_im_binding_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 65",
                [],
                |row| row.get(0),
            )
            .expect("managed IM binding migration");
        let context_quality_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 70",
                [],
                |row| row.get(0),
            )
            .expect("OnePiece context quality migration");
        let context_quality_schema_objects: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'context_quality_assessments' OR name LIKE 'context_quality_assessments_%_idx'",
                [],
                |row| row.get(0),
            )
            .expect("OnePiece context quality schema");
        let lsp_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 58",
                [],
                |row| row.get(0),
            )
            .expect("LSP code intelligence migration");
        let effective_skill_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 60",
                [],
                |row| row.get(0),
            )
            .expect("effective Skill migration");
        let evidence_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 67",
                [],
                |row| row.get(0),
            )
            .expect("Skill evolution evidence migration");

        let native_tool_migration: String = connection
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 68",
                [],
                |row| row.get(0),
            )
            .expect("native tool persistence migration");

        assert_eq!(migration_count, 70);
        assert_eq!(foreign_keys, 1);
        assert_eq!(synchronous, SQLITE_SYNCHRONOUS_FULL);
        assert_eq!(agent_count, 6);
        assert_eq!(skill_table_exists, 0);
        assert_eq!(cli_config_tables, 2);
        assert_eq!(cli_config_migration, "cli-agent-applied-ownership-snapshot");
        assert_eq!(skill_reliability_migration, "skill-management-reliability");
        assert_eq!(plan_migration, "plan-execution-foundation");
        assert_eq!(code_index_migration, "workspace-code-index-foundation");
        assert_eq!(code_index_mode_migration, "workspace-code-index-mode");
        assert_eq!(
            automatic_code_index_mode_migration,
            "automatic-code-index-mode"
        );
        assert_eq!(
            reconciliation_migration,
            "plan-and-code-index-reconciliation"
        );
        assert_eq!(stable_participant_migration, "stable-session-participants");
        assert_eq!(plan_agent_loop_migration, "onepiece-plan-agent-loop");
        assert_eq!(managed_im_binding_migration, "managed-im-session-bindings");
        assert_eq!(
            context_quality_migration,
            "onepiece-context-quality-history"
        );
        assert_eq!(context_quality_schema_objects, 5);
        assert_eq!(lsp_migration, "lsp-code-intelligence-foundation");
        assert_eq!(effective_skill_migration, "effective-skill-runtime");
        assert_eq!(evidence_migration, "skill-evolution-evidence-foundation");
        assert_eq!(native_tool_migration, "onepiece-native-tool-persistence");
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
        assert_eq!(migration_count, 70);
    }

    #[test]
    fn pooled_connections_serve_concurrent_readers_and_writers() {
        use std::thread;

        let directory = TempDirectory::new("native-database-concurrent");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");

        // More workers than MAX_POOL_SIZE, so this also exercises checkout back-pressure:
        // excess threads wait for a returned connection instead of opening unbounded handles.
        let workers = (MAX_POOL_SIZE as usize) + 4;
        let handles: Vec<_> = (0..workers)
            .map(|index| {
                let database = database.clone();
                thread::spawn(move || {
                    let connection = database.connection().expect("checkout");
                    connection
                        .execute(
                            "INSERT OR REPLACE INTO settings (key, value, created_at, updated_at) \
                             VALUES (?1, ?2, ?3, ?3)",
                            params![format!("concurrent-{index}"), "ok", "1700000000"],
                        )
                        .expect("concurrent insert");
                    // A read on the same connection under WAL must not be locked out by writers.
                    connection
                        .query_row("SELECT COUNT(*) FROM agents", [], |row| {
                            row.get::<_, i64>(0)
                        })
                        .expect("concurrent read");
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("worker thread");
        }

        let connection = database.connection().expect("final checkout");
        let written: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key LIKE 'concurrent-%'",
                [],
                |row| row.get(0),
            )
            .expect("written count");
        let agents: i64 = connection
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
            .expect("agent count");

        assert_eq!(written, workers as i64, "every concurrent writer committed");
        assert_eq!(
            agents, 6,
            "registry seeding ran exactly once, not per connection"
        );
    }

    #[test]
    fn every_pooled_connection_uses_recovery_durability_settings() {
        let directory = TempDirectory::new("native-database-durability");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let connections: Vec<_> = (0..MAX_POOL_SIZE)
            .map(|_| database.connection().expect("pooled connection"))
            .collect();

        for connection in connections {
            let journal_mode: String = connection
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .expect("journal mode");
            let synchronous: i64 = connection
                .query_row("PRAGMA synchronous", [], |row| row.get(0))
                .expect("synchronous");
            let foreign_keys: i64 = connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .expect("foreign keys");
            let busy_timeout: i64 = connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .expect("busy timeout");

            assert_eq!(journal_mode, "wal");
            assert_eq!(synchronous, SQLITE_SYNCHRONOUS_FULL);
            assert_eq!(foreign_keys, 1);
            assert_eq!(busy_timeout, BUSY_TIMEOUT.as_millis() as i64);
        }
    }
}
