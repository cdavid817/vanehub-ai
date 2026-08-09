use super::DatabaseError;
use crate::contexts::agent_runtime::domain::AgentLifecycle;
use crate::contexts::tooling::cli_parameters;
use rusqlite::{params, Connection, OptionalExtension};

pub(crate) fn migrate(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%s', 'now'))
        );
        "#,
    )?;

    apply_migration(conn, 1, "initial-schema", apply_initial_schema)?;
    apply_migration(
        conn,
        2,
        "agent-managed-sdk-dependency",
        apply_agent_sdk_dependency_migration,
    )?;
    apply_migration(
        conn,
        3,
        "session-management",
        apply_session_management_migration,
    )?;
    apply_migration(conn, 4, "chat-messages", apply_chat_messages_migration)?;
    apply_migration(conn, 5, "app-settings", apply_app_settings_migration)?;
    apply_migration(conn, 6, "cli-tool-status", apply_cli_tool_status_migration)?;
    apply_migration(
        conn,
        7,
        "skill-management",
        crate::contexts::tooling::skills::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        8,
        "project-worktree-management",
        apply_project_worktree_migration,
    )?;
    apply_migration(
        conn,
        9,
        "session-runtime-metadata",
        apply_session_runtime_metadata_migration,
    )?;
    apply_migration(
        conn,
        10,
        "im-connectors",
        crate::contexts::communications::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        11,
        "im-session-source",
        crate::contexts::communications::infrastructure::apply_session_source_schema,
    )?;
    apply_migration(
        conn,
        12,
        "cli-parameter-settings",
        apply_cli_parameter_settings_migration,
    )?;
    apply_migration(
        conn,
        13,
        "session-chat-configuration",
        crate::contexts::sessions::infrastructure::apply_configuration_schema,
    )?;
    apply_migration(
        conn,
        14,
        "floating-assistant-configuration",
        crate::contexts::desktop::infrastructure::apply_floating_assistant_schema,
    )?;
    apply_migration(
        conn,
        15,
        "local-extension-management",
        crate::contexts::tooling::extensions::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        16,
        "cli-local-environment-details",
        apply_cli_environment_details_migration,
    )?;
    apply_migration(
        conn,
        17,
        "message-rich-blocks",
        apply_message_rich_blocks_migration,
    )?;
    apply_migration(
        conn,
        18,
        "session-management-organization",
        apply_session_management_organization_migration,
    )?;
    apply_migration(
        conn,
        19,
        "prompt-hook-management",
        crate::contexts::tooling::prompt_hooks::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        20,
        "remote-workspace-sessions",
        apply_remote_workspace_migration,
    )?;
    apply_migration(
        conn,
        21,
        "sdk-operation-logs",
        crate::contexts::tooling::sdk::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        22,
        "session-usage-records",
        crate::contexts::sessions::infrastructure::apply_usage_schema,
    )?;
    apply_migration(
        conn,
        23,
        "scheduled-task-management",
        apply_scheduled_task_management_migration,
    )?;
    apply_migration(
        conn,
        24,
        "ssh-connection-management",
        apply_ssh_connection_management_migration,
    )?;
    apply_migration(conn, 25, "loop-engineering-runtime", |connection| {
        crate::contexts::agent_runtime::infrastructure::apply_loop_schema(connection)?;
        crate::contexts::sessions::infrastructure::apply_loop_ownership_schema(connection)
    })?;
    apply_migration(
        conn,
        26,
        "agent-execution-observability",
        crate::contexts::execution_observability::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        27,
        "multi-agent-coordination",
        apply_retired_coordination_schema,
    )?;
    apply_migration(
        conn,
        28,
        "remote-terminal-management",
        crate::contexts::workspaces::infrastructure::apply_remote_terminal_schema,
    )?;
    apply_migration(
        conn,
        29,
        "api-agent-registration",
        crate::contexts::agent_runtime::infrastructure::apply_api_agent_schema,
    )?;
    apply_migration(
        conn,
        30,
        "openai-compatible-agent-registration",
        crate::contexts::agent_runtime::infrastructure::apply_openai_compatible_schema,
    )?;
    apply_migration(
        conn,
        31,
        "agent-cross-session-memory",
        crate::contexts::agent_runtime::infrastructure::apply_memory_schema,
    )?;
    apply_migration(
        conn,
        32,
        "agent-tool-trust",
        crate::contexts::agent_runtime::infrastructure::apply_agent_tool_trust_schema,
    )?;
    apply_migration(
        conn,
        33,
        "session-message-search-index",
        apply_session_message_search_migration,
    )?;
    apply_migration(
        conn,
        34,
        "cli-agent-global-config",
        crate::contexts::tooling::cli_config::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        35,
        "cli-agent-applied-ownership-snapshot",
        crate::contexts::tooling::cli_config::infrastructure::apply_applied_snapshot_schema,
    )?;
    apply_transactional_migration(
        conn,
        36,
        "mcp-truthful-url-transports",
        apply_mcp_truthful_url_transport_migration,
    )?;
    apply_transactional_migration(
        conn,
        37,
        "skill-management-reliability",
        crate::contexts::tooling::skills::infrastructure::apply_reliability_schema,
    )?;
    apply_migration(
        conn,
        38,
        "agent-management-origin",
        crate::contexts::agent_runtime::infrastructure::apply_agent_origin_schema,
    )?;
    apply_migration(
        conn,
        39,
        "onepiece-provider-profiles",
        crate::contexts::agent_runtime::infrastructure::apply_onepiece_provider_profiles_schema,
    )?;
    apply_migration(
        conn,
        40,
        "onepiece-provider-catalog",
        crate::contexts::agent_runtime::infrastructure::apply_onepiece_provider_catalog_schema,
    )?;
    apply_migration(
        conn,
        41,
        "onepiece-provider-endpoints",
        crate::contexts::agent_runtime::infrastructure::apply_onepiece_provider_endpoint_schema,
    )?;
    apply_migration(
        conn,
        42,
        "agent-memory-shared-pool",
        crate::contexts::agent_runtime::infrastructure::apply_memory_shared_pool_schema,
    )?;
    // 43, not 42: `agent-memory-shared-pool` landed on main under 42 first and may already be in
    // users' databases. `apply_migration` 是版本门控的——两条 42 号迁移里的第二条永远不会执行，
    // 启动即 "no such table: retrieval_documents"。
    apply_migration(
        conn,
        43,
        "retrieval-vector-index",
        crate::contexts::retrieval::infrastructure::apply_retrieval_schema,
    )?;
    // 44, not 42: this worktree's own `permissions-core` migration originally claimed 42 too,
    // independently of `agent-memory-shared-pool` above — same class of collision, same fix.
    // Renumbered on merge rather than kept at 42, since 42/43 already shipped under those names.
    apply_migration(conn, 44, "permissions-core", |connection| {
        crate::contexts::permissions::infrastructure::schema::apply_permissions_core_schema(
            connection,
        )?;
        crate::contexts::permissions::infrastructure::schema::backfill_principals_from_legacy_trust_flag(
            connection,
        )
    })?;
    // 45-48, not 43-46: `retrieval-vector-index` and `permissions-core` reached main under 43 and
    // 44 while this branch was open, so these four move up behind them. `apply_migration` is
    // version-gated — the second migration to claim a number never runs, and the table it was
    // supposed to create is simply missing at startup.
    apply_migration(
        conn,
        45,
        "remove-multi-agent-coordination",
        apply_remove_coordination_migration,
    )?;
    apply_migration(
        conn,
        46,
        "expert-role-management",
        crate::contexts::agent_runtime::infrastructure::apply_expert_role_schema,
    )?;
    apply_migration(
        conn,
        47,
        "session-seats",
        crate::contexts::sessions::infrastructure::apply_session_seat_schema,
    )?;
    apply_migration(
        conn,
        48,
        "message-speaker",
        crate::contexts::sessions::infrastructure::apply_message_speaker_schema,
    )?;
    apply_migration(
        conn,
        49,
        "plan-execution-foundation",
        crate::contexts::task_orchestration::infrastructure::apply_schema,
    )?;
    apply_migration(
        conn,
        50,
        "plan-and-code-index-reconciliation",
        apply_plan_and_code_index_reconciliation,
    )?;

    Ok(())
}

