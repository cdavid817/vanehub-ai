//! Migration bodies that no bounded context owns.
//!
//! Most migrations in `migrate` delegate to an `apply_schema` published by the context that
//! owns the table. The ones here either predate that decomposition or have never been claimed,
//! so they are grouped rather than scattered: the grouping is what makes the unclaimed set
//! countable, and each can move to its owner independently without disturbing `migrate`'s call
//! order — which is version-gated, so reordering it is not a refactor but a schema change.

use super::{table_has_column, DatabaseError};
use crate::contexts::agent_runtime::domain::AgentLifecycle;
use crate::contexts::tooling::cli_parameters;
use rusqlite::{params, Connection};

pub(super) fn apply_session_recovery_performance_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS messages_fts_insert;
        DROP TRIGGER IF EXISTS messages_fts_delete;
        DROP TRIGGER IF EXISTS messages_fts_update;
        DROP TRIGGER IF EXISTS messages_fts_enter_streaming;
        DROP TRIGGER IF EXISTS messages_fts_leave_streaming;

        INSERT INTO session_message_fts(session_message_fts, rowid, content)
        SELECT 'delete', rowid, content
        FROM messages
        WHERE status = 'streaming';

        CREATE TRIGGER messages_fts_insert
        AFTER INSERT ON messages
        WHEN new.status <> 'streaming' BEGIN
            INSERT INTO session_message_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER messages_fts_delete
        AFTER DELETE ON messages
        WHEN old.status <> 'streaming' BEGIN
            INSERT INTO session_message_fts(session_message_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER messages_fts_update
        AFTER UPDATE OF content ON messages
        WHEN old.status <> 'streaming' AND new.status <> 'streaming' BEGIN
            INSERT INTO session_message_fts(session_message_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
            INSERT INTO session_message_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER messages_fts_enter_streaming
        AFTER UPDATE OF status ON messages
        WHEN old.status <> 'streaming' AND new.status = 'streaming' BEGIN
            INSERT INTO session_message_fts(session_message_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER messages_fts_leave_streaming
        AFTER UPDATE OF status ON messages
        WHEN old.status = 'streaming' AND new.status <> 'streaming' BEGIN
            INSERT INTO session_message_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE INDEX IF NOT EXISTS idx_messages_session_run_sequence
            ON messages(session_id, execution_run_id, session_sequence, id);
        CREATE INDEX IF NOT EXISTS idx_messages_unfinished_session_sequence
            ON messages(session_id, session_sequence, id, execution_run_id)
            WHERE execution_run_id IS NOT NULL AND status IN ('pending', 'streaming');
        DROP INDEX IF EXISTS idx_sessions_recovery_candidates;
        CREATE INDEX IF NOT EXISTS idx_sessions_pending_recovery_id
            ON sessions(id)
            WHERE archived = 0
              AND recovery_status NOT IN ('action_required', 'quarantined')
              AND (
                active_execution_run_id IS NOT NULL
                OR lifecycle_state IN ('starting', 'running')
                OR recovery_status = 'reconciling'
              );
        "#,
    )?;
    Ok(())
}

pub(super) fn apply_context_quality_history_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE context_quality_assessments (
            attempt_id TEXT PRIMARY KEY NOT NULL,
            session_correlation TEXT,
            decision_sequence INTEGER NOT NULL CHECK (decision_sequence >= 0),
            recorded_at TEXT NOT NULL,
            outcome TEXT NOT NULL CHECK (outcome IN ('compacted', 'bypassed', 'fallback', 'failed')),
            path TEXT CHECK (path IS NULL OR path IN ('optimizer', 'compatibility')),
            reason TEXT,
            trigger_source TEXT CHECK (
                trigger_source IS NULL OR trigger_source IN ('token-aware', 'character-fallback')
            ),
            before_characters INTEGER NOT NULL CHECK (before_characters >= 0),
            after_characters INTEGER NOT NULL CHECK (after_characters >= 0),
            saved_characters INTEGER NOT NULL CHECK (saved_characters >= 0),
            before_tokens INTEGER CHECK (before_tokens IS NULL OR before_tokens >= 0),
            after_tokens INTEGER CHECK (after_tokens IS NULL OR after_tokens >= 0),
            saved_tokens INTEGER CHECK (saved_tokens IS NULL OR saved_tokens >= 0),
            measurement_quality TEXT NOT NULL CHECK (
                measurement_quality IN (
                    'reported', 'reported-plus-estimated-delta', 'estimated', 'characters-only'
                )
            ),
            protocol_complete INTEGER CHECK (protocol_complete IS NULL OR protocol_complete IN (0, 1)),
            protected_retained INTEGER CHECK (protected_retained IS NULL OR protected_retained IN (0, 1)),
            verbatim_retained INTEGER CHECK (verbatim_retained IS NULL OR verbatim_retained IN (0, 1)),
            reinjection_complete INTEGER CHECK (reinjection_complete IS NULL OR reinjection_complete IN (0, 1)),
            assessment_version TEXT NOT NULL,
            context_policy_version TEXT NOT NULL,
            optimizer_version TEXT NOT NULL,
            verifier_version TEXT NOT NULL
        );
        CREATE INDEX context_quality_assessments_recorded_at_idx
            ON context_quality_assessments(recorded_at DESC, attempt_id DESC);
        CREATE INDEX context_quality_assessments_session_idx
            ON context_quality_assessments(session_correlation, recorded_at DESC);
        CREATE INDEX context_quality_assessments_outcome_idx
            ON context_quality_assessments(outcome, recorded_at DESC);
        CREATE INDEX context_quality_assessments_policy_idx
            ON context_quality_assessments(context_policy_version, recorded_at DESC);
        "#,
    )?;
    Ok(())
}

