use super::*;
use crate::test_support::TempDirectory;
use rusqlite::ToSql;

fn recovery_performance_fixture(connection: &Connection) {
    connection
        .execute_batch(
            r#"
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                lifecycle_state TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO sessions (id, lifecycle_state) VALUES ('session-1', 'running');
            "#,
        )
        .expect("session performance fixture");
    apply_chat_messages_migration(connection).expect("message schema");
    apply_session_message_search_migration(connection).expect("legacy search schema");
}

fn search_count(connection: &Connection, query: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM session_message_fts WHERE session_message_fts MATCH ?1",
            [query],
            |row| row.get(0),
        )
        .expect("message search count")
}

fn query_plan(connection: &Connection, query: &str, params: &[&dyn ToSql]) -> Vec<String> {
    connection
        .prepare(&format!("EXPLAIN QUERY PLAN {query}"))
        .expect("prepare query plan")
        .query_map(params, |row| row.get::<_, String>(3))
        .expect("query plan")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect query plan")
}

fn legacy_session_recovery_fixture(connection: &Connection) {
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                lifecycle_state TEXT NOT NULL,
                runtime_session_id TEXT,
                loop_run_id TEXT,
                loop_iteration_id TEXT,
                loop_role TEXT
            );

            CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                status TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_use TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE plan_subtask_attempts (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                status TEXT NOT NULL
            );

            INSERT INTO sessions (
                id, lifecycle_state, runtime_session_id,
                loop_run_id, loop_iteration_id, loop_role
            ) VALUES
                ('clean-legacy', 'idle', NULL, NULL, NULL, NULL),
                ('orphan-active', 'running', 'provider-resume-1', NULL, NULL, NULL),
                ('plan-owned', 'starting', NULL, NULL, NULL, NULL),
                ('loop-owned', 'running', NULL, 'loop-run-1', 'iteration-1', 'worker');

            INSERT INTO messages (
                id, session_id, role, status, content, tool_use, created_at
            ) VALUES
                ('message-b', 'clean-legacy', 'assistant', 'completed', 'second', NULL, '100'),
                ('message-a', 'clean-legacy', 'user', 'completed', 'first', NULL, '100'),
                ('message-c', 'clean-legacy', 'assistant', 'completed', 'third', NULL, '101'),
                (
                    'message-tool', 'orphan-active', 'assistant', 'streaming', 'partial',
                    '[{"id":"tool-1","status":"running"}]', '200'
                ),
                ('message-plan', 'plan-owned', 'assistant', 'streaming', 'plan', NULL, '300'),
                ('message-loop', 'loop-owned', 'assistant', 'streaming', 'loop', NULL, '400');

            INSERT INTO plan_subtask_attempts (id, session_id, status)
            VALUES ('attempt-1', 'plan-owned', 'running');
            "#,
        )
        .expect("legacy recovery fixture");
}

fn mcp_migration_fixture(connection: &Connection) {
    connection
        .execute_batch(
            r#"
            CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
            );
            CREATE TABLE mcp_servers (
                name TEXT PRIMARY KEY,
                transport_type TEXT NOT NULL
            );
            "#,
        )
        .expect("MCP migration fixture");
}

fn insert_mcp_server(connection: &Connection, name: &str, transport_type: &str) {
    connection
        .execute(
            "INSERT INTO mcp_servers (name, transport_type) VALUES (?1, ?2)",
            params![name, transport_type],
        )
        .expect("MCP server fixture");
}

fn run_mcp_transport_migration(connection: &Connection) -> Result<(), DatabaseError> {
    apply_transactional_migration(
        connection,
        36,
        "mcp-truthful-url-transports",
        apply_mcp_truthful_url_transport_migration,
    )
}

fn transport_type(connection: &Connection, name: &str) -> String {
    connection
        .query_row(
            "SELECT transport_type FROM mcp_servers WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("persisted transport type")
}

#[test]
fn session_message_search_migration_backfills_existing_messages() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    connection
        .execute_batch(
            "CREATE TABLE sessions (id TEXT PRIMARY KEY); \
             INSERT INTO sessions (id) VALUES ('session-before-fts');",
        )
        .expect("session fixture");
    apply_chat_messages_migration(&connection).expect("message schema");
    connection
        .execute(
            "INSERT INTO messages \
             (id, session_id, role, content, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                "message-before-fts",
                "session-before-fts",
                "user",
                "content persisted before migration",
                "2026-08-02T00:00:00Z"
            ],
        )
        .expect("pre-migration message");

    apply_session_message_search_migration(&connection).expect("search migration");

    let matches: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM session_message_fts \
             WHERE session_message_fts MATCH ?1",
            ["\"persisted before\""],
            |row| row.get(0),
        )
        .expect("backfilled FTS count");
    assert_eq!(matches, 1);
}

