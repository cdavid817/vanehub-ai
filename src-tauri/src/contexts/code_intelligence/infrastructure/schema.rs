use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS lsp_configuration (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
            updated_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
        );

        CREATE TABLE IF NOT EXISTS lsp_language_configurations (
            language_id TEXT PRIMARY KEY
                CHECK (language_id IN ('rust', 'typescript_javascript')),
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            executable_override TEXT,
            startup_arguments_json TEXT NOT NULL DEFAULT '[]',
            initialization_options_json TEXT NOT NULL DEFAULT '{}',
            revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
            updated_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
        );

        CREATE TABLE IF NOT EXISTS lsp_workspace_trust (
            canonical_workspace_root TEXT PRIMARY KEY,
            trusted INTEGER NOT NULL DEFAULT 0 CHECK (trusted IN (0, 1)),
            revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
            created_at TEXT NOT NULL DEFAULT (strftime('%s', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
        );

        INSERT OR IGNORE INTO lsp_configuration (id, enabled) VALUES (1, 0);
        INSERT OR IGNORE INTO lsp_language_configurations (
            language_id, enabled, startup_arguments_json, initialization_options_json
        ) VALUES ('rust', 0, '[]', '{}');
        INSERT OR IGNORE INTO lsp_language_configurations (
            language_id, enabled, startup_arguments_json, initialization_options_json
        ) VALUES ('typescript_javascript', 0, '[\"--stdio\"]', '{}');",
    )?;
    Ok(())
}

/// Moves the supported-language set out of the storage layer.
///
/// `apply_schema` pinned it there twice: a `CHECK` constraint enumerating the two language ids,
/// and a NOT NULL startup-arguments column that only ever mirrored a compile-time constant.
/// SQLite cannot alter a `CHECK`, so the table is rebuilt rather than altered.
///
/// Existing startup arguments become NULL. They were never a user choice -- nothing could write
/// them but the constant -- so carrying them across would record every existing installation as
/// having explicitly overridden its arguments, freezing today's defaults for good.
pub(crate) fn apply_language_registry_schema(connection: &Connection) -> Result<(), DatabaseError> {
    if startup_arguments_are_nullable(connection)? {
        return Ok(());
    }
    connection.execute_batch(
        "CREATE TABLE lsp_language_configurations_registry (
            language_id TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            executable_override TEXT,
            startup_arguments_json TEXT,
            initialization_options_json TEXT NOT NULL DEFAULT '{}',
            revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
            updated_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
        );

        INSERT INTO lsp_language_configurations_registry (
            language_id, enabled, executable_override, startup_arguments_json,
            initialization_options_json, revision, updated_at
        )
        SELECT language_id, enabled, executable_override, NULL,
               initialization_options_json, revision, updated_at
        FROM lsp_language_configurations;

        DROP TABLE lsp_language_configurations;
        ALTER TABLE lsp_language_configurations_registry
            RENAME TO lsp_language_configurations;",
    )?;
    Ok(())
}

/// The rebuilt table is the only one whose startup-arguments column is nullable, so nullability
/// answers "has this already run" without a marker column or a second history table.
fn startup_arguments_are_nullable(connection: &Connection) -> Result<bool, DatabaseError> {
    let mut statement = connection.prepare("PRAGMA table_info(lsp_language_configurations)")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == "startup_arguments_json" {
            return Ok(row.get::<_, i64>(3)? == 0);
        }
    }
    Ok(false)
}
