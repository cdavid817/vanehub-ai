use crate::platform::database::{migrate, table_has_column};
use rusqlite::{Connection, OptionalExtension};

const PRE_REMOTE_TERMINAL_FIXTURE: &str =
    include_str!("../tests/fixtures/database/pre-remote-terminal.sql");

fn count(connection: &Connection, sql: &str) -> i64 {
    connection
        .query_row(sql, [], |row| row.get(0))
        .expect("count rows")
}

#[test]
fn pre_remote_terminal_fixture_preserves_records_and_adds_defaults() {
    let connection = Connection::open_in_memory().expect("in-memory sqlite");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    migrate(&connection).expect("initial migration");
    crate::contexts::agent_runtime::infrastructure::seed_registry(&connection)
        .expect("seed agents");
    connection
        .execute_batch(
            r#"
            INSERT INTO ssh_connections
                (id, name, host, port, user, default_path, auth_mode, key_path,
                 credential_ref, test_status, created_at, updated_at, revision)
            VALUES
                ('ssh-pre-terminal', 'Pre Terminal', 'dev.example.com', 2222, 'dev',
                 '/work/app', 'key', '/keys/dev', NULL, 'not-tested',
                 '2026-07-24', '2026-07-24', 7);
            INSERT INTO sessions
                (id, title, agent_id, interaction_mode, lifecycle_state,
                 remote_workspace_host, remote_workspace_port, remote_workspace_user,
                 remote_workspace_path, remote_workspace_display_name, remote_workspace_uri,
                 remote_ssh_connection_id, remote_ssh_connection_revision,
                 pinned, archived, created_at, updated_at)
            VALUES
                ('session-pre-terminal', 'Preserved remote terminal', 'codex-cli', 'cli', 'idle',
                 'dev.example.com', 2222, 'dev', '/work/app', 'Work',
                 'ssh://dev@dev.example.com:2222/work/app',
                 'ssh-pre-terminal', 7, 0, 0, '2026-07-24', '2026-07-24');
            "#,
        )
        .expect("seed pre-change records");
    connection
        .execute_batch(PRE_REMOTE_TERMINAL_FIXTURE)
        .expect("load pre-change fixture");

    migrate(&connection).expect("upgrade pre-change fixture");

    assert!(table_has_column(&connection, "ssh_connections", "revision").expect("revision column"));
    assert!(
        table_has_column(&connection, "sessions", "remote_ssh_connection_revision")
            .expect("session revision column")
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT name || ':' || revision FROM ssh_connections WHERE id = ?1",
                ["ssh-pre-terminal"],
                |row| row.get::<_, String>(0),
            )
            .expect("preserved SSH profile"),
        "Pre Terminal:1"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT title FROM sessions WHERE id = ?1",
                ["session-pre-terminal"],
                |row| row.get::<_, String>(0),
            )
            .expect("preserved session"),
        "Preserved remote terminal"
    );
    let binding: Option<String> = connection
        .query_row(
            "SELECT remote_ssh_connection_id FROM sessions WHERE id = ?1",
            ["session-pre-terminal"],
            |row| row.get(0),
        )
        .expect("default session binding");
    assert_eq!(binding, None);
    assert_eq!(
        connection
            .query_row(
                "SELECT enabled || ':' || retention_days || ':' || capacity_bytes
                 FROM terminal_capture_settings WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("capture defaults"),
        "1:30:536870912"
    );
}