// Migration 49 existed as either Plan execution on main or workspace code indexing in a
// concurrent worktree. Reapply both idempotent schemas so databases from either history converge.
fn apply_plan_and_code_index_reconciliation(conn: &Connection) -> Result<(), DatabaseError> {
    crate::contexts::task_orchestration::infrastructure::apply_schema(conn)?;
    crate::contexts::retrieval::infrastructure::apply_code_index_schema(conn)
}

/// Version 27 created the multi-Agent coordination table. The capability is retired, so the slot
/// is kept as a no-op rather than deleted: the migration sequence is asserted to be dense by the
/// fixture tests, and leaving a permanent hole would make every future migration carry the gap.
fn apply_retired_coordination_schema(_conn: &Connection) -> Result<(), DatabaseError> {
    Ok(())
}

/// Drops what version 27 left behind on installs that actually ran it. On a fresh database the
/// table was never created and this is a no-op.
///
/// Numbered 43, not 42: the concurrently-developed `permissions-core` branch already claimed 42
/// and has run it on shared local databases (every worktree shares one `ai.vanehub.app` database),
/// so reusing 42 would leave this migration permanently skipped there and the table undropped.
fn apply_remove_coordination_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch("DROP TABLE IF EXISTS coordination_runs;")?;
    Ok(())
}

