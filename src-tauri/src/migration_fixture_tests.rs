use crate::platform::database::{migrate, table_has_column};
use rusqlite::{params, Connection};

const EMPTY_FIXTURE: &str = include_str!("../tests/fixtures/database/empty.sql");
const LEGACY_V1_FIXTURE: &str = include_str!("../tests/fixtures/database/legacy-v1.sql");
const CURRENT_V20_DATA_FIXTURE: &str =
    include_str!("../tests/fixtures/database/current-v20-data.sql");

/// Contiguous through 64. Migration 53 reconciles Plan execution and workspace code indexing,
/// migrations 54-58 add Loop, recovery, and LSP foundations, migration 59 introduces stable
/// shared-session participant identity, migration 60 adds effective Skill reconciliation, and
/// migration 61 resets legacy session execution preferences and governed CLI security selections;
/// migrations 62-63 complete the OnePiece Plan-Agent loop and session association, and migration
/// 64 introduces invocation-grained Token accounting.
fn expected_versions() -> Vec<i64> {
    (1..=64).collect()
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
fn migration_49_collision_histories_converge_at_version_53() {
    for migration_49_name in [
        "plan-execution-foundation",
        "workspace-code-index-foundation",
    ] {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )
        .expect("create migration history");
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (49, ?1)",
            params![migration_49_name],
        )
        .expect("seed migration 49 history");

        migrate(&conn).expect("reconcile migration collision");

        assert_eq!(applied_versions(&conn), expected_versions());
        for table in ["plans", "code_index_workspaces"] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .expect("query reconciled table");
            assert_eq!(
                exists, 1,
                "{table} missing for version 49 history {migration_49_name}"
            );
        }
    }
}

#[test]
fn code_index_worktree_migration_history_gains_plan_schema_at_version_53() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    migrate(&conn).expect("initial migration");
    conn.execute_batch(
        "DROP TABLE plan_generation_failures;
         DROP TABLE plan_control_requests;
         DROP TABLE plan_verification_evidence;
         DROP TABLE plan_subtask_attempts;
         DROP TABLE plan_subtask_runs;
         DROP TABLE plan_runs;
         DROP TABLE plan_subtask_dependencies;
         DROP TABLE plan_subtasks;
         DROP TABLE plan_versions;
         DROP TABLE plans;
         DELETE FROM schema_migrations WHERE version BETWEEN 49 AND 53;",
    )
    .expect("remove canonical migration tail");
    for (version, name) in [
        (49, "workspace-code-index-foundation"),
        (50, "workspace-code-index-mode"),
        (51, "automatic-code-index-mode"),
    ] {
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![version, name],
        )
        .expect("seed worktree migration history");
    }

    migrate(&conn).expect("reconcile worktree migration history");

    assert_eq!(applied_versions(&conn), expected_versions());
    let plans_exist: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'plans'",
            [],
            |row| row.get(0),
        )
        .expect("query plans table");
    assert_eq!(plans_exist, 1);
}

