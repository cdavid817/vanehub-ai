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
