use crate::platform::database::{migrate, table_has_column};
use rusqlite::Connection;

const EMPTY_FIXTURE: &str = include_str!("../tests/fixtures/database/empty.sql");
const LEGACY_V1_FIXTURE: &str = include_str!("../tests/fixtures/database/legacy-v1.sql");
const CURRENT_V20_DATA_FIXTURE: &str =
    include_str!("../tests/fixtures/database/current-v20-data.sql");

/// Contiguous again: the 42 this branch left open was taken by `agent-memory-shared-pool` on
/// main, and the later migrations remain dense through workspace code indexing at 49.
fn expected_versions() -> Vec<i64> {
    (1..=49).collect()
}

fn applied_versions(conn: &Connection) -> Vec<i64> {
    conn.prepare("SELECT version FROM schema_migrations ORDER BY version")
        .expect("prepare versions")
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("query versions")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect versions")
}

#[test]
fn empty_fixture_migrates_to_latest_schema() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(EMPTY_FIXTURE)
        .expect("load empty fixture");

    migrate(&conn).expect("migrate empty fixture");

    assert_eq!(applied_versions(&conn), expected_versions());
    assert!(
        table_has_column(&conn, "onepiece_provider_profiles", "active")
            .expect("OnePiece provider profile table")
    );
    assert!(table_has_column(&conn, "sessions", "remote_workspace_uri")
        .expect("remote workspace column"));
    assert!(table_has_column(&conn, "messages", "rich_blocks").expect("rich block column"));
    assert!(table_has_column(&conn, "usage_records", "message_id").expect("usage record table"));
    assert!(
        table_has_column(&conn, "scheduled_tasks", "next_run_at").expect("scheduled task table")
    );
    assert!(
        table_has_column(&conn, "ssh_connections", "credential_ref").expect("ssh connection table")
    );
    assert!(table_has_column(&conn, "execution_runs", "trace_id").expect("execution run table"));
    assert!(table_has_column(&conn, "execution_spans", "fidelity").expect("execution span table"));
    assert!(table_has_column(&conn, "loop_runs", "definition_snapshot").expect("Loop run table"));
    assert!(table_has_column(&conn, "sessions", "loop_role").expect("Loop role column"));
    assert!(
        table_has_column(&conn, "ssh_connections", "revision").expect("SSH connection revision")
    );
    assert!(
        table_has_column(&conn, "sessions", "remote_ssh_connection_id")
            .expect("remote SSH session binding")
    );
    assert!(
        table_has_column(&conn, "terminal_command_templates", "scope")
            .expect("command template table")
    );
    assert!(table_has_column(&conn, "terminal_output_fts", "content")
        .expect("terminal output FTS table"));
    assert!(
        table_has_column(&conn, "cli_config_profiles", "managed_keys_json")
            .expect("CLI configuration profile table")
    );
    assert!(
        table_has_column(&conn, "cli_config_applied_state", "live_fingerprint")
            .expect("CLI configuration applied-state table")
    );
    assert!(
        table_has_column(&conn, "cli_config_applied_state", "applied_payload_json")
            .expect("CLI configuration applied payload snapshot")
    );
    assert!(
        table_has_column(&conn, "cli_config_applied_state", "managed_keys_json")
            .expect("CLI configuration ownership snapshot")
    );
}

#[test]
fn legacy_v1_fixture_upgrades_without_losing_records() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(LEGACY_V1_FIXTURE)
        .expect("load legacy fixture");

    migrate(&conn).expect("migrate legacy fixture");

    assert_eq!(applied_versions(&conn), expected_versions());
    assert!(
        table_has_column(&conn, "agents", "managed_sdk_dependency_id").expect("managed SDK column")
    );
    assert!(
        table_has_column(&conn, "workflow_state", "active_session_id")
            .expect("active session column")
    );
    assert_eq!(
        conn.query_row(
            "SELECT display_name FROM agents WHERE id = 'legacy-agent'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("legacy agent"),
        "Legacy Agent"
    );
    assert_eq!(
        conn.query_row(
            "SELECT command FROM mcp_servers WHERE name = 'legacy-mcp'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("legacy MCP server"),
        "legacy-mcp"
    );
}

#[test]
fn current_v20_fixture_is_idempotent_and_readable() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    migrate(&conn).expect("initial current migration");
    crate::contexts::agent_runtime::infrastructure::seed_registry(&conn).expect("seed agents");
    conn.execute_batch(CURRENT_V20_DATA_FIXTURE)
        .expect("load current data fixture");

    migrate(&conn).expect("repeat current migration");

    assert_eq!(applied_versions(&conn), expected_versions());
    assert!(
        table_has_column(&conn, "sdk_operation_logs", "operation_id")
            .expect("SDK operation log column")
    );
    assert_eq!(
        conn.query_row(
            "SELECT content FROM messages WHERE id = 'fixture-message'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("fixture message"),
        "Persisted fixture"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'application_language'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("fixture setting"),
        "en"
    );
}

#[test]
fn onepiece_legacy_runtime_configuration_migrates_to_an_active_profile() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    migrate(&conn).expect("initial migration");
    crate::contexts::agent_runtime::infrastructure::seed_registry(&conn).expect("seed agents");
    conn.execute(
        "UPDATE agents SET provider = 'Legacy Provider', model_id = 'legacy-model', interface_format = 'openai-compatible', base_url = 'https://legacy.example.test/v1' WHERE id = 'onepiece'",
        [],
    )
    .expect("configure legacy OnePiece row");
    conn.execute_batch(
        "DROP TABLE onepiece_provider_profiles;
         DELETE FROM schema_migrations WHERE version IN (39, 40, 41);",
    )
    .expect("simulate pre-profile schema");

    migrate(&conn).expect("migrate legacy OnePiece configuration");

    let migrated = conn
        .query_row(
            "SELECT id, name, source_preset_id, source_provider_id, source_endpoint_type, source_preset_version, provider, model_id, interface_format, base_url, active FROM onepiece_provider_profiles WHERE agent_id = 'onepiece'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<u32>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, bool>(10)?,
                ))
            },
        )
        .expect("migrated OnePiece profile");
    assert_eq!(migrated.0, "legacy-default");
    assert_eq!(migrated.1, "Legacy Provider");
    assert_eq!(migrated.2, None);
    assert_eq!(migrated.3, None);
    assert_eq!(migrated.4, None);
    assert_eq!(migrated.5, None);
    assert_eq!(migrated.6, "Legacy Provider");
    assert_eq!(migrated.7, "legacy-model");
    assert_eq!(migrated.8, "openai-compatible");
    assert_eq!(
        migrated.9.as_deref(),
        Some("https://legacy.example.test/v1")
    );
    assert!(migrated.10);
}