#[test]
fn recovery_performance_migration_indexes_streamed_content_only_at_terminal_state() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    recovery_performance_fixture(&connection);
    connection
        .execute_batch(
            r#"
            INSERT INTO messages (
                id, session_id, role, status, content, created_at, updated_at
            ) VALUES
                ('streaming-message', 'session-1', 'assistant', 'streaming',
                 'partial searchable', '1', '1'),
                ('completed-message', 'session-1', 'assistant', 'completed',
                 'completed searchable', '2', '2');
            "#,
        )
        .expect("legacy indexed messages");
    assert_eq!(search_count(&connection, "searchable"), 2);

    apply_session_recovery_foundation_migration(&connection).expect("recovery schema");
    apply_session_recovery_performance_migration(&connection).expect("recovery performance schema");

    assert_eq!(search_count(&connection, "partial"), 0);
    assert_eq!(search_count(&connection, "completed"), 1);
    connection
        .execute(
            "UPDATE messages SET content = 'updated streamed content' WHERE id = ?1",
            ["streaming-message"],
        )
        .expect("persist streaming content");
    assert_eq!(search_count(&connection, "updated"), 0);

    connection
        .execute(
            "UPDATE messages SET status = 'failed' WHERE id = ?1",
            ["streaming-message"],
        )
        .expect("terminalize streamed message");
    assert_eq!(search_count(&connection, "updated"), 1);
    connection
        .execute("DELETE FROM messages WHERE id = ?1", ["streaming-message"])
        .expect("delete terminal message");
    assert_eq!(search_count(&connection, "updated"), 0);
}

#[test]
fn recovery_performance_indexes_match_the_production_hot_queries() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    recovery_performance_fixture(&connection);
    apply_session_recovery_foundation_migration(&connection).expect("recovery schema");
    apply_session_recovery_performance_migration(&connection).expect("recovery performance schema");

    let no_cursor: Option<String> = None;
    let candidate_limit = 100_i64;
    let candidate_plan = query_plan(
        &connection,
        "SELECT id FROM sessions
         WHERE archived = 0
           AND recovery_status NOT IN ('action_required', 'quarantined')
           AND (
             active_execution_run_id IS NOT NULL
             OR lifecycle_state IN ('starting', 'running')
             OR recovery_status = 'reconciling'
           )
           AND (?1 IS NULL OR id > ?1)
         ORDER BY id LIMIT ?2",
        &[&no_cursor, &candidate_limit],
    );
    assert!(candidate_plan
        .iter()
        .any(|detail| detail.contains("idx_sessions_pending_recovery_id")));

    let evidence_limit = 257_i64;
    let evidence_plan = query_plan(
        &connection,
        "SELECT id FROM messages INDEXED BY idx_messages_session_run_sequence
         WHERE session_id = ?1 AND execution_run_id = ?2
         ORDER BY session_sequence, id LIMIT ?3",
        &[&"session-1", &"run-1", &evidence_limit],
    );
    assert!(evidence_plan
        .iter()
        .any(|detail| detail.contains("idx_messages_session_run_sequence")));

    let conflict_plan = query_plan(
        &connection,
        "SELECT id FROM messages INDEXED BY idx_messages_unfinished_session_sequence
         WHERE session_id = ?1
           AND execution_run_id IS NOT NULL
           AND execution_run_id <> ?2
           AND status IN ('pending', 'streaming')
         ORDER BY session_sequence, id LIMIT 1",
        &[&"session-1", &"run-1"],
    );
    assert!(conflict_plan
        .iter()
        .any(|detail| detail.contains("idx_messages_unfinished_session_sequence")));
}