pub(super) fn apply_session_execution_policy_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    conn.execute("UPDATE sessions SET chat_preferences = NULL", [])?;
    conn.execute(
        "DELETE FROM cli_parameter_settings WHERE \
         (agent_id = 'claude-code' AND parameter_id = 'permissionMode') OR \
         (agent_id = 'codex-cli' AND parameter_id IN ('sandbox', 'approvalPolicy')) OR \
         (agent_id = 'gemini-cli' AND parameter_id IN ('approvalMode', 'sandbox')) OR \
         (agent_id = 'opencode' AND parameter_id IN ('agent', 'autoApprove')) OR \
         (agent_id = 'antigravity-cli' AND parameter_id IN ('mode', 'sandbox'))",
        [],
    )?;
    Ok(())
}

pub(super) fn apply_operation_recovery_evidence_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS operation_recovery_evidence (
            operation_id TEXT PRIMARY KEY,
            execution_run_id TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN (
                'queued', 'running', 'succeeded', 'failed', 'cancelled'
            )),
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_operation_recovery_evidence_run
            ON operation_recovery_evidence(execution_run_id, updated_at, operation_id);
        "#,
    )?;
    Ok(())
}

pub(super) fn apply_session_recovery_foundation_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    let session_columns = [
        (
            "recovery_status",
            "TEXT NOT NULL DEFAULT 'clean' CHECK (recovery_status IN ('clean', 'reconciling', 'action_required', 'quarantined'))",
        ),
        (
            "recovery_revision",
            "INTEGER NOT NULL DEFAULT 0 CHECK (recovery_revision >= 0)",
        ),
        (
            "state_revision",
            "INTEGER NOT NULL DEFAULT 0 CHECK (state_revision >= 0)",
        ),
        (
            "history_revision",
            "INTEGER NOT NULL DEFAULT 0 CHECK (history_revision >= 0)",
        ),
        ("active_execution_run_id", "TEXT"),
        (
            "next_message_sequence",
            "INTEGER NOT NULL DEFAULT 1 CHECK (next_message_sequence > 0)",
        ),
    ];
    for (column, declaration) in session_columns {
        if !table_has_column(conn, "sessions", column)? {
            conn.execute(
                &format!("ALTER TABLE sessions ADD COLUMN {column} {declaration}"),
                [],
            )?;
        }
    }

    if !table_has_column(conn, "messages", "session_sequence")? {
        conn.execute(
            "ALTER TABLE messages ADD COLUMN session_sequence INTEGER NOT NULL DEFAULT 0 CHECK (session_sequence >= 0)",
            [],
        )?;
    }
    if !table_has_column(conn, "messages", "execution_run_id")? {
        conn.execute("ALTER TABLE messages ADD COLUMN execution_run_id TEXT", [])?;
    }
    if !table_has_column(conn, "messages", "seat_round_id")? {
        conn.execute("ALTER TABLE messages ADD COLUMN seat_round_id TEXT", [])?;
    }
    if !table_has_column(conn, "messages", "parent_execution_run_id")? {
        conn.execute(
            "ALTER TABLE messages ADD COLUMN parent_execution_run_id TEXT",
            [],
        )?;
    }

    conn.execute_batch(
        r#"
        WITH ranked_messages AS (
            SELECT id,
                   ROW_NUMBER() OVER (
                       PARTITION BY session_id
                       ORDER BY created_at ASC, id ASC
                   ) AS assigned_sequence
            FROM messages
        )
        UPDATE messages
        SET session_sequence = (
            SELECT assigned_sequence
            FROM ranked_messages
            WHERE ranked_messages.id = messages.id
        );

        UPDATE sessions
        SET next_message_sequence = COALESCE(
            (
                SELECT MAX(messages.session_sequence) + 1
                FROM messages
                WHERE messages.session_id = sessions.id
            ),
            1
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_session_sequence
            ON messages(session_id, session_sequence);
        CREATE INDEX IF NOT EXISTS idx_messages_execution_run
            ON messages(execution_run_id);
        CREATE INDEX IF NOT EXISTS idx_messages_seat_round
            ON messages(session_id, seat_round_id, session_sequence);
        CREATE INDEX IF NOT EXISTS idx_sessions_recovery_candidates
            ON sessions(recovery_status, lifecycle_state, active_execution_run_id);

        CREATE TABLE IF NOT EXISTS session_recovery_reports (
            report_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            recovery_revision INTEGER NOT NULL CHECK (recovery_revision > 0),
            trigger TEXT NOT NULL CHECK (trigger IN ('startup', 'explicit_retry', 'user_acknowledgement')),
            observed_lifecycle TEXT NOT NULL,
            observed_execution_run_id TEXT,
            decision TEXT NOT NULL CHECK (decision IN (
                'completed',
                'failed',
                'cancelled',
                'interrupted_without_tool_ambiguity',
                'action_required',
                'quarantined',
                'retry_later',
                'acknowledged'
            )),
            reason_codes_json TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE (session_id, recovery_revision),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_session_recovery_reports_session_created
            ON session_recovery_reports(session_id, created_at DESC, report_id DESC);
        "#,
    )?;
    Ok(())
}

// Versions 49-51 existed in two histories: Plan execution on main, or workspace code indexing in
// a concurrent worktree. Version 53 is unclaimed by both and makes their idempotent schemas meet.
pub(super) fn apply_plan_and_code_index_reconciliation(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    super::super::legacy_plan_schema::apply_legacy_plan_schema(conn)?;
    crate::contexts::retrieval::infrastructure::apply_code_index_schema(conn)
}

/// Version 27 created the multi-Agent coordination table. The capability is retired, so the slot
/// is kept as a no-op rather than deleted: the migration sequence is asserted to be dense by the
/// fixture tests, and leaving a permanent hole would make every future migration carry the gap.
pub(super) fn apply_retired_coordination_schema(_conn: &Connection) -> Result<(), DatabaseError> {
    Ok(())
}

/// Drops what version 27 left behind on installs that actually ran it. On a fresh database the
/// table was never created and this is a no-op.
///
/// Numbered 43, not 42: the concurrently-developed `permissions-core` branch already claimed 42
/// and has run it on shared local databases (every worktree shares one `ai.vanehub.app` database),
/// so reusing 42 would leave this migration permanently skipped there and the table undropped.
pub(super) fn apply_remove_coordination_migration(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch("DROP TABLE IF EXISTS coordination_runs;")?;
    Ok(())
}

pub(super) fn apply_mcp_truthful_url_transport_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
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

pub(super) fn apply_session_message_search_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
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

pub(super) fn apply_scheduled_task_management_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
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

pub(super) fn apply_cli_parameter_settings_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    cli_parameters::apply_schema(conn).map_err(|error| DatabaseError::Storage(error.to_string()))
}