#[test]
fn onepiece_endpoint_migration_separates_provider_and_protocol_identity() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(
        "CREATE TABLE agents (
            id TEXT PRIMARY KEY,
            launch_kind TEXT NOT NULL,
            provider TEXT NOT NULL,
            model_id TEXT,
            interface_format TEXT,
            base_url TEXT
         );
         INSERT INTO agents (id, launch_kind, provider) VALUES ('onepiece', 'api', 'VaneHub');",
    )
    .expect("minimal agent schema");
    crate::contexts::agent_runtime::infrastructure::apply_onepiece_provider_profiles_schema(&conn)
        .expect("profile schema");
    crate::contexts::agent_runtime::infrastructure::apply_onepiece_provider_catalog_schema(&conn)
        .expect("catalog schema");
    conn.execute_batch(
        "INSERT INTO onepiece_provider_profiles
            (id, name, source_preset_id, source_preset_version, provider, model_id, interface_format, base_url, active)
         VALUES
            ('anthropic-endpoint', 'DeepSeek Anthropic', 'deepseek::anthropic-messages', 3, 'DeepSeek', 'deepseek-chat', 'anthropic', 'https://api.deepseek.com/anthropic', 0),
            ('default-endpoint', 'OpenRouter', 'openrouter', 3, 'OpenRouter', 'openai/gpt', 'openai-compatible', 'https://openrouter.ai/api/v1', 0);",
    )
    .expect("legacy catalog profiles");

    crate::contexts::agent_runtime::infrastructure::apply_onepiece_provider_endpoint_schema(&conn)
        .expect("endpoint schema");

    let explicit = conn
        .query_row(
            "SELECT source_provider_id, source_endpoint_type FROM onepiece_provider_profiles WHERE id = 'anthropic-endpoint'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("explicit endpoint identity");
    assert_eq!(
        explicit,
        ("deepseek".to_string(), "anthropic-messages".to_string())
    );
    let defaulted = conn
        .query_row(
            "SELECT source_provider_id, source_endpoint_type FROM onepiece_provider_profiles WHERE id = 'default-endpoint'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("default endpoint identity");
    assert_eq!(
        defaulted,
        (
            "openrouter".to_string(),
            "openai-chat-completions".to_string()
        )
    );
}

#[test]
fn pre_ssh_connection_schema_gains_remote_ports_without_losing_records() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    migrate(&conn).expect("initial migration");
    crate::contexts::agent_runtime::infrastructure::seed_registry(&conn).expect("seed agents");
    conn.execute_batch(
        r#"
        INSERT INTO known_remote_workspaces
            (uri, host, port, user, path, display_name, last_opened_at)
        VALUES
            ('ssh://dev@host/work', 'host', 2222, 'dev', '/work', 'Work', '2026-01-01');
        INSERT INTO sessions
            (id, title, agent_id, interaction_mode, lifecycle_state,
             remote_workspace_host, remote_workspace_port, remote_workspace_user,
             remote_workspace_path, remote_workspace_display_name, remote_workspace_uri,
             pinned, archived, created_at, updated_at)
        VALUES
            ('remote-fixture', 'Remote', 'codex-cli', 'cli', 'idle',
             'host', 2222, 'dev', '/work', 'Work', 'ssh://dev@host/work',
             0, 0, '2026-01-01', '2026-01-01');
        DELETE FROM schema_migrations WHERE version = 24;
        DROP INDEX idx_ssh_connections_updated;
        DROP TABLE ssh_connections;
        ALTER TABLE known_remote_workspaces DROP COLUMN port;
        ALTER TABLE sessions DROP COLUMN remote_workspace_port;
        "#,
    )
    .expect("simulate version 23 schema");

    assert!(!table_has_column(&conn, "known_remote_workspaces", "port").expect("history port"));
    assert!(!table_has_column(&conn, "sessions", "remote_workspace_port").expect("session port"));

    migrate(&conn).expect("upgrade version 23 schema");

    assert!(table_has_column(&conn, "known_remote_workspaces", "port").expect("history port"));
    assert!(table_has_column(&conn, "sessions", "remote_workspace_port").expect("session port"));
    assert_eq!(
        conn.query_row(
            "SELECT host || ':' || port FROM known_remote_workspaces WHERE uri = 'ssh://dev@host/work'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("preserved remote workspace"),
        "host:22"
    );
    assert_eq!(
        conn.query_row(
            "SELECT remote_workspace_host FROM sessions WHERE id = 'remote-fixture'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("preserved session"),
        "host"
    );
}