fn apply_mcp_truthful_url_transport_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS mcp_transport_migration_journal (
            migration_version INTEGER NOT NULL,
            server_name TEXT NOT NULL,
            previous_transport_type TEXT NOT NULL,
            migrated_transport_type TEXT NOT NULL,
            migrated_at TEXT NOT NULL DEFAULT (strftime('%s', 'now')),
            PRIMARY KEY (migration_version, server_name)
        );

        INSERT OR IGNORE INTO mcp_transport_migration_journal (
            migration_version,
            server_name,
            previous_transport_type,
            migrated_transport_type
        )
        SELECT 36, name, 'sse', 'streamable_http'
        FROM mcp_servers
        WHERE transport_type = 'sse';

        UPDATE mcp_servers
        SET transport_type = 'streamable_http'
        WHERE transport_type = 'sse';
        "#,
    )?;
    Ok(())
}

fn apply_session_message_search_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS session_message_fts USING fts5(
            content,
            content='messages',
            content_rowid='rowid',
            tokenize='trigram'
        );

        CREATE TRIGGER IF NOT EXISTS messages_fts_insert
        AFTER INSERT ON messages BEGIN
            INSERT INTO session_message_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS messages_fts_delete
        AFTER DELETE ON messages BEGIN
            INSERT INTO session_message_fts(session_message_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS messages_fts_update
        AFTER UPDATE OF content ON messages BEGIN
            INSERT INTO session_message_fts(session_message_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
            INSERT INTO session_message_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        INSERT INTO session_message_fts(session_message_fts) VALUES ('rebuild');
        "#,
    )?;
    Ok(())
}

fn apply_scheduled_task_management_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS scheduled_tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            frequency TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            next_run_at TEXT NOT NULL,
            latest_status TEXT NOT NULL DEFAULT 'never-run',
            latest_run_at TEXT,
            latest_run_session_id TEXT,
            latest_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        );

        CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_enabled_next_run
            ON scheduled_tasks(enabled, next_run_at);
        "#,
    )?;
    Ok(())
}

fn apply_cli_parameter_settings_migration(conn: &Connection) -> Result<(), DatabaseError> {
    cli_parameters::apply_schema(conn).map_err(|error| DatabaseError::Storage(error.to_string()))
}

