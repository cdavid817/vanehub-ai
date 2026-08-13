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
        super::legacy_plan_schema::apply_legacy_plan_schema,
    )?;
    apply_migration(
        conn,
        50,
        "workspace-code-index-foundation",
        crate::contexts::retrieval::infrastructure::apply_code_index_schema,
    )?;
    apply_migration(
        conn,
        51,
        "workspace-code-index-mode",
        crate::contexts::retrieval::infrastructure::apply_code_index_mode_schema,
    )?;
    apply_migration(
        conn,
        52,
        "automatic-code-index-mode",
        crate::contexts::retrieval::infrastructure::apply_code_index_automatic_mode_schema,
    )?;
    apply_migration(
        conn,
        53,
        "plan-and-code-index-reconciliation",
        apply_plan_and_code_index_reconciliation,
    )?;
    apply_migration(conn, 54, "loop-evidence-iteration-index", |connection| {
        connection.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_loop_evidence_iteration_created
                ON loop_evidence(iteration_id, created_at);
            "#,
        )?;
        Ok(())
    })?;
    apply_transactional_migration(
        conn,
        55,
        "session-recovery-evidence-foundation",
        apply_session_recovery_foundation_migration,
    )?;
    apply_migration(
        conn,
        56,
        "operation-recovery-evidence",
        apply_operation_recovery_evidence_migration,
    )?;
    apply_transactional_migration(
        conn,
        57,
        "session-recovery-performance-hardening",
        apply_session_recovery_performance_migration,
    )?;
    apply_migration(
        conn,
        58,
        "lsp-code-intelligence-foundation",
        crate::contexts::code_intelligence::api::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        59,
        "stable-session-participants",
        crate::contexts::sessions::infrastructure::apply_stable_participant_schema,
    )?;
    apply_transactional_migration(
        conn,
        60,
        "effective-skill-runtime",
        crate::contexts::tooling::skills::infrastructure::apply_effective_runtime_schema,
    )?;
    apply_transactional_migration(
        conn,
        61,
        "session-execution-policy",
        apply_session_execution_policy_migration,
    )?;
    apply_migration(
        conn,
        62,
        "onepiece-plan-agent-loop",
        crate::contexts::task_orchestration::infrastructure::apply_plan_agent_loop_schema,
    )?;
    apply_migration(
        conn,
        63,
        "plan-session-association",
        crate::contexts::task_orchestration::infrastructure::apply_plan_session_association_schema,
    )?;
    apply_transactional_migration(
        conn,
        64,
        "fine-grained-token-accounting",
        crate::contexts::sessions::infrastructure::apply_usage_accounting_schema,
    )?;
    apply_migration(
        conn,
        65,
        "managed-im-session-bindings",
        crate::contexts::communications::infrastructure::apply_session_binding_schema,
    )?;
    apply_migration(
        conn,
        66,
        "unified-todo-board",
        crate::contexts::work_board::infrastructure::apply_schema,
    )?;
    repair_missing_stable_participant_schema(conn)?;

    // Fail fast when a migration was skipped or the persisted history contains a gap.
    assert_migration_history_is_dense(conn)?;

    Ok(())
}

fn apply_session_recovery_performance_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

fn apply_session_execution_policy_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

fn apply_operation_recovery_evidence_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

