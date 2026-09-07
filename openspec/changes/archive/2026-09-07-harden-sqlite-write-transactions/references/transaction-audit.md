# 事务调用点审计清单

## 实施结果(2026-09-07,任务 1.1 人工确认后)

- 生产代码 99 个调用点全部改为 `platform::database::SqliteWriteTransaction`:持 `&mut Connection` / `PooledSqlite` 的走 `write_transaction()`,持 `&Connection` 的(`skill_evolution_generation` retention 与 dossier、`skill_evolution_assessment` policy/supersession 等原 `unchecked_transaction` 点)走 `write_transaction_unchecked()`;此前已显式写成 `transaction_with_behavior(Immediate)` 的 56 处同样收口到入口,`TransactionBehavior` 导入随之移除。`skill_evolution_curation::SqliteCuratorRepository` 自带的事务包装方法同名改为 `write_transaction`,内部走入口。
- 保留 deferred 并登记进 `tests/architecture.rs` 允许清单的纯读快照共三处:`sessions::SqliteSessionRepository::read_terminal_evidence`(会话行 + 消息的一致视图)、`skill_evolution_curation::{draft_review_store::review_binding, preview_store::preview_binding}`(原代码已显式写 `Deferred`,只读)。启发式标为「疑似纯读」的 `work_board::reconcile` 实际经 `reconcile_query` 写入,已改为写事务;「待人工判断」四处中 `sessions::read_terminal_evidence` 纯读、其余三处均写。
- 目录级豁免:`platform/database/`(入口自身与启动期迁移)、`contexts/tooling/cli_parameters/legacy_baseline.rs`(仅 `cfg(test)` 编译)。
- 任务 1.2 的长扫描复核:`execution_observability::maintain_retention`(先读单行设置再 `DELETE`)、`workspaces::enforce_capacity`、`tooling/skills::reconcile_workspace_aliases`、各 `skill_evolution_*` retention 的读取部分都是按主键或索引的小范围查询,持写锁期间不做全表扫描,未移出事务。
- 下表为改造前的启发式初稿,保留作为审计轨迹;分类以本节为准。

生成方式:扫描 `src-tauri/src` 中全部 `.transaction()` / `unchecked_transaction()` 调用,向后读取至 `.commit()` 之前的语句,按第一条语句是读还是写做启发式分类;`read:`/`write:` 前缀表示依据的是帮助函数名而非 SQL 文本。这是任务 1.1 的起点,不是结论:每一行都要人工打开确认,尤其是「写优先」里经由帮助函数(`write:save_records` 等)可能内部先读的情况,以及「待人工判断」。

已在 `extend-cli-providers-with-acp` 与 `add-session-worktree-cleanup` 中改为 Immediate 的调用点不在此表(它们已使用 `transaction_with_behavior`),任务 3.1 复核时改为经由统一入口即可。

| 分类 | 数量 |
| --- | --- |
| 先读后写 | 25 |
| 写优先 | 69 |
| 疑似纯读 | 1 |
| 待人工判断 | 4 |
| 启动期迁移(登记豁免) | 0 |
| 测试代码(不改) | 24 |

