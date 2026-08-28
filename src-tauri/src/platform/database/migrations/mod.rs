mod inline_schema;
#[cfg(test)]
mod tests;

use super::DatabaseError;
use inline_schema::{
    apply_agent_sdk_dependency_migration, apply_app_settings_migration,
    apply_chat_messages_migration, apply_cli_environment_details_migration,
    apply_cli_parameter_settings_migration, apply_cli_tool_status_migration,
    apply_context_quality_history_migration, apply_initial_schema,
    apply_mcp_truthful_url_transport_migration, apply_message_rich_blocks_migration,
    apply_operation_recovery_evidence_migration, apply_plan_and_code_index_reconciliation,
    apply_project_worktree_migration, apply_remote_workspace_migration,
    apply_remove_coordination_migration, apply_retired_coordination_schema,
    apply_scheduled_task_management_migration, apply_session_execution_policy_migration,
    apply_session_management_migration, apply_session_management_organization_migration,
    apply_session_message_search_migration, apply_session_recovery_foundation_migration,
    apply_session_recovery_performance_migration, apply_session_runtime_metadata_migration,
    apply_ssh_connection_management_migration,
};
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
        crate::platform::legacy_plan_schema::apply_legacy_plan_schema,
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
        crate::platform::legacy_plan_schema::apply_legacy_plan_agent_loop_schema,
    )?;
    apply_migration(
        conn,
        63,
        "plan-session-association",
        crate::platform::legacy_plan_schema::apply_legacy_plan_session_association_schema,
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
    apply_transactional_migration(
        conn,
        67,
        "skill-evolution-evidence-foundation",
        crate::contexts::skill_evolution_evidence::infrastructure::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        68,
        "onepiece-native-tool-persistence",
        crate::contexts::agent_runtime::infrastructure::apply_native_tool_schema,
    )?;
    apply_transactional_migration(
        conn,
        69,
        "onepiece-artifact-catalog-metadata",
        crate::contexts::artifacts::infrastructure::apply_artifact_catalog_schema,
    )?;
    apply_transactional_migration(
        conn,
        70,
        "onepiece-context-quality-history",
        apply_context_quality_history_migration,
    )?;
    apply_transactional_migration(
        conn,
        71,
        "goal-management",
        crate::contexts::goals::infrastructure::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        72,
        "skill-configuration-records",
        crate::contexts::tooling::skills::infrastructure::apply_skill_configuration_schema,
    )?;
    apply_transactional_migration(
        conn,
        73,
        "skill-tool-runtime-foundation",
        crate::contexts::tooling::skill_tools::infrastructure::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        74,
        "agent-context-evidence-manifests",
        crate::contexts::agent_runtime::infrastructure::apply_context_manifest_schema,
    )?;
    apply_transactional_migration(
        conn,
        75,
        "agent-code-review",
        crate::contexts::sessions::infrastructure::apply_review_schema,
    )?;
    apply_transactional_migration(
        conn,
        76,
        "canonical-agent-run-state",
        crate::contexts::operations::infrastructure::apply_run_schema,
    )?;
    apply_transactional_migration(
        conn,
        77,
        "agent-evaluation-platform",
        crate::contexts::execution_observability::infrastructure::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        78,
        "hybrid-local-model-runtime",
        crate::contexts::agent_runtime::infrastructure::apply_hybrid_local_model_schema,
    )?;
    apply_transactional_migration(
        conn,
        79,
        "agent-runner-projections",
        crate::contexts::operations::infrastructure::apply_runner_projection_schema,
    )?;
    apply_transactional_migration(
        conn,
        80,
        "retire-plan-execution",
        crate::platform::legacy_plan_schema::apply_retire_plan_execution_migration,
    )?;
    apply_transactional_migration(
        conn,
        81,
        "cli-parameter-profiles",
        crate::contexts::tooling::cli_parameters::infrastructure::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        82,
        "local-media-profiles",
        crate::contexts::local_media::infrastructure::apply_schema,
    )?;
    apply_transactional_migration(
        conn,
        83,
        "cli-environment-snapshots",
        crate::contexts::tooling::cli::infrastructure::environment_schema::apply_environment_snapshot_schema,
    )?;
    apply_transactional_migration(
        conn,
        84,
        "cli-version-catalogs",
        crate::contexts::tooling::cli::infrastructure::environment_schema::apply_version_catalog_schema,
    )?;
    apply_transactional_migration(
        conn,
        85,
        "cli-action-plans",
        crate::contexts::tooling::cli::infrastructure::environment_schema::apply_action_plan_schema,
    )?;
    apply_transactional_migration(
        conn,
        86,
        "lsp-language-registry",
        crate::contexts::code_intelligence::api::apply_language_registry_schema,
    )?;
    apply_transactional_migration(
        conn,
        87,
        "im-session-connector-access",
        crate::contexts::communications::infrastructure::apply_session_connector_access_schema,
    )?;
    // Renumbered above main's ceiling on merge, exactly as the pre-merge note said they would be.
    // Each still carries a repair: `apply_migration` is version-gated, and a database another
    // worktree already migrated arrives with the history complete and these tables absent, at which
    // point the gated call never runs. The repair re-asserts the schema rather than leaving a
    // database whose history looks whole while its tables are missing.
    apply_migration(
        conn,
        88,
        "execution-evidence-journal",
        crate::contexts::execution_observability::infrastructure::apply_evidence_schema,
    )?;
    apply_migration(
        conn,
        89,
        "unified-log-query-index",
        crate::contexts::operations::infrastructure::apply_log_query_index_schema,
    )?;
    apply_migration(
        conn,
        90,
        "review-decision-state",
        crate::contexts::sessions::infrastructure::apply_review_decision_schema,
    )?;
    apply_migration(
        conn,
        91,
        "review-file-viewed-witness",
        crate::contexts::sessions::infrastructure::apply_review_file_witness_schema,
    )?;
    repair_missing_stable_participant_schema(conn)?;
    repair_missing_cli_parameter_profile_schema(conn)?;
    crate::contexts::execution_observability::infrastructure::repair_missing_evidence_schema(conn)?;
    crate::contexts::operations::infrastructure::repair_missing_log_query_index_schema(conn)?;
    crate::contexts::sessions::infrastructure::repair_missing_review_decision_schema(conn)?;
    crate::contexts::sessions::infrastructure::repair_missing_review_file_witness(conn)?;

    // Fail fast when a migration was skipped or the persisted history contains a gap.
    assert_migration_history_is_dense(conn)?;
    Ok(())
}

