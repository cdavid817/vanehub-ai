use crate::platform::database::{table_has_column, DatabaseError};
use rusqlite::Connection;

pub(crate) fn apply_remote_terminal_schema(connection: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(connection, "ssh_connections", "revision")? {
        connection.execute(
            "ALTER TABLE ssh_connections ADD COLUMN revision INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !table_has_column(connection, "sessions", "remote_ssh_connection_id")? {
        connection.execute(
            "ALTER TABLE sessions ADD COLUMN remote_ssh_connection_id TEXT",
            [],
        )?;
    }
    if !table_has_column(connection, "sessions", "remote_ssh_connection_revision")? {
        connection.execute(
            "ALTER TABLE sessions ADD COLUMN remote_ssh_connection_revision INTEGER",
            [],
        )?;
    }

    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS ssh_host_trust (
            connection_id TEXT PRIMARY KEY,
            host TEXT NOT NULL,
            port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
            algorithm TEXT NOT NULL,
            fingerprint TEXT NOT NULL,
            confirmed_at TEXT NOT NULL,
            FOREIGN KEY (connection_id) REFERENCES ssh_connections(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS terminal_command_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            scope TEXT NOT NULL CHECK (scope IN ('global', 'connection', 'workspace')),
            connection_id TEXT,
            workspace_uri TEXT,
            working_directory TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (connection_id) REFERENCES ssh_connections(id) ON DELETE CASCADE,
            CHECK (
                (scope = 'global' AND connection_id IS NULL AND workspace_uri IS NULL)
                OR (scope = 'connection' AND connection_id IS NOT NULL AND workspace_uri IS NULL)
                OR (scope = 'workspace' AND connection_id IS NULL AND workspace_uri IS NOT NULL)
            )
        );

        CREATE TABLE IF NOT EXISTS terminal_command_runs (
            id TEXT PRIMARY KEY,
            template_id TEXT,
            session_id TEXT NOT NULL,
            connection_id TEXT,
            command_snapshot TEXT NOT NULL,
            working_directory TEXT,
            status TEXT NOT NULL CHECK (
                status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')
            ),
            exit_code INTEGER,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            FOREIGN KEY (template_id) REFERENCES terminal_command_templates(id)
                ON DELETE SET NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (connection_id) REFERENCES ssh_connections(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS terminal_output_chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stream_id TEXT NOT NULL,
            sequence INTEGER NOT NULL CHECK (sequence >= 0),
            session_id TEXT NOT NULL,
            connection_id TEXT,
            terminal_id TEXT,
            run_id TEXT,
            source TEXT NOT NULL CHECK (source IN ('pty', 'quick-command', 'gap')),
            content TEXT NOT NULL,
            content_bytes INTEGER NOT NULL CHECK (content_bytes >= 0),
            captured_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (connection_id) REFERENCES ssh_connections(id) ON DELETE SET NULL,
            FOREIGN KEY (run_id) REFERENCES terminal_command_runs(id) ON DELETE SET NULL,
            UNIQUE (stream_id, sequence)
        );

        CREATE TABLE IF NOT EXISTS terminal_capture_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
            retention_days INTEGER NOT NULL DEFAULT 30 CHECK (retention_days > 0),
            capacity_bytes INTEGER NOT NULL DEFAULT 536870912 CHECK (capacity_bytes > 0),
            updated_at TEXT NOT NULL
        );

        INSERT OR IGNORE INTO terminal_capture_settings
            (id, enabled, retention_days, capacity_bytes, updated_at)
        VALUES
            (1, 1, 30, 536870912, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

        CREATE INDEX IF NOT EXISTS idx_sessions_remote_ssh_connection
            ON sessions(remote_ssh_connection_id);
        CREATE INDEX IF NOT EXISTS idx_terminal_command_templates_scope
            ON terminal_command_templates(scope, connection_id, workspace_uri, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_terminal_command_runs_session_started
            ON terminal_command_runs(session_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_terminal_output_chunks_session_time
            ON terminal_output_chunks(session_id, captured_at DESC);
        CREATE INDEX IF NOT EXISTS idx_terminal_output_chunks_connection_time
            ON terminal_output_chunks(connection_id, captured_at DESC);
        CREATE INDEX IF NOT EXISTS idx_terminal_output_chunks_run_sequence
            ON terminal_output_chunks(run_id, sequence);

        CREATE VIRTUAL TABLE IF NOT EXISTS terminal_output_fts USING fts5(
            content,
            content='terminal_output_chunks',
            content_rowid='id',
            tokenize='trigram'
        );

        CREATE TRIGGER IF NOT EXISTS terminal_output_chunks_fts_insert
        AFTER INSERT ON terminal_output_chunks BEGIN
            INSERT INTO terminal_output_fts(rowid, content) VALUES (new.id, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS terminal_output_chunks_fts_delete
        AFTER DELETE ON terminal_output_chunks BEGIN
            INSERT INTO terminal_output_fts(terminal_output_fts, rowid, content)
            VALUES ('delete', old.id, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS terminal_output_chunks_fts_update
        AFTER UPDATE OF content ON terminal_output_chunks BEGIN
            INSERT INTO terminal_output_fts(terminal_output_fts, rowid, content)
            VALUES ('delete', old.id, old.content);
            INSERT INTO terminal_output_fts(rowid, content) VALUES (new.id, new.content);
        END;
        "#,
    )?;
    Ok(())
}