#[test]
fn terminal_output_fts_and_delete_rules_cover_multilingual_paths() {
    let connection = Connection::open_in_memory().expect("in-memory sqlite");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    migrate(&connection).expect("migrate");
    crate::contexts::agent_runtime::infrastructure::seed_registry(&connection)
        .expect("seed agents");
    connection
        .execute_batch(
            r#"
            INSERT INTO ssh_connections
                (id, name, host, port, user, default_path, auth_mode, test_status,
                 created_at, updated_at)
            VALUES
                ('ssh-fts', 'FTS host', 'dev.example.com', 22, 'dev', '/work/app',
                 'key', 'not-tested', '2026-07-24', '2026-07-24');
            INSERT INTO ssh_host_trust
                (connection_id, host, port, algorithm, fingerprint, confirmed_at)
            VALUES
                ('ssh-fts', 'dev.example.com', 22, 'ssh-ed25519', 'SHA256:fixture',
                 '2026-07-24');
            INSERT INTO sessions
                (id, title, agent_id, interaction_mode, lifecycle_state,
                 remote_ssh_connection_id, remote_ssh_connection_revision,
                 pinned, archived, created_at, updated_at)
            VALUES
                ('session-fts', 'Search session', 'codex-cli', 'cli', 'idle',
                 'ssh-fts', 1, 0, 0, '2026-07-24', '2026-07-24');
            INSERT INTO terminal_command_templates
                (id, name, command, scope, connection_id, tags, created_at, updated_at)
            VALUES
                ('template-fts', 'Build', 'npm run build', 'connection', 'ssh-fts',
                 '["build"]', '2026-07-24', '2026-07-24');
            INSERT INTO terminal_command_runs
                (id, template_id, session_id, connection_id, command_snapshot, status,
                 started_at, finished_at)
            VALUES
                ('run-fts', 'template-fts', 'session-fts', 'ssh-fts', 'npm run build',
                 'succeeded', '2026-07-24T10:00:00Z', '2026-07-24T10:00:01Z');
            INSERT INTO terminal_output_chunks
                (stream_id, sequence, session_id, connection_id, terminal_id, run_id,
                 source, content, content_bytes, captured_at)
            VALUES
                ('stream-fts', 0, 'session-fts', 'ssh-fts', 'terminal-1', 'run-fts',
                 'quick-command', '项目构建完成 /work/app/src/main.rs', 42,
                 '2026-07-24T10:00:01Z');
            "#,
        )
        .expect("seed terminal records");

    let cjk_matches: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM terminal_output_fts
             WHERE terminal_output_fts MATCH ?1",
            ["项目构"],
            |row| row.get(0),
        )
        .expect("search CJK output");
    let path_matches: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM terminal_output_fts
             WHERE terminal_output_fts MATCH ?1",
            [r#""work/app/src""#],
            |row| row.get(0),
        )
        .expect("search path output");
    assert_eq!(cjk_matches, 1);
    assert_eq!(path_matches, 1);

    connection
        .execute("DELETE FROM ssh_connections WHERE id = ?1", ["ssh-fts"])
        .expect("delete SSH profile");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM ssh_host_trust"), 0);
    assert_eq!(
        count(
            &connection,
            "SELECT COUNT(*) FROM terminal_command_templates"
        ),
        0
    );
    let detached_run: (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT template_id, connection_id FROM terminal_command_runs WHERE id = ?1",
            ["run-fts"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("detached run");
    assert_eq!(detached_run, (None, None));
    assert_eq!(
        count(&connection, "SELECT COUNT(*) FROM terminal_output_chunks"),
        1
    );

    connection
        .execute(
            "DELETE FROM terminal_output_chunks WHERE stream_id = ?1",
            ["stream-fts"],
        )
        .expect("purge output");
    assert_eq!(
        count(&connection, "SELECT COUNT(*) FROM terminal_command_runs"),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT 1 FROM terminal_output_fts WHERE terminal_output_fts MATCH ?1",
                ["项目构"],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .expect("search purged output"),
        None
    );

    connection
        .execute("DELETE FROM sessions WHERE id = ?1", ["session-fts"])
        .expect("delete session");
    assert_eq!(
        count(&connection, "SELECT COUNT(*) FROM terminal_command_runs"),
        0
    );
    assert_eq!(
        count(&connection, "SELECT COUNT(*) FROM pragma_foreign_key_check"),
        0,
        "all delete paths must preserve foreign-key integrity"
    );
}