#[test]
fn migration_repairs_a_conflicting_54_without_the_message_speaker_column() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    migrate(&conn).expect("initial migration");
    conn.execute_batch(
        "INSERT INTO agents (id, display_name, provider, launch_kind)
         VALUES ('repair-agent', 'Repair Agent', 'test', 'api');
         INSERT INTO sessions
             (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)
         VALUES
             ('repair-session', 'Repair Session', 'repair-agent', 'api', 'idle',
              '2026-08-10', '2026-08-10');",
    )
    .expect("repair fixture ownership");
    conn.execute_batch(
        "DROP INDEX idx_messages_speaker_seat;
         ALTER TABLE messages DROP COLUMN speaker_seat_id;
         UPDATE schema_migrations
         SET name = 'session-recovery-evidence-foundation'
         WHERE version = 54;",
    )
    .expect("simulate early version 54 schema");

    assert!(!table_has_column(&conn, "messages", "speaker_seat_id").expect("missing column"));
    assert_eq!(
        conn.query_row(
            "SELECT name FROM schema_migrations WHERE version = 54",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("recorded version 54"),
        "session-recovery-evidence-foundation"
    );

    migrate(&conn).expect("repair migration");

    assert!(table_has_column(&conn, "messages", "speaker_seat_id").expect("repaired column"));
    conn.execute(
        "INSERT INTO messages
         (id, session_id, role, content, created_at, updated_at, speaker_seat_id)
         VALUES ('repair-message', 'repair-session', 'assistant', '', '2026-08-10',
                 '2026-08-10', 'repair-seat')",
        [],
    )
    .expect("write a message with a stable speaker");
    assert_eq!(
        conn.query_row(
            "SELECT speaker_seat_id FROM messages WHERE id = 'repair-message'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("persisted speaker"),
        "repair-seat"
    );
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
    assert!(table_has_column(&conn, "messages", "speaker_seat_id")
        .expect("stable message speaker column"));
    assert!(!table_has_column(&conn, "usage_records", "message_id").expect("retired usage table"));
    assert!(
        table_has_column(&conn, "model_invocations", "purpose").expect("model invocation ledger")
    );
    assert!(
        table_has_column(&conn, "token_usage_observations", "normalization_version")
            .expect("usage observation ledger")
    );
    assert!(
        table_has_column(&conn, "usage_ingestion_cursors", "epoch").expect("usage cursor table")
    );
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
    assert!(
        table_has_column(&conn, "operation_recovery_evidence", "execution_run_id")
            .expect("operation recovery evidence table")
    );
}

#[test]
fn token_accounting_migration_does_not_import_pre_release_usage() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    migrate(&conn).expect("initial migration");
    conn.execute_batch(
        r#"
        INSERT INTO agents (id, display_name, provider, launch_kind)
        VALUES ('legacy-agent', 'Legacy Agent', 'test', 'cli');
        INSERT INTO sessions
            (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)
        VALUES
            ('legacy-session', 'Legacy Session', 'legacy-agent', 'chat', 'idle',
             '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z');
        INSERT INTO messages
            (id, session_id, role, content, status, created_at, updated_at)
        VALUES
            ('legacy-message', 'legacy-session', 'assistant', '', 'completed',
             '2026-08-12T00:00:01Z', '2026-08-12T00:00:01Z');
        CREATE TABLE usage_records (
            message_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            provider_id TEXT,
            model_id TEXT,
            input_count INTEGER NOT NULL,
            output_count INTEGER NOT NULL,
            cache_read_count INTEGER NOT NULL,
            cache_creation_count INTEGER NOT NULL,
            accounting_kind TEXT NOT NULL,
            unit TEXT NOT NULL,
            source TEXT NOT NULL,
            occurred_at TEXT NOT NULL
        );
        INSERT INTO usage_records
            (message_id, session_id, agent_id, provider_id, model_id,
             input_count, output_count, cache_read_count, cache_creation_count,
             accounting_kind, unit, source, occurred_at)
        VALUES
            ('legacy-message', 'legacy-session', 'legacy-agent', 'legacy-provider', 'legacy-model',
             10, 4, 3, 2, 'reported', 'tokens', 'cli-result',
             '2026-08-12T00:00:02Z');
        DROP TABLE token_usage_observations;
        DROP TABLE usage_ingestion_cursors;
        DROP TABLE model_invocations;
        DELETE FROM schema_migrations WHERE version = 64;
        "#,
    )
    .expect("prepare pre-ledger database");

    migrate(&conn).expect("apply token accounting migration");
    migrate(&conn).expect("repeat migration");

    let invocation_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM model_invocations", [], |row| {
            row.get(0)
        })
        .expect("invocation count");
    assert_eq!(invocation_count, 0);
    let observation_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM token_usage_observations", [], |row| {
            row.get(0)
        })
        .expect("observation count");
    assert_eq!(observation_count, 0);
    assert!(!table_has_column(&conn, "usage_records", "message_id").expect("retired usage table"));
    let preserved_messages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE id = 'legacy-message'",
            [],
            |row| row.get(0),
        )
        .expect("preserved message count");
    assert_eq!(preserved_messages, 1);
}

#[test]
fn current_schema_adds_disabled_lsp_configuration_and_empty_workspace_trust() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    migrate(&conn).expect("migrate current schema");

    let master_enabled: i64 = conn
        .query_row(
            "SELECT enabled FROM lsp_configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("LSP master configuration");
    let languages = conn
        .prepare(
            "SELECT language_id, enabled, startup_arguments_json, initialization_options_json \
             FROM lsp_language_configurations ORDER BY language_id",
        )
        .expect("prepare LSP language configuration")
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .expect("query LSP language configuration")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect LSP language configuration");
    let trusted_workspaces: i64 = conn
        .query_row("SELECT COUNT(*) FROM lsp_workspace_trust", [], |row| {
            row.get(0)
        })
        .expect("workspace trust count");
    let migration_name: String = conn
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 58",
            [],
            |row| row.get(0),
        )
        .expect("LSP foundation migration");

    assert_eq!(master_enabled, 0);
    assert_eq!(
        languages,
        vec![
            ("rust".into(), 0, "[]".into(), "{}".into()),
            (
                "typescript_javascript".into(),
                0,
                "[\"--stdio\"]".into(),
                "{}".into()
            ),
        ]
    );
    assert_eq!(trusted_workspaces, 0);
    assert_eq!(migration_name, "lsp-code-intelligence-foundation");
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