/// `(version, name)` for every migration `migrate` records, in order. This is the ground
/// truth the post-migration density check compares `schema_migrations` against, so a
/// version-number collision (two migrations claiming the same number — the second is
/// silently skipped because `apply_migration` is version-gated, leaving its table missing)
/// surfaces at startup instead of as an opaque "no such table" crash. This has already
/// happened across shared local databases (every worktree shares one `ai.vanehub.app`
/// database). Keep this in lockstep with the `apply_migration` / `apply_transactional_migration`
/// calls in `migrate` — the `migration_sequence_matches_expected` test guards against drift,
/// and `assert_migration_history_is_dense` rejects a gapped history at startup.
/// Every migration version, in order.
///
/// Exposed so tests can derive their expectations instead of hardcoding an upper bound that every
/// new migration invalidates. Test-only: production reads `EXPECTED_MIGRATIONS` directly.
#[cfg(test)]
pub(crate) fn expected_migration_versions() -> Vec<i64> {
    EXPECTED_MIGRATIONS
        .iter()
        .map(|(version, _)| *version)
        .collect()
}

// `pub(super)` because `platform::database::mod` derives its own migration-count assertions from
// this list rather than restating the number; the helper above covers callers outside that module.
pub(super) const EXPECTED_MIGRATIONS: &[(i64, &str)] = &[
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
    (67, "skill-evolution-evidence-foundation"),
    (68, "onepiece-native-tool-persistence"),
    (69, "onepiece-artifact-catalog-metadata"),
    (70, "onepiece-context-quality-history"),
    (71, "goal-management"),
    (72, "skill-configuration-records"),
    (73, "skill-tool-runtime-foundation"),
    (74, "agent-context-evidence-manifests"),
    (75, "agent-code-review"),
    (76, "canonical-agent-run-state"),
    (77, "agent-evaluation-platform"),
    (78, "hybrid-local-model-runtime"),
    (79, "agent-runner-projections"),
    (80, "retire-plan-execution"),
    (81, "cli-parameter-profiles"),
    (82, "local-media-profiles"),
    (83, "cli-environment-snapshots"),
    (84, "cli-version-catalogs"),
    (85, "cli-action-plans"),
    (86, "lsp-language-registry"),
    (87, "im-session-connector-access"),
    (88, "execution-evidence-journal"),
    (89, "unified-log-query-index"),
    (90, "review-decision-state"),
    (91, "review-file-viewed-witness"),
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

/// The same collision, one migration later: `add-local-composer-media-tools` and
/// `upgrade-cli-parameter-management` both reserved 81 while unmerged, and local media renumbered
/// to 82 once the CLI change landed first.
///
/// A database written by the unmerged branch records `(81, "local-media-profiles")`. On the merged
/// binary the version gate then skips 81 legitimately, 82 re-runs local media's idempotent schema,
/// and the history is still dense `1..82` — so nothing fails and `cli_parameter_profiles` is simply
/// never created. The next managed CLI launch dies in `resolve_launch_parameters` with an opaque
/// repository error, and a restart re-skips 81 forever. Names are not compared at startup by
/// design (see `assert_migration_history_is_dense`), so the invariant is enforced here instead of
/// by rewriting that database's history.
fn repair_missing_cli_parameter_profile_schema(conn: &Connection) -> Result<(), DatabaseError> {
    if table_has_column(conn, "cli_parameter_profiles", "agent_id")? {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    crate::contexts::tooling::cli_parameters::infrastructure::apply_schema(&transaction)?;
    transaction.commit()?;
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