#[test]
fn skill_reliability_migration_upgrades_database_without_api_binding_table() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    migrate(&connection).expect("current schema");
    connection
        .execute(
            "INSERT INTO skills (id, scope, workspace_path, source, enabled, skill_dir, \
             skill_md_path, content_hash, metadata_json, created_at, updated_at) \
             VALUES (?1, 'global', '', 'user-created', 1, ?2, ?3, 'hash', '{}', ?4, ?4)",
            params![
                "preserved-skill",
                "/managed/preserved-skill",
                "/managed/preserved-skill/SKILL.md",
                "2026-08-02T00:00:00Z"
            ],
        )
        .expect("existing Skill record");
    connection
        .execute_batch(
            r#"
            DELETE FROM schema_migrations WHERE version = 37;
            DROP TABLE skill_api_agent_bindings;
            "#,
        )
        .expect("pre-migration-37 fixture");

    let migration_state: (i64, i64) = connection
        .query_row(
            "SELECT COUNT(*), MAX(version) FROM schema_migrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("fixture migration state");
    assert_eq!(migration_state, (90, 91));

    migrate(&connection).expect("upgrade migration");

    let api_binding_table: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'skill_api_agent_bindings'",
            [],
            |row| row.get(0),
        )
        .expect("API binding table");
    let reliability_migration: String = connection
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 37",
            [],
            |row| row.get(0),
        )
        .expect("reliability migration");
    let preserved_skill: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM skills WHERE id = 'preserved-skill'",
            [],
            |row| row.get(0),
        )
        .expect("preserved Skill record");

    assert_eq!(api_binding_table, 1);
    assert_eq!(reliability_migration, "skill-management-reliability");
    assert_eq!(preserved_skill, 1);
}

#[test]
fn mcp_transport_migration_upgrades_an_old_database_and_journals_the_row() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    mcp_migration_fixture(&connection);
    insert_mcp_server(&connection, "historical-url", "sse");

    run_mcp_transport_migration(&connection).expect("transport migration");

    let journal: (String, String, String) = connection
        .query_row(
            "SELECT server_name, previous_transport_type, migrated_transport_type \
             FROM mcp_transport_migration_journal WHERE migration_version = 36",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migration journal row");
    assert_eq!(
        transport_type(&connection, "historical-url"),
        "streamable_http"
    );
    assert_eq!(
        journal,
        (
            "historical-url".into(),
            "sse".into(),
            "streamable_http".into()
        )
    );
}

#[test]
fn mcp_transport_migration_is_idempotent_for_an_already_migrated_database() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    mcp_migration_fixture(&connection);
    insert_mcp_server(&connection, "modern-url", "streamable_http");

    run_mcp_transport_migration(&connection).expect("first migration");
    run_mcp_transport_migration(&connection).expect("idempotent reopen migration");

    let versions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 36",
            [],
            |row| row.get(0),
        )
        .expect("migration version count");
    let journal_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM mcp_transport_migration_journal",
            [],
            |row| row.get(0),
        )
        .expect("journal count");
    assert_eq!(versions, 1);
    assert_eq!(journal_rows, 0);
    assert_eq!(transport_type(&connection, "modern-url"), "streamable_http");
}

#[test]
fn mcp_transport_migration_only_changes_legacy_sse_in_mixed_transports() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    mcp_migration_fixture(&connection);
    insert_mcp_server(&connection, "local-command", "stdio");
    insert_mcp_server(&connection, "historical-url", "sse");
    insert_mcp_server(&connection, "modern-url", "streamable_http");

    run_mcp_transport_migration(&connection).expect("transport migration");

    let journal_names: Vec<String> = connection
        .prepare(
            "SELECT server_name FROM mcp_transport_migration_journal \
             WHERE migration_version = 36 ORDER BY server_name",
        )
        .expect("journal query")
        .query_map([], |row| row.get(0))
        .expect("journal rows")
        .collect::<Result<_, _>>()
        .expect("journal names");
    assert_eq!(transport_type(&connection, "local-command"), "stdio");
    assert_eq!(
        transport_type(&connection, "historical-url"),
        "streamable_http"
    );
    assert_eq!(transport_type(&connection, "modern-url"), "streamable_http");
    assert_eq!(journal_names, vec!["historical-url"]);
}

#[test]
fn mcp_transport_migration_rolls_back_all_changes_on_failure() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    mcp_migration_fixture(&connection);
    insert_mcp_server(&connection, "reject-update", "sse");
    connection
        .execute_batch(
            r#"
            CREATE TRIGGER reject_mcp_transport_update
            BEFORE UPDATE OF transport_type ON mcp_servers
            BEGIN
                SELECT RAISE(ABORT, 'injected migration failure');
            END;
            "#,
        )
        .expect("failure trigger");

    assert!(run_mcp_transport_migration(&connection).is_err());

    let version_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 36",
            [],
            |row| row.get(0),
        )
        .expect("migration version count");
    let journal_table_exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'mcp_transport_migration_journal'",
            [],
            |row| row.get(0),
        )
        .expect("journal table check");
    assert_eq!(transport_type(&connection, "reject-update"), "sse");
    assert_eq!(version_rows, 0);
    assert_eq!(journal_table_exists, 0);
}