fn apply_session_recovery_foundation_migration(conn: &Connection) -> Result<(), DatabaseError> {
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

/// `(version, name)` for every migration `migrate` records, in order. This is the ground
/// truth the post-migration density check compares `schema_migrations` against, so a
/// version-number collision (two migrations claiming the same number — the second is
/// silently skipped because `apply_migration` is version-gated, leaving its table missing)
/// surfaces at startup instead of as an opaque "no such table" crash. This has already
/// happened across shared local databases (every worktree shares one `ai.vanehub.app`
/// database). Keep this in lockstep with the `apply_migration` / `apply_transactional_migration`
/// calls in `migrate` — the `migration_sequence_is_dense_and_matches_expected` test guards
/// against drift.
const EXPECTED_MIGRATIONS: &[(i64, &str)] = &[
    (1, "initial-schema"),
    (2, "agent-managed-sdk-dependency"),
    (3, "session-management"),
    (4, "chat-messages"),
    (5, "app-settings"),
    (6, "cli-tool-status"),
    (7, "skill-management"),
    (8, "project-worktree-management"),
    (9, "session-runtime-metadata"),
    (10, "im-connectors"),
    (11, "im-session-source"),
    (12, "cli-parameter-settings"),
    (13, "session-chat-configuration"),
    (14, "floating-assistant-configuration"),
    (15, "local-extension-management"),
    (16, "cli-local-environment-details"),
    (17, "message-rich-blocks"),
    (18, "session-management-organization"),
    (19, "prompt-hook-management"),
    (20, "remote-workspace-sessions"),
    (21, "sdk-operation-logs"),
    (22, "session-usage-records"),
    (23, "scheduled-task-management"),
    (24, "ssh-connection-management"),
    (25, "loop-engineering-runtime"),
    (26, "agent-execution-observability"),
    (27, "multi-agent-coordination"),
    (28, "remote-terminal-management"),
    (29, "api-agent-registration"),
    (30, "openai-compatible-agent-registration"),
    (31, "agent-cross-session-memory"),
    (32, "agent-tool-trust"),
    (33, "session-message-search-index"),
    (34, "cli-agent-global-config"),
    (35, "cli-agent-applied-ownership-snapshot"),
    (36, "mcp-truthful-url-transports"),
    (37, "skill-management-reliability"),
    (38, "agent-management-origin"),
    (39, "onepiece-provider-profiles"),
    (40, "onepiece-provider-catalog"),
    (41, "onepiece-provider-endpoints"),
    (42, "agent-memory-shared-pool"),
    (43, "retrieval-vector-index"),
    (44, "permissions-core"),
    (45, "remove-multi-agent-coordination"),
    (46, "expert-role-management"),
    (47, "session-seats"),
    (48, "message-speaker"),
    (49, "plan-execution-foundation"),
    (50, "workspace-code-index-foundation"),
    (51, "workspace-code-index-mode"),
    (52, "automatic-code-index-mode"),
    (53, "plan-and-code-index-reconciliation"),
    (54, "loop-evidence-iteration-index"),
    (55, "session-recovery-evidence-foundation"),
    (56, "operation-recovery-evidence"),
    (57, "session-recovery-performance-hardening"),
    (58, "lsp-code-intelligence-foundation"),
    (59, "stable-session-participants"),
    (60, "effective-skill-runtime"),
    (61, "session-execution-policy"),
    (62, "onepiece-plan-agent-loop"),
    (63, "plan-session-association"),
    (64, "fine-grained-token-accounting"),
    (65, "managed-im-session-bindings"),
    (66, "unified-todo-board"),
];

fn assert_migration_history_is_dense(conn: &Connection) -> Result<(), DatabaseError> {
    // Density + upper-bound check only. A version-number *collision* (two migrations
    // claiming the same number) does not create a gap — the second is silently skipped
    // and the first's row fills the version — so name divergence is the only signal.
    // That is asserted in tests (`migration_sequence_matches_expected`), not at startup,
    // because a shared local database already in a collided state would otherwise become
    // unbootable here, which is worse than the missing-table crash it would hit later.
    let max_expected = EXPECTED_MIGRATIONS
        .iter()
        .map(|(version, _)| *version)
        .max()
        .unwrap_or(0);
    let mut rows = conn.prepare("SELECT version FROM schema_migrations ORDER BY version ASC")?;
    let versions: Vec<i64> = rows
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut prev: Option<i64> = None;
    for version in &versions {
        if let Some(p) = prev {
            if *version != p + 1 {
                return Err(DatabaseError::Storage(format!(
                    "migration history is not dense: version {p} is followed by {version} \
                     (a migration did not record its schema_migrations row — the schema and the \
                     version table have diverged)"
                )));
            }
        }
        if *version > max_expected {
            return Err(DatabaseError::Storage(format!(
                "migration version {version} is recorded but exceeds the highest version \
                 ({max_expected}) this binary expects — an unknown migration is in the history"
            )));
        }
        prev = Some(*version);
    }
    Ok(())
}

/// Parallel development worktrees can share the application data directory while carrying
/// different migrations at version 54. The version gate then legitimately preserves the first
/// record, so enforce the required schema invariant without rewriting that database history.
fn repair_missing_stable_participant_schema(conn: &Connection) -> Result<(), DatabaseError> {
    if table_has_column(conn, "messages", "speaker_seat_id")? {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    crate::contexts::sessions::infrastructure::apply_stable_participant_schema(&transaction)?;
    transaction.commit()?;
    Ok(())
}

// Versions 49-51 existed in two histories: Plan execution on main, or workspace code indexing in
// a concurrent worktree. Version 53 is unclaimed by both and makes their idempotent schemas meet.
fn apply_plan_and_code_index_reconciliation(conn: &Connection) -> Result<(), DatabaseError> {
    super::legacy_plan_schema::apply_legacy_plan_schema(conn)?;
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

    // Wrap the schema change and the version-bookkeeping row in one transaction so a
    // mid-migration failure rolls back the DDL/DML that already landed. Without this,
    // SQLite auto-commits each DDL statement, leaving the schema partially applied
    // while `schema_migrations` never records the version — the next startup re-runs
    // the migration and relies on `IF NOT EXISTS` / `table_has_column` idempotency to
    // paper over it, which is not guaranteed for data-bearing migrations.
    let transaction = conn.unchecked_transaction()?;
    migration(&transaction)?;
    transaction.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        params![version, name],
    )?;
    transaction.commit()?;
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
        apply_session_recovery_performance_migration(&connection)
            .expect("recovery performance schema");

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
        apply_session_recovery_performance_migration(&connection)
            .expect("recovery performance schema");

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
        assert_eq!(migration_state, (65, 66));

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
}