pub(super) fn apply_remote_workspace_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

pub(super) fn apply_ssh_connection_management_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
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

pub(super) fn apply_session_management_organization_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
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

pub(super) fn apply_project_worktree_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

pub(super) fn apply_session_runtime_metadata_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "sessions", "runtime_session_id")? {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN runtime_session_id TEXT",
            [],
        )?;
    }
    Ok(())
}

pub(super) fn apply_cli_tool_status_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

pub(super) fn apply_cli_environment_details_migration(
    conn: &Connection,
) -> Result<(), DatabaseError> {
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

pub(super) fn apply_message_rich_blocks_migration(conn: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "messages", "rich_blocks")? {
        conn.execute("ALTER TABLE messages ADD COLUMN rich_blocks TEXT", [])?;
    }
    Ok(())
}

pub(super) fn apply_app_settings_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

pub(super) fn apply_chat_messages_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

pub(super) fn apply_initial_schema(conn: &Connection) -> Result<(), DatabaseError> {
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

pub(super) fn apply_agent_sdk_dependency_migration(conn: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "agents", "managed_sdk_dependency_id")? {
        conn.execute(
            "ALTER TABLE agents ADD COLUMN managed_sdk_dependency_id TEXT",
            [],
        )?;
    }
    Ok(())
}

pub(super) fn apply_session_management_migration(conn: &Connection) -> Result<(), DatabaseError> {
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