#[test]
fn mcp_transport_journal_supports_a_targeted_down_migration() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    mcp_migration_fixture(&connection);
    insert_mcp_server(&connection, "historical-url", "sse");
    insert_mcp_server(&connection, "modern-url", "streamable_http");
    run_mcp_transport_migration(&connection).expect("transport migration");

    let transaction = connection
        .unchecked_transaction()
        .expect("down migration transaction");
    transaction
        .execute_batch(
            r#"
            UPDATE mcp_servers
            SET transport_type = (
                SELECT journal.previous_transport_type
                FROM mcp_transport_migration_journal AS journal
                WHERE journal.migration_version = 36
                  AND journal.server_name = mcp_servers.name
            )
            WHERE transport_type = 'streamable_http'
              AND EXISTS (
                SELECT 1
                FROM mcp_transport_migration_journal AS journal
                WHERE journal.migration_version = 36
                  AND journal.server_name = mcp_servers.name
                  AND journal.migrated_transport_type = mcp_servers.transport_type
              );
            DELETE FROM schema_migrations WHERE version = 36;
            DELETE FROM mcp_transport_migration_journal WHERE migration_version = 36;
            "#,
        )
        .expect("journal down migration");
    transaction.commit().expect("commit down migration");

    assert_eq!(transport_type(&connection, "historical-url"), "sse");
    assert_eq!(transport_type(&connection, "modern-url"), "streamable_http");
}

#[test]
fn mcp_transport_migration_survives_reopen_without_reapplying() {
    let directory = TempDirectory::new("mcp-transport-migration-reopen");
    let path = directory.path().join("migration.sqlite");
    {
        let connection = Connection::open(&path).expect("fixture database");
        mcp_migration_fixture(&connection);
        insert_mcp_server(&connection, "historical-url", "sse");
        run_mcp_transport_migration(&connection).expect("transport migration");
    }

    let reopened = Connection::open(&path).expect("reopened database");
    run_mcp_transport_migration(&reopened).expect("reopen migration");
    let journal_rows: i64 = reopened
        .query_row(
            "SELECT COUNT(*) FROM mcp_transport_migration_journal",
            [],
            |row| row.get(0),
        )
        .expect("journal count");
    assert_eq!(
        transport_type(&reopened, "historical-url"),
        "streamable_http"
    );
    assert_eq!(journal_rows, 1);
}

fn coordination_table_exists(connection: &Connection) -> bool {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'coordination_runs'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("table lookup")
        > 0
}

#[test]
fn remove_coordination_migration_drops_a_pre_existing_table_and_is_idempotent() {
    let connection = Connection::open_in_memory().expect("database");
    // Recreate what migration 27 used to leave behind on an existing install.
    connection
        .execute_batch(
            "CREATE TABLE coordination_runs (id TEXT PRIMARY KEY, run_snapshot TEXT NOT NULL);",
        )
        .expect("seed legacy table");
    assert!(coordination_table_exists(&connection));

    apply_remove_coordination_migration(&connection).expect("first apply");
    assert!(!coordination_table_exists(&connection));

    apply_remove_coordination_migration(&connection).expect("second apply");
    assert!(!coordination_table_exists(&connection));
}

#[test]
fn migrate_leaves_no_coordination_table_on_a_fresh_database() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migrate");
    assert!(!coordination_table_exists(&connection));
}