| 分类 | 文件 | 行 | 函数 | 事务内第一条语句 |
| --- | --- | --- | --- | --- |
| 先读后写 | `contexts/agent_runtime/infrastructure/sqlite_repository.rs` | 389 | `delete` | `SELECT` |
| 先读后写 | `contexts/agent_runtime/infrastructure/sqlite_repository.rs` | 585 | `save_onepiece_provider_profile` | `SELECT` |
| 先读后写 | `contexts/agent_runtime/infrastructure/sqlite_repository.rs` | 628 | `activate_onepiece_provider_profile` | `SELECT` |
| 先读后写 | `contexts/agent_runtime/infrastructure/sqlite_repository.rs` | 663 | `delete_onepiece_provider_profile` | `SELECT` |
| 先读后写 | `contexts/communications/infrastructure/sqlite_repository.rs` | 377 | `consume_pairing_intent` | `SELECT` |
| 先读后写 | `contexts/execution_observability/infrastructure/retention.rs` | 27 | `maintain_retention` | `SELECT` |
| 先读后写 | `contexts/personalization/infrastructure/sqlite_policy_repository.rs` | 327 | `seed_default_global` | `read:load_by_scope_key` |
| 先读后写 | `contexts/sessions/infrastructure/transactions.rs` | 27 | `acknowledge_recovery` | `read:load_session` |
| 先读后写 | `contexts/skill_evolution_assessment/infrastructure/supersession_repository.rs` | 57 | `recheck` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_curation/infrastructure/repository.rs` | 100 | `transition_with_audit` | `read:current_candidate` |
| 先读后写 | `contexts/skill_evolution_curation/infrastructure/repository.rs` | 154 | `persist_decision` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_generation/api/queries.rs` | 174 | `regenerate` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/notification_read_repository.rs` | 36 | `open_notification_after_visible` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/notification_repository.rs` | 49 | `claim_due_digest_notifications` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/notification_repository.rs` | 144 | `persist_digest` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/rebuild_repository.rs` | 40 | `begin_rebuild` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/rebuild_repository.rs` | 96 | `advance_rebuild` | `read:load_rebuild` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/rebuild_repository.rs` | 196 | `activate_rebuild` | `read:load_rebuild` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/rebuild_repository.rs` | 260 | `cancel_rebuild` | `read:load_rebuild` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/rebuild_repository/lifecycle.rs` | 11 | `validate_rebuild` | `read:load_rebuild` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/retention_repository.rs` | 33 | `apply_detail_retention` | `SELECT` |
| 先读后写 | `contexts/skill_evolution_system_activity/infrastructure/timeline_repository.rs` | 24 | `deliver_timeline` | `read:load_envelope` |
| 先读后写 | `contexts/tooling/prompt_hooks/infrastructure/sqlite_repository.rs` | 348 | `publish_draft` | `SELECT` |
| 先读后写 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 1321 | `reconcile_workspace_aliases` | `SELECT` |
| 先读后写 | `contexts/workspaces/infrastructure/capture_maintenance.rs` | 56 | `enforce_capacity` | `SELECT` |
| 写优先 | `contexts/agent_runtime/infrastructure/context_manifest_repository.rs` | 50 | `save` | `INSERT` |
| 写优先 | `contexts/agent_runtime/infrastructure/context_quality_repository.rs` | 46 | `append_and_prune` | `INSERT` |
| 写优先 | `contexts/agent_runtime/infrastructure/loop_repository.rs` | 377 | `save_continue_transition` | `UPDATE` |
| 写优先 | `contexts/agent_runtime/infrastructure/loop_repository.rs` | 444 | `save_recovery_transition` | `UPDATE` |
| 写优先 | `contexts/agent_runtime/infrastructure/native_tool_repository.rs` | 88 | `insert_artifact` | `INSERT` |
| 写优先 | `contexts/agent_runtime/infrastructure/native_tool_repository.rs` | 186 | `insert_change_set` | `INSERT` |
| 写优先 | `contexts/agent_runtime/infrastructure/sqlite_repository.rs` | 825 | `replace_hybrid_routing_rules` | `DELETE` |
| 写优先 | `contexts/artifacts/infrastructure/sqlite_catalog.rs` | 22 | `insert_immutable` | `INSERT` |
| 写优先 | `contexts/code_intelligence/infrastructure/configuration_repository.rs` | 37 | `save_configuration` | `UPDATE` |
| 写优先 | `contexts/communications/infrastructure/sqlite_repository.rs` | 286 | `save_pairing_intent` | `DELETE` |
| 写优先 | `contexts/communications/infrastructure/sqlite_repository.rs` | 682 | `save_configuration` | `INSERT` |
| 写优先 | `contexts/communications/infrastructure/sqlite_repository.rs` | 734 | `delete_configuration` | `DELETE` |
| 写优先 | `contexts/desktop/infrastructure/sqlite_settings_repository.rs` | 45 | `save_folder_opener_preferences` | `write:upsert_setting` |
| 写优先 | `contexts/desktop/infrastructure/sqlite_settings_repository.rs` | 106 | `save_automatic_archival` | `write:upsert_setting` |
| 写优先 | `contexts/execution_observability/infrastructure/evaluation_repository.rs` | 33 | `save_terminal` | `INSERT` |
| 写优先 | `contexts/execution_observability/infrastructure/settings_repository.rs` | 57 | `update_settings` | `UPDATE` |
| 写优先 | `contexts/execution_observability/infrastructure/sqlite_repository.rs` | 34 | `insert_run` | `INSERT` |
| 写优先 | `contexts/execution_observability/infrastructure/sqlite_repository.rs` | 90 | `insert_span` | `INSERT` |
| 写优先 | `contexts/operations/infrastructure/log_index_repository.rs` | 153 | `insert` | `INSERT` |
| 写优先 | `contexts/operations/infrastructure/run_repository.rs` | 84 | `insert` | `INSERT` |
| 写优先 | `contexts/operations/infrastructure/run_repository.rs` | 123 | `save` | `UPDATE` |
| 写优先 | `contexts/permissions/infrastructure/resolution_repository.rs` | 173 | `commit_resolution` | `INSERT` |
| 写优先 | `contexts/permissions/infrastructure/resolution_repository.rs` | 266 | `acknowledge_delivery_and_activate` | `UPDATE` |
| 写优先 | `contexts/retrieval/infrastructure/code_index_repository.rs` | 182 | `invalidate_stale_version` | `DELETE` |
| 写优先 | `contexts/retrieval/infrastructure/code_index_repository.rs` | 228 | `save_configuration` | `UPDATE` |
| 写优先 | `contexts/retrieval/infrastructure/code_index_repository.rs` | 287 | `replace_file` | `DELETE` |
| 写优先 | `contexts/retrieval/infrastructure/code_index_repository.rs` | 468 | `rebuild_workspace` | `DELETE` |
| 写优先 | `contexts/retrieval/infrastructure/code_index_repository.rs` | 754 | `confirm_embedding` | `UPDATE` |
| 写优先 | `contexts/retrieval/infrastructure/code_index_repository.rs` | 814 | `record_audit` | `INSERT` |
| 写优先 | `contexts/retrieval/infrastructure/sqlite_repository.rs` | 75 | `reconcile_apply` | `INSERT` |
| 写优先 | `contexts/sessions/infrastructure/review_repository.rs` | 175 | `save` | `write:save_head` |
| 写优先 | `contexts/sessions/infrastructure/sqlite_repository.rs` | 591 | `insert` | `write:insert_message` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 123 | `start_generation` | `UPDATE` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 224 | `terminalize_generation` | `UPDATE` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 386 | `publish_recovery` | `UPDATE` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 499 | `create_session` | `write:insert_session` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 513 | `activate_session` | `write:update_active_workflow` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 524 | `archive_session` | `write:update_session_state` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 546 | `delete_session` | `DELETE` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 565 | `delete_category` | `UPDATE` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 595 | `complete_message` | `write:update_message` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 609 | `save_runtime_session` | `write:update_session_state` |
| 写优先 | `contexts/sessions/infrastructure/transactions.rs` | 626 | `cancel_messages` | `UPDATE` |
| 写优先 | `contexts/skill_evolution_assessment/infrastructure/assessment_repository.rs` | 72 | `persist_completed` | `INSERT` |
| 写优先 | `contexts/skill_evolution_assessment/infrastructure/assessment_repository.rs` | 160 | `complete_leased` | `UPDATE` |
| 写优先 | `contexts/skill_evolution_assessment/infrastructure/policy_repository.rs` | 65 | `update` | `UPDATE` |
| 写优先 | `contexts/skill_evolution_curation/infrastructure/repository.rs` | 22 | `insert_candidate` | `INSERT` |
| 写优先 | `contexts/skill_evolution_generation/infrastructure/dossier_repository.rs` | 24 | `persist` | `INSERT` |
| 写优先 | `contexts/skill_evolution_system_activity/infrastructure/preferences_repository.rs` | 35 | `update_preferences` | `write:persist_preferences` |
| 写优先 | `contexts/skill_evolution_system_activity/infrastructure/projection_batch_repository.rs` | 19 | `commit_projection_batch` | `write:persist_source_event` |
| 写优先 | `contexts/skill_evolution_system_activity/infrastructure/retention_repository.rs` | 99 | `apply_source_purge` | `INSERT` |
| 写优先 | `contexts/ssh_connections/infrastructure/sqlite_repository.rs` | 94 | `insert` | `write:insert_profile` |
| 写优先 | `contexts/ssh_connections/infrastructure/sqlite_repository.rs` | 103 | `update` | `UPDATE` |
| 写优先 | `contexts/tooling/cli/infrastructure/environment_repository.rs` | 458 | `create_bulk_plan_atomic` | `write:insert_bulk` |
| 写优先 | `contexts/tooling/extensions/infrastructure/sqlite_repository.rs` | 100 | `apply_enablement` | `UPDATE` |
| 写优先 | `contexts/tooling/prompt_hooks/infrastructure/sqlite_repository.rs` | 87 | `delete_user_hook` | `DELETE` |
| 写优先 | `contexts/tooling/prompt_hooks/infrastructure/sqlite_repository.rs` | 194 | `save_traces` | `write:insert_trace` |
| 写优先 | `contexts/tooling/prompt_hooks/infrastructure/sqlite_repository.rs` | 259 | `create_user_draft` | `write:insert_user_hook` |
| 写优先 | `contexts/tooling/prompt_hooks/infrastructure/sqlite_repository.rs` | 413 | `publish_rollback` | `write:insert_version` |
| 写优先 | `contexts/tooling/prompt_hooks/infrastructure/sqlite_repository.rs` | 425 | `save_execution_observations` | `INSERT` |
| 写优先 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 233 | `save_skills` | `write:save_records` |
| 写优先 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 246 | `delete_skill` | `INSERT` |
| 写优先 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 290 | `save_mount_path` | `INSERT` |
| 写优先 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 319 | `save_synchronization` | `write:save_records` |
| 写优先 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 392 | `save_builtin_reconciliation` | `write:save_record` |
| 写优先 | `contexts/tooling/skills/infrastructure/sqlite_repository.rs` | 411 | `complete_builtin_cleanup` | `UPDATE` |
| 写优先 | `contexts/work_board/api.rs` | 78 | `move_item` | `write:normalize_stage` |
| 写优先 | `contexts/workspaces/infrastructure/capture_maintenance.rs` | 20 | `purge_session` | `DELETE` |
| 写优先 | `contexts/workspaces/infrastructure/capture_maintenance.rs` | 38 | `purge_before` | `DELETE` |
| 疑似纯读 | `contexts/work_board/infrastructure.rs` | 106 | `reconcile` | `SELECT` |
| 待人工判断 | `contexts/sessions/infrastructure/sqlite_repository.rs` | 61 | `read_terminal_evidence` | `-` |
| 待人工判断 | `contexts/skill_evolution_generation/infrastructure/retention_repository.rs` | 36 | `purge_source_evidence` | `-` |
| 待人工判断 | `contexts/skill_evolution_generation/infrastructure/retention_repository.rs` | 58 | `apply_retention` | `-` |
| 待人工判断 | `contexts/skill_evolution_orchestration/infrastructure/notification_repository.rs` | 45 | `pending` | `-` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_lock_tests.rs` | 60 | `reading_a_source_file_does_not_wait_on_the_database_write_lock` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_lock_tests.rs` | 100 | `a_failed_insert_leaves_no_transaction_holding_the_write_lock` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_lock_tests.rs` | 137 | `a_query_releases_its_connection_before_returning_the_page` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_query_bounds_tests.rs` | 256 | `a_query_over_a_maximum_corpus_holds_no_write_lock_while_answering` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_query_bounds_tests.rs` | 289 | `an_exhausting_search_holds_no_write_lock_either` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_repair_store_tests.rs` | 70 | `assert_write_lock_is_free` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/log_index_repair_store_tests.rs` | 263 | `source_reading_and_parsing_happens_outside_any_write_transaction` | `INSERT` |
| 测试代码(不改) | `contexts/operations/infrastructure/run_repository.rs` | 378 | `run_schema_failure_rolls_back_and_legacy_tables_are_untouched` | `CREATE` |
| 测试代码(不改) | `contexts/operations/infrastructure/run_repository.rs` | 407 | `runner_projection_migration_rolls_back_all_ddl_on_failure` | `write:apply_runner_projection_schema` |
| 测试代码(不改) | `contexts/permissions/infrastructure/resolution_schema.rs` | 721 | `a_failed_invariant_check_leaves_the_pre_migration_table_intact` | `write:apply_grant_identity_and_resolution_ledger` |
| 测试代码(不改) | `contexts/permissions/infrastructure/resolution_schema.rs` | 800 | `a_failure_at_any_stage_of_the_rebuild_leaves_the_original_table_untouched` | `write:apply_grant_identity_and_resolution_ledger` |
| 测试代码(不改) | `contexts/personalization/infrastructure/schema_tests.rs` | 232 | `a_failing_migration_leaves_no_partial_schema_and_no_version_row` | `write:apply_schema` |
| 测试代码(不改) | `contexts/sessions/infrastructure/review_repository.rs` | 538 | `schema_migration_is_transactional_when_following_statement_fails` | `write:apply_schema` |
| 测试代码(不改) | `contexts/sessions/infrastructure/tests.rs` | 279 | `message_sequence_allocator_reserves_consecutive_ranges_atomically` | `-` |
| 测试代码(不改) | `contexts/skill_evolution_curation/infrastructure/notification_receipts_tests.rs` | 82 | `queue` | `-` |
| 测试代码(不改) | `contexts/skill_evolution_evidence/infrastructure/tests.rs` | 125 | `failed_migration_transaction_preserves_the_preexisting_schema` | `write:apply_schema` |
| 测试代码(不改) | `contexts/skill_evolution_system_activity/infrastructure/tests.rs` | 156 | `source_outboxes_commit_atomically_and_reject_non_owned_domains` | `UPDATE` |
| 测试代码(不改) | `contexts/tooling/cli_parameters/legacy_baseline.rs` | 784 | `save_profile_to_conn` | `DELETE` |
| 测试代码(不改) | `contexts/workspaces/infrastructure/output_search.rs` | 90 | `performance_long_terminal_search_is_indexed_page_bounded_and_content_free` | `INSERT` |
| 测试代码(不改) | `platform/database/migrations/mod.rs` | 914 | `repair_missing_stable_participant_schema` | `write:apply_stable_participant_schema` |
| 测试代码(不改) | `platform/database/migrations/mod.rs` | 936 | `repair_missing_cli_parameter_profile_schema` | `write:apply_schema` |
| 测试代码(不改) | `platform/database/migrations/mod.rs` | 966 | `apply_migration` | `INSERT` |
| 测试代码(不改) | `platform/database/migrations/mod.rs` | 994 | `apply_transactional_migration` | `INSERT` |
| 测试代码(不改) | `platform/database/migrations/tests.rs` | 497 | `mcp_transport_journal_supports_a_targeted_down_migration` | `UPDATE` |
