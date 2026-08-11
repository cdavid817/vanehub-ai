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