#[test]
fn session_seat_migration_adds_the_column_and_leaves_existing_rows_readable() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migrate");

    // `sessions.agent_id` carries a real FOREIGN KEY into `agents`, and `migrate` creates
    // tables without seeding rows, so the referenced agent has to be inserted here.
    connection
        .execute(
            "INSERT INTO agents (id, display_name, provider, launch_kind)                  VALUES ('claude-code', 'Claude Code', 'Anthropic', 'cli')",
            [],
        )
        .expect("agent fixture");
    let agent_id = "claude-code";

    // A session written before seats existed must still read back, with an empty seat list that
    // callers present as the one-seat case built from agent_id.
    connection
        .execute(
            "INSERT INTO sessions (id, title, agent_id, interaction_mode, lifecycle_state,                  pinned, archived, created_at, updated_at)                  VALUES ('s1', 'legacy', ?1, 'cli', 'idle', 0, 0, 't', 't')",
            params![agent_id],
        )
        .expect("legacy session insert");

    let seats: String = connection
        .query_row("SELECT seats FROM sessions WHERE id = 's1'", [], |row| {
            row.get(0)
        })
        .expect("seats column readable");
    assert_eq!(seats, "[]");

    // Re-running must not fail or duplicate the column.
    migrate(&connection).expect("idempotent migrate");
}

/// `EXPECTED_MIGRATIONS` is the ground truth the post-migration density check compares
/// against, so it must stay in lockstep with the `apply_migration` /
/// `apply_transactional_migration` calls in `migrate`. A fresh migrate must produce exactly
/// those (version, name) rows — this guards against both drift in the constant and a
/// silent version-number collision (the second migration claiming a number is skipped, so
/// the recorded name would be the first's, not the expected one).
#[test]
fn migration_sequence_matches_expected() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migrate");

    let mut rows = connection
        .prepare("SELECT version, name FROM schema_migrations ORDER BY version ASC")
        .expect("prepare");
    let recorded: Vec<(i64, String)> = rows
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");

    let expected: Vec<(i64, String)> = EXPECTED_MIGRATIONS
        .iter()
        .map(|(v, n)| (*v, (*n).to_string()))
        .collect();
    assert_eq!(
        recorded, expected,
        "EXPECTED_MIGRATIONS drifted from migrate()"
    );
}

#[test]
fn session_execution_policy_migration_resets_legacy_security_configuration() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migrate");
    connection
        .execute(
            "INSERT INTO agents (id, display_name, provider, launch_kind) \
             VALUES ('codex-cli', 'Codex CLI', 'OpenAI', 'cli')",
            [],
        )
        .expect("agent fixture");
    connection
        .execute(
            "INSERT INTO sessions (id, title, agent_id, interaction_mode, lifecycle_state, \
             pinned, archived, created_at, updated_at, chat_preferences) \
             VALUES ('execution-policy-session', 'legacy', 'codex-cli', 'cli', 'idle', \
             0, 0, 't', 't', '{\"permissionMode\":\"auto\"}')",
            [],
        )
        .expect("legacy session");
    for (parameter_id, value) in [
        ("sandbox", "\"workspace-write\""),
        ("approvalPolicy", "\"never\""),
        ("ephemeral", "true"),
    ] {
        connection
            .execute(
                "INSERT INTO cli_parameter_settings \
                 (agent_id, parameter_id, enabled, value_json, updated_at) \
                 VALUES ('codex-cli', ?1, 1, ?2, 't')",
                params![parameter_id, value],
            )
            .expect("legacy CLI selection");
    }

    apply_session_execution_policy_migration(&connection).expect("policy migration");

    let preferences: Option<String> = connection
        .query_row(
            "SELECT chat_preferences FROM sessions WHERE id = 'execution-policy-session'",
            [],
            |row| row.get(0),
        )
        .expect("preferences");
    assert_eq!(preferences, None);
    let remaining: Vec<String> = connection
        .prepare(
            "SELECT parameter_id FROM cli_parameter_settings \
             WHERE agent_id = 'codex-cli' ORDER BY parameter_id",
        )
        .expect("prepare")
        .query_map([], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(remaining, vec!["ephemeral".to_string()]);
}

/// A non-dense history (a missing row, as a mid-migration failure + unrecorded version
/// would leave) must fail the startup density check rather than booting with a diverged
/// schema.
#[test]
fn density_check_rejects_a_missing_migration_row() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("migrate");
    connection
        .execute("DELETE FROM schema_migrations WHERE version = 40", [])
        .expect("delete a row to create a gap");

    let error = assert_migration_history_is_dense(&connection)
        .expect_err("a gapped history must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("not dense"),
        "expected a density error, got: {message}"
    );
}