fn apply_remote_workspace_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS known_remote_workspaces (
            uri TEXT PRIMARY KEY,
            host TEXT NOT NULL,
            user TEXT,
            path TEXT NOT NULL,
            display_name TEXT NOT NULL,
            last_opened_at TEXT NOT NULL
        );
        "#,
    )?;
    for column in [
        "remote_workspace_host",
        "remote_workspace_user",
        "remote_workspace_path",
        "remote_workspace_display_name",
        "remote_workspace_uri",
    ] {
        if !table_has_column(conn, "sessions", column)? {
            conn.execute(
                &format!("ALTER TABLE sessions ADD COLUMN {column} TEXT"),
                [],
            )?;
        }
    }
    Ok(())
}

fn apply_ssh_connection_management_migration(conn: &Connection) -> Result<(), DatabaseError> {
    crate::contexts::ssh_connections::apply_schema(conn)?;
    if !table_has_column(conn, "known_remote_workspaces", "port")? {
        conn.execute(
            "ALTER TABLE known_remote_workspaces ADD COLUMN port INTEGER NOT NULL DEFAULT 22",
            [],
        )?;
    }
    if !table_has_column(conn, "sessions", "remote_workspace_port")? {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN remote_workspace_port INTEGER",
            [],
        )?;
    }
    Ok(())
}

fn apply_session_management_organization_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS session_categories (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_session_categories_sort
            ON session_categories(sort_order, name);
        "#,
    )?;
    if !table_has_column(conn, "sessions", "category_id")? {
        conn.execute("ALTER TABLE sessions ADD COLUMN category_id TEXT", [])?;
    }
    if !table_has_column(conn, "messages", "file_references")? {
        conn.execute("ALTER TABLE messages ADD COLUMN file_references TEXT", [])?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_category_updated ON sessions(category_id, updated_at)",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value, created_at, updated_at)
         VALUES ('automaticArchivalEnabled', 'true', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value, created_at, updated_at)
         VALUES ('automaticArchivalInactiveDays', '10', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        [],
    )?;
    Ok(())
}

fn apply_project_worktree_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS known_projects (
            path TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            is_git INTEGER NOT NULL DEFAULT 0,
            last_opened_at TEXT NOT NULL
        );
        "#,
    )?;
    for column in [
        "project_path",
        "worktree_path",
        "worktree_name",
        "worktree_branch",
    ] {
        if !table_has_column(conn, "sessions", column)? {
            conn.execute(
                &format!("ALTER TABLE sessions ADD COLUMN {column} TEXT"),
                [],
            )?;
        }
    }
    Ok(())
}

fn apply_session_runtime_metadata_migration(conn: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "sessions", "runtime_session_id")? {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN runtime_session_id TEXT",
            [],
        )?;
    }
    Ok(())
}

fn apply_cli_tool_status_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_tool_status (
            agent_id TEXT PRIMARY KEY,
            installed INTEGER,
            current_version TEXT,
            latest_version TEXT,
            available_versions TEXT NOT NULL DEFAULT '[]',
            detected_path TEXT,
            last_checked_at TEXT,
            last_error TEXT,
            last_operation_id TEXT,
            version_check_status TEXT NOT NULL DEFAULT 'not-detected'
        );
        "#,
    )?;
    Ok(())
}

fn apply_cli_environment_details_migration(conn: &Connection) -> Result<(), DatabaseError> {
    let columns = [
        ("environment_type", "TEXT NOT NULL DEFAULT 'unknown'"),
        ("installations", "TEXT NOT NULL DEFAULT '[]'"),
        ("active_installation_path", "TEXT"),
        ("conflict_state", "TEXT NOT NULL DEFAULT 'none'"),
        (
            "lifecycle_eligibility",
            "TEXT NOT NULL DEFAULT 'unavailable'",
        ),
    ];
    for (column, definition) in columns {
        if !table_has_column(conn, "cli_tool_status", column)? {
            conn.execute(
                &format!("ALTER TABLE cli_tool_status ADD COLUMN {column} {definition}"),
                [],
            )?;
        }
    }
    Ok(())
}