#[test]
fn session_recovery_migration_adds_schema_and_backfills_durable_order() {
    let connection = Connection::open_in_memory().expect("database");
    legacy_session_recovery_fixture(&connection);

    apply_session_recovery_foundation_migration(&connection).expect("recovery migration");

    for column in [
        "recovery_status",
        "recovery_revision",
        "state_revision",
        "history_revision",
        "active_execution_run_id",
        "next_message_sequence",
    ] {
        assert!(table_has_column(&connection, "sessions", column).expect("session column"));
    }
    for column in [
        "session_sequence",
        "execution_run_id",
        "seat_round_id",
        "parent_execution_run_id",
    ] {
        assert!(table_has_column(&connection, "messages", column).expect("message column"));
    }

    let ordered: Vec<(String, i64, Option<String>)> = connection
        .prepare(
            "SELECT id, session_sequence, execution_run_id FROM messages \
             WHERE session_id = 'clean-legacy' ORDER BY session_sequence",
        )
        .expect("ordered messages")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("message query")
        .collect::<Result<_, _>>()
        .expect("message rows");
    assert_eq!(
        ordered,
        vec![
            ("message-a".to_string(), 1, None),
            ("message-b".to_string(), 2, None),
            ("message-c".to_string(), 3, None),
        ]
    );

    let session_defaults: (String, i64, i64, i64, Option<String>, i64) = connection
        .query_row(
            "SELECT recovery_status, recovery_revision, state_revision, history_revision, \
                    active_execution_run_id, next_message_sequence \
             FROM sessions WHERE id = 'clean-legacy'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("session defaults");
    assert_eq!(session_defaults, ("clean".to_string(), 0, 0, 0, None, 4));

    let unique_index: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'index' AND name = 'idx_messages_session_sequence'",
            [],
            |row| row.get(0),
        )
        .expect("sequence index");
    let reports_table: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'session_recovery_reports'",
            [],
            |row| row.get(0),
        )
        .expect("recovery reports table");
    assert_eq!(unique_index, 1);
    assert_eq!(reports_table, 1);
}