fn apply_message_rich_blocks_migration(conn: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "messages", "rich_blocks")? {
        conn.execute("ALTER TABLE messages ADD COLUMN rich_blocks TEXT", [])?;
    }
    Ok(())
}

fn apply_app_settings_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn apply_chat_messages_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'completed',
            content TEXT NOT NULL DEFAULT '',
            thinking_content TEXT,
            tool_use TEXT,
            rich_blocks TEXT,
            token_input INTEGER DEFAULT 0,
            token_output INTEGER DEFAULT 0,
            metadata TEXT,
            file_references TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_created
            ON messages(session_id, created_at);
        "#,
    )?;
    Ok(())
}

fn apply_migration(
    conn: &Connection,
    version: i64,
    name: &str,
    migration: fn(&Connection) -> Result<(), DatabaseError>,
) -> Result<(), DatabaseError> {
    let applied = conn
        .query_row(
            "SELECT 1 FROM schema_migrations WHERE version = ?1",
            params![version],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if applied {
        return Ok(());
    }

    migration(conn)?;
    conn.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        params![version, name],
    )?;
    Ok(())
}

fn apply_transactional_migration(
    conn: &Connection,
    version: i64,
    name: &str,
    migration: fn(&Connection) -> Result<(), DatabaseError>,
) -> Result<(), DatabaseError> {
    let applied = conn
        .query_row(
            "SELECT 1 FROM schema_migrations WHERE version = ?1",
            params![version],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if applied {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    migration(&transaction)?;
    transaction.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        params![version, name],
    )?;
    transaction.commit()?;
    Ok(())
}

fn apply_initial_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            provider TEXT NOT NULL,
            launch_kind TEXT NOT NULL,
            launch_command TEXT,
            launch_url TEXT,
            executable_name TEXT,
            managed_sdk_dependency_id TEXT
        );

        CREATE TABLE IF NOT EXISTS agent_modes (
            agent_id TEXT NOT NULL,
            mode TEXT NOT NULL,
            PRIMARY KEY (agent_id, mode),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS agent_capability_tags (
            agent_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (agent_id, tag),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS workflow_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            active_agent_id TEXT,
            active_interaction_mode TEXT,
            lifecycle_state TEXT NOT NULL,
            intent TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS session_details (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            adapter TEXT NOT NULL,
            message TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS mcp_servers (
            name TEXT PRIMARY KEY,
            transport_type TEXT NOT NULL DEFAULT 'stdio',
            command TEXT,
            args TEXT,
            env TEXT,
            url TEXT,
            headers TEXT,
            description TEXT,
            active INTEGER NOT NULL DEFAULT 1,
            scope TEXT NOT NULL DEFAULT 'user',
            project_path TEXT,
            last_connection_status TEXT,
            last_connected TEXT,
            last_error TEXT,
            last_tools TEXT,
            last_test_duration_ms INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO workflow_state (id, lifecycle_state, intent) VALUES (1, ?1, ?2)",
        params![
            AgentLifecycle::Idle.as_str(),
            "Current development workflow"
        ],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO session_details (id, adapter, message) VALUES (1, ?1, ?2)",
        params!["none", "No active session."],
    )?;

    Ok(())
}

fn apply_agent_sdk_dependency_migration(conn: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "agents", "managed_sdk_dependency_id")? {
        conn.execute(
            "ALTER TABLE agents ADD COLUMN managed_sdk_dependency_id TEXT",
            [],
        )?;
    }
    Ok(())
}

fn apply_session_management_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            interaction_mode TEXT NOT NULL,
            lifecycle_state TEXT NOT NULL,
            folder TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            archived INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        );
        "#,
    )?;

    if !table_has_column(conn, "workflow_state", "active_session_id")? {
        conn.execute(
            "ALTER TABLE workflow_state ADD COLUMN active_session_id TEXT",
            [],
        )?;
    }

    Ok(())
}

pub(crate) fn table_has_column(
    conn: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, DatabaseError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

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
        assert_eq!(migration_state, (48, 49));

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
}