#[test]
fn session_recovery_migration_preserves_runtime_and_orchestrator_evidence() {
    let connection = Connection::open_in_memory().expect("database");
    legacy_session_recovery_fixture(&connection);

    apply_session_recovery_foundation_migration(&connection).expect("first migration");
    apply_session_recovery_foundation_migration(&connection).expect("idempotent migration");

    let orphan: (String, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT lifecycle_state, runtime_session_id, active_execution_run_id \
             FROM sessions WHERE id = 'orphan-active'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("orphan session");
    assert_eq!(
        orphan,
        (
            "running".to_string(),
            Some("provider-resume-1".to_string()),
            None
        )
    );

    let tool_use: String = connection
        .query_row(
            "SELECT tool_use FROM messages WHERE id = 'message-tool'",
            [],
            |row| row.get(0),
        )
        .expect("tool snapshot");
    assert_eq!(tool_use, r#"[{"id":"tool-1","status":"running"}]"#);

    let plan_attempt: (String, String) = connection
        .query_row(
            "SELECT session_id, status FROM plan_subtask_attempts WHERE id = 'attempt-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("plan attempt");
    assert_eq!(
        plan_attempt,
        ("plan-owned".to_string(), "running".to_string())
    );

    let loop_owner: (String, String, String) = connection
        .query_row(
            "SELECT loop_run_id, loop_iteration_id, loop_role \
             FROM sessions WHERE id = 'loop-owned'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("loop ownership");
    assert_eq!(
        loop_owner,
        (
            "loop-run-1".to_string(),
            "iteration-1".to_string(),
            "worker".to_string()
        )
    );
}

#[test]
fn fresh_database_retires_plan_work_board_sources() {
    let connection = Connection::open_in_memory().expect("database");
    migrate(&connection).expect("fresh migration");
    connection
        .execute(
            "INSERT INTO work_items
             (id,title,stage,priority,rank,created_at,updated_at)
             VALUES ('item-1','item','inbox','none',1000,'now','now')",
            [],
        )
        .expect("work item fixture");

    let result = connection.execute(
        "INSERT INTO work_item_links
         (work_item_id, source_kind, source_id, relation, created_at)
         VALUES ('item-1', 'plan', 'plan-1', 'primary', 'now')",
        [],
    );
    assert!(result.is_err(), "the retired source kind must be rejected");
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 80",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("migration row"),
        1
    );
}

#[test]
fn retire_plan_execution_preserves_history_and_mixed_work_items() {
    let directory = TempDirectory::new("retire-plan-execution");
    let worktree = directory.path().join("recorded-plan-worktree");
    std::fs::create_dir_all(&worktree).expect("recorded worktree fixture");
    let marker = worktree.join("marker.txt");
    std::fs::write(&marker, "keep").expect("worktree marker");

    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE sessions (id TEXT PRIMARY KEY);
            CREATE TABLE work_items (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
                stage TEXT NOT NULL, priority TEXT NOT NULL, rank INTEGER NOT NULL,
                project_path TEXT, due_at TEXT, archived INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE work_item_links (
                work_item_id TEXT NOT NULL,
                source_kind TEXT NOT NULL CHECK(source_kind IN ('session','plan','plan_run','scheduled_task')),
                source_id TEXT NOT NULL,
                relation TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (source_kind, source_id),
                FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_work_item_links_item ON work_item_links(work_item_id);
            INSERT INTO work_items
                (id,title,stage,priority,rank,created_at,updated_at)
            VALUES ('plan-only','Plan only','planned','none',1000,'now','now'),
                   ('mixed','Mixed','planned','none',2000,'now','now');
            INSERT INTO work_item_links VALUES
                ('plan-only','plan','plan-1','primary','now'),
                ('mixed','plan_run','plan-run-1','execution','now'),
                ('mixed','session','session-1','supporting','now');
            "#,
        )
        .expect("legacy work board fixture");
    crate::platform::legacy_plan_schema::apply_legacy_plan_session_association_schema(&connection)
        .expect("legacy plan schema");
    crate::contexts::operations::infrastructure::apply_run_schema(&connection)
        .expect("agent run schema");
    crate::contexts::operations::infrastructure::apply_runner_projection_schema(&connection)
        .expect("runner projection schema");

    connection
        .execute_batch(
            r#"
            INSERT INTO sessions (id) VALUES ('session-1');
            INSERT INTO plans (id,status,current_version,created_at,updated_at)
            VALUES ('plan-1','approved',1,'now','now');
            INSERT INTO plan_versions
                (id,plan_id,version,goal,project_path,base_ref,created_at)
            VALUES ('plan-version-1','plan-1',1,'goal','project','main','now');
            "#,
        )
        .expect("historical plan fixture");
    connection
        .execute(
            "INSERT INTO plan_runs
             (id,plan_id,plan_version_id,status,project_path,base_ref,worktree_path,
              created_at,updated_at,driver_intent)
             VALUES ('plan-run-1','plan-1','plan-version-1','running','project','main',?1,
                     'now','now','run')",
            [worktree.to_string_lossy().as_ref()],
        )
        .expect("active plan run");
    connection
        .execute_batch(
            r#"
            INSERT INTO agent_runs
                (run_id,owner_type,owner_id,state,version,updated_at,snapshot_json)
            VALUES (
                'agent-run-1','plan_run','plan-run-1','running',1,'now',
                '{"id":"agent-run-1","owner":{"ownerType":"plan_run","ownerId":"plan-run-1"},"links":[],"state":"running","version":1,"updatedAt":"now","events":[]}'
            );
            "#,
        )
        .expect("canonical run fixture");

    crate::platform::legacy_plan_schema::apply_retire_plan_execution_migration(&connection)
        .expect("first retirement");
    crate::platform::legacy_plan_schema::apply_retire_plan_execution_migration(&connection)
        .expect("idempotent retirement");

    let plan_run: (String, String, String) = connection
        .query_row(
            "SELECT status,driver_intent,worktree_path FROM plan_runs WHERE id='plan-run-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retained plan run");
    assert_eq!(plan_run.0, "cancelled");
    assert_eq!(plan_run.1, "stopped");
    assert_eq!(plan_run.2, worktree.to_string_lossy());
    assert!(
        marker.exists(),
        "migration must not mutate filesystem worktrees"
    );

    let canonical: (String, i64, String, i64) = connection
        .query_row(
            "SELECT state,version,json_extract(snapshot_json,'$.state'),
                    (SELECT COUNT(*) FROM agent_run_events WHERE run_id=agent_runs.run_id)
             FROM agent_runs WHERE run_id='agent-run-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("terminal canonical run");
    assert_eq!(canonical, ("cancelled".into(), 2, "cancelled".into(), 1));

    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM plans", [], |row| row.get::<_, i64>(0))
            .expect("plan history"),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM work_items WHERE id='plan-only'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .expect("plan-only item"),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM work_item_links WHERE work_item_id='mixed' AND source_kind='session'", [], |row| row.get::<_, i64>(0))
            .expect("mixed retained source"),
        1
    );
}
