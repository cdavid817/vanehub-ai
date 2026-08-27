# VaneHub Native Architecture

This document records the implemented context map, module ownership, compatibility contracts, and architecture decisions for the Rust native runtime. Normative rules live in `openspec/project.md`.

## Context Map

| Context | Published responsibility | Upstream dependencies | Downstream consumers |
| --- | --- | --- | --- |
| `agent_runtime` | Agent catalog, workflow selection, readiness, provider invocation, generation lifecycle | `tooling` effective CLI/prompt configuration, `sessions` application API, `operations` ports | Tauri commands, `communications` inbound execution |
| `sessions` | Session/message/category/configuration lifecycle, export, maintenance, usage read model | `operations` ports, bounded `workspaces` file access | Tauri commands, `agent_runtime`, `communications` |
| `workspaces` | Projects, remote workspaces, worktrees, file/Git inspection, PTY shell | `operations` ports | Tauri commands, `sessions` bounded file reads |
| `tooling` | CLI, MCP, SDK, extension, plugin, Skill, and Prompt Hook subdomains | `operations` ports and platform adapters | Tauri commands, `agent_runtime` published configuration APIs |
| `communications` | IM configuration, credentials, transports, routing, authorization, delivery | `sessions` and `agent_runtime` published APIs, `operations` ports | Tauri commands and connector transports |
| `desktop` | Settings, paths, startup, proxy preferences, window/tray/floating lifecycle | `operations` ports and platform adapters | Tauri bootstrap and commands |
| `operations` | Observable tasks and unified diagnostic/operation logging contracts | Platform clock/id and unified-log implementation | Every context |

Cross-context calls are synchronous published application APIs by default. An explicit event is used only when a completed action has independently handled downstream reactions. No context may reach into another context's storage or infrastructure.

## Source Module Inventory

Every current Rust source module is covered below, either by an exact path or by a path group whose files share one owner and layer. Removed compatibility paths remain listed so they cannot be reintroduced accidentally.

| Current module | Current role | Target | Migration task |
| --- | --- | --- | --- |
| `main.rs` | Native binary entry point | Delegates only to the library `run()` entry | 11.1 |
| `lib.rs` | Module exposure and native entry delegation | Implemented 15-line delegation to `bootstrap::run()` | 11.4 |
| `bootstrap/runtime.rs` | Tauri builder, app-data resolution, explicit context assembly order, state registration, and background-job startup | Implemented composition root; an absolute `VANEHUB_APP_DATA_DIR` override supports isolated packaged-runtime verification while empty values preserve the platform default | 11.1 |
| `commands/registry.rs` | Complete command registration grouped by bounded context | Implemented auditable invoke handler | 11.2 |
| `contexts/tooling/cli_parameters.rs` | CLI parameter catalog, validation, persistence API, and launch argument projection | Implemented Tooling-owned API consumed by Sessions and Agent Runtime | 11.3 |
| `command_safety.rs` | Removed compatibility facade | Process construction and audit logging live in `platform::process` | 11.5 |
| `network_proxy.rs` | Removed compatibility facade | Proxy-aware clients and streams live in `platform::network` | 11.5 |
| `logging.rs` | Removed root implementation | Persistence, redaction, rotation, and active-directory state live in `platform::logging`; semantic ports live in `operations` | 11.3, 11.5 |
| `usage.rs` | Usage domain/read model, SQLite schema, aggregation helpers | `sessions::{domain,application,infrastructure}` | 8 |
| `session_configuration.rs` | Legacy SQLite/profile compatibility adapter delegating provider/model/permission/reasoning invariants to `sessions::domain` | Move persistence and use-case orchestration into sessions infrastructure/application; publish configuration for `agent_runtime` | 8.1-8.4, 9 |
| `session_tabs.rs` | Removed legacy session-root compatibility helper | SQLite workspace projections now live and are tested in workspace infrastructure | 7.9 |
| `shell.rs` | Removed mixed PTY manager, commands, rules, and DTOs | Migrated into workspace domain/application/infrastructure and command adapters | 7.8 |
| `commands/mod.rs` | Tauri command root | Context-grouped `commands` registry | 11.2 |
| `commands/session_tabs/*` | Removed legacy workspace query wrappers | Migrated to `commands/workspaces/*` | 7.7 |
| `commands/shell/*` | Removed legacy shell wrappers | Migrated to `commands/workspaces/shell_*.rs` | 7.8 |
| `tasks/*` | Removed task compatibility tree | Operation models/use cases live in `contexts/operations`; queries live in `commands/operations` | 11.5 |
| `contexts/tooling/mcp/domain` | MCP identities, configuration invariants, and connection semantics | Implemented MCP domain | 4.1 |
| `contexts/tooling/mcp/application` | MCP management/connection-test use cases and consuming-side ports | Implemented MCP application boundary | 4.2 |
| `contexts/tooling/mcp/infrastructure` | SQLite row mapping, rmcp process/network connection, operation/log/clock/path adapters | Implemented MCP infrastructure over platform and operations APIs | 4.3-4.4 |
| `contexts/tooling/mcp/api.rs` | Published in-process MCP application facade | Implemented tooling MCP API | 4.5 |
| `commands/tooling/mcp/*` | One-file Tauri handlers, transport DTOs, mapping, and background scheduling | Implemented MCP inbound interface | 4.5-4.6 |
| `bootstrap/mcp.rs` | Concrete MCP dependency assembly | Implemented composition root | 4.5 |
| `contexts/tooling/sdk/domain` | SDK identities, catalog, status/version/update rules, and lifecycle plans | Implemented SDK domain | 5.6 |
| `contexts/tooling/sdk/application` | SDK queries/mutations plus repository, package, operation, logging, and clock ports | Implemented SDK application boundary | 5.7 |
| `contexts/tooling/sdk/infrastructure` | Package-state reads, SQLite operation logs, bounded npm/process execution, operation tasks, unified logs, and clock adapters | Implemented SDK infrastructure over platform and operations APIs | 5.8 |
| `contexts/tooling/sdk/api.rs` | Published in-process SDK application facade and immutable contracts | Implemented tooling SDK API | 5.9 |
| `commands/tooling/sdk/*` | One-file Tauri handlers, transport DTOs, mapping, and background scheduling | Implemented SDK inbound interface | 5.9 |
| `bootstrap/sdk.rs` | Concrete SDK dependency assembly | Implemented composition root | 5.9 |
| `contexts/tooling/extensions/domain` | Allowlisted catalog, host compatibility, installation drift, health reconciliation, enablement, lifecycle, and removal invariants | Implemented extension domain | 6.1 |
| `contexts/tooling/extensions/application` | Overview/health/preview/operation use cases plus behavior-oriented repository, environment, installation, runtime, mutation, operation, logging, and clock ports | Implemented extension application boundary | 6.1 |
| `contexts/tooling/extensions/infrastructure` | SQLite state, managed installation paths, bounded explicit processes, owned loopback runtimes, operation tasks, unified logs, mutation locks, and clock adapters | Implemented extension infrastructure over platform and operations APIs | 6.2 |
| `contexts/tooling/extensions/api.rs` | Published in-process extension application facade and immutable contracts | Implemented tooling extension API | 6.2 |
| `commands/tooling/extensions/*` | One-file Tauri handlers, transport DTOs, mapping, and background scheduling | Implemented extension inbound interface | 6.2 |
| `bootstrap/extensions.rs` | Concrete extension dependency assembly with shared owned-runtime adapter | Implemented composition root | 6.2 |
| `contexts/tooling/plugin_integrations/domain` | Built-in identity/catalog, fixed readiness plan, lifecycle state, and authenticated/missing/error classification rules | Implemented plugin integration domain | 6.3 |
| `contexts/tooling/plugin_integrations/application` | Overview/readiness use cases plus external-tool, clock, and semantic logging ports | Implemented plugin integration application boundary | 6.3 |
| `contexts/tooling/plugin_integrations/infrastructure` | Bounded explicit GitHub CLI execution, safe failure classification, unified diagnostic logging, and clock adapters | Implemented plugin integration infrastructure over platform and operations APIs | 6.4 |
| `contexts/tooling/plugin_integrations/api.rs` | Published in-process plugin integration application facade and immutable contracts | Implemented tooling plugin integration API | 6.4 |
| `commands/tooling/plugin_integrations/*` | One-file Tauri handlers, transport DTOs, and explicit mapping | Implemented plugin integration inbound interface | 6.4 |
| `bootstrap/plugin_integrations.rs` | Concrete plugin integration dependency assembly | Implemented composition root | 6.4 |
| `contexts/tooling/skills/domain` | Scoped identity, validated metadata/source, six built-ins, bounded mount paths, binding/enablement plans, mutation policy, and drift classification | Implemented Skill domain | 6.5 |
| `contexts/tooling/skills/application` | Skill management, preview, import, mount migration, drift detection/synchronization, and workspace selection use cases over behavior-oriented repository, filesystem, clock, selection, and semantic logging ports | Implemented Skill application boundary with deterministic fake-port tests | 6.6 |
| `contexts/tooling/skills/infrastructure` | Transactional SQLite repository, bounded filesystem journal, live binding observation, workspace selection, clock, and unified diagnostic logging adapters | Implemented Skill outbound adapters | 6.7 |
| `contexts/tooling/skills/api.rs` | Published in-process Skill application facade and immutable contracts | Implemented Skill API | 6.7 |
| `commands/tooling/skills/*` | One-file Tauri handlers, transport DTOs, and explicit contract mapping for all Skill commands | Implemented Skill inbound interface | 6.7 |
| `bootstrap/skills.rs` | Concrete Skill dependency assembly | Implemented composition root | 6.7 |
| `contexts/tooling/prompt_hooks/domain` | Validated Hook identity/manifest, stable category/stage/source values, deterministic ordering, managed CLI bindings, pure template interpolation, seven built-ins, and immutable built-in mutation policy | Implemented Prompt Hook domain | 6.8 |
| `contexts/tooling/prompt_hooks/application` | Catalog/override/user-hook management, preview, effective-prompt assembly, bounded safe traces, and semantic diagnostics over repository, clock, trace-id, and logging ports | Implemented Prompt Hook application boundary with deterministic fake-port tests | 6.9 |
| `contexts/tooling/prompt_hooks/api.rs` | Published Prompt Hook facade including the immutable effective-prompt contract consumed by agent runtime | Implemented Prompt Hook API | 6.9-6.10 |
| `contexts/tooling/prompt_hooks/infrastructure` | Explicit SQLite row/domain repository plus system clock, UUID trace-id, and unified diagnostic logging adapters | Implemented Prompt Hook outbound adapters with transaction, migration, redaction, and mapping tests | 6.10 |
| `commands/tooling/prompt_hooks/*` | One-file Tauri handlers, transport DTOs, and explicit contract/error mapping for all Prompt Hook commands | Implemented Prompt Hook inbound interface | 6.10 |
| `bootstrap/prompt_hooks.rs` | Concrete Prompt Hook dependency assembly injected into Tauri state and agent-runtime callers | Implemented composition root | 6.10 |
| `contexts/desktop/domain` | Strong settings types plus floating-assistant platform enablement, anchor validation, monitor placement, surface transition, and close-visibility rules | Implemented desktop domain | 7.1, 7.3 |
| `contexts/desktop/application` | Settings/environment, floating-assistant, tray initialization, and graceful-exit use cases over context-owned repository, window, lifecycle, shutdown, clock, and platform action ports | Implemented desktop application boundary with deterministic fake-port tests | 7.1-7.4 |
| `contexts/desktop/api.rs` | Published in-process settings/environment, floating-assistant, and lifecycle facades with immutable contracts | Implemented desktop APIs consumed only by commands, bootstrap, and lifecycle edge | 7.1-7.4 |
| `contexts/desktop/infrastructure` | SQLite settings/floating repositories; Tauri window/tray/lifecycle; clock, proxy, log-directory, autostart, directory, Node process, proxy action, and unified log adapters | Implemented desktop outbound adapters with persistence, lifecycle, timeout, mapping, and redaction tests | 7.2-7.4 |
| `commands/desktop/*` | One-file Tauri handlers, transport DTOs, settings/floating events, and explicit contract/error mapping for all desktop commands | Implemented desktop inbound interface | 7.2, 7.4 |
| `bootstrap/desktop.rs` | Concrete desktop settings, floating window, lifecycle, unified logging, and Communications API shutdown assembly injected into Tauri state | Implemented desktop composition root and startup edge | 7.2, 7.4 |
| `contexts/workspaces/domain` | Project/remote/worktree/path rules plus bounded terminal dimensions and platform-safe workspace reset commands | Implemented and covered by pure workspace domain tests | 7.5, 7.8-7.9 |
| `contexts/workspaces/application` | Project/history/worktree, bounded query, and shell lifecycle use cases over context-owned ports | Implemented workspace application boundary with deterministic fake-port tests | 7.6-7.9 |
| `contexts/workspaces/infrastructure` | Existing-table SQLite projections, bounded filesystem/Git/log queries, portable-PTY lifecycle, Tauri dialogs/events, clock/ids, and unified diagnostics | Implemented and covered by SQLite, Git, filesystem, traversal, PTY, event, and logging adapter tests | 7.6-7.9 |
| `contexts/workspaces/api.rs` | Published in-process workspace facade used by commands, production sessions/chat file reads, and session cleanup | Implemented workspace application API | 7.6-7.9 |
| `commands/workspaces/*` | One-file project/query/shell handlers with explicit camel-case DTO and lowercase-enum mapping | Implemented workspace inbound interface with DTO, error, and registration contract tests | 7.6-7.9 |
| `bootstrap/workspaces.rs` | Concrete workspace project/query/shell dependency assembly injected as one `WorkspaceApi` state | Implemented workspace composition root | 7.6-7.9 |
| `contexts/sessions/domain` | Session/message/category identities and aggregates, ownership/activation, lifecycle/pin/archive rules, bounded file references, and chat configuration invariants | Implemented pure sessions domain and wired legacy compatibility paths through it | 8.1 |
| `contexts/sessions/application` | Session creation/management, query/search, category/configuration, message/file-reference, export, maintenance, and usage use cases over context-owned persistence, transaction, clock, id, creation-context, runtime-cleanup, file-content, operation, profile, and logging ports | Implemented sessions application boundary with deterministic fake-port and atomic-coordination tests; bounded query, stable export filename, configuration validation, and message failure contracts are covered, with the superseded creation facade removed | 8.2-8.8 |
| `contexts/sessions/infrastructure` | Explicit session/message/category/configuration/usage SQLite row mapping, context-owned configuration and v22 usage schemas, multi-table transaction coordination, workspace creation, runtime cleanup, CLI-profile defaults, operation task, file, clock/id, and unified-log adapters | Implemented and production-assembled with local-calendar usage boundaries, message-owned usage deletion, bounded SQLite search/paging, unified diagnostic mapping, round-trip, invalid-row, migration/backfill, stale-active-pointer, foreign-key ownership, and failure-injected rollback tests; legacy root configuration and usage modules are removed | 8.3-8.8 |
| `contexts/sessions/api.rs` | Published in-process facade for session creation, current/archived/search/active queries, switching, rename, pin/archive/delete, categories, chat configuration, message persistence/composition, export, usage, and maintenance | Implemented session application API | 8.4-8.8 |
| `commands/sessions/*` | One-file creation/management/message/export/usage handlers, explicit camel-case DTO mapping, background operation execution, session events, and legacy error-string mapping | Implemented session inbound interface with input DTO, output mapping, state-event, shared safe-error, native registration, and frontend invoke compatibility tests covering all 22 migrated commands | 8.4-8.8 |
| `bootstrap/sessions.rs` | Concrete sessions repositories, creation/runtime/file/profile/operation/logging/clock/id adapters assembled and injected as one `SessionsApi` state; non-blocking worker bridges desktop archival policy into startup recovery and hourly maintenance | Implemented sessions composition root and maintenance scheduler | 8.4-8.8 |
| `contexts/agent_runtime/domain` | Agent identity/catalog, launch metadata, interaction modes, availability assessment, workflow selection/readiness/lifecycle, and generation transition invariants | Implemented pure agent-runtime domain with root agent/chat/provider compatibility paths removed | 9.1, 9.7 |
| `contexts/agent_runtime/application` | Agent registry/query/selection/readiness/session-details/launch/message/stop use cases over registry, workflow, sessions, CLI profile, effective-prompt, process, operation, logging, clock, event, and generation-coordination ports | Implemented application boundary with deterministic fake-port tests for lifecycle sequencing, safe terminal failures, reservation/attachment, and cancellation-event deduplication | 9.2 |
| `contexts/agent_runtime/infrastructure` | Explicit agent/mode/capability and workflow/session-details SQLite row mapping, context-owned stable registry seed, plus SDK/executable availability facts | Implemented registry/workflow repositories and runtime availability adapter with stable-id, injected-availability, round-trip, and invalid-row tests | 9.3 |
| `contexts/agent_runtime/infrastructure/providers` | Provider command construction, model/reasoning/permission mapping, prompt delivery, resume arguments, and output-event parsing for the four stable agent ids | Implemented and exclusively production-used through agent-runtime process/application ports, with per-agent invocation, parameter-mapping, and JSON/JSONL output fixtures | 9.4, 9.7 |
| `contexts/agent_runtime/infrastructure/{generation_coordinator,process_adapter,sessions_gateway,cli_profile,prompt_gateway,runtime_support,events}` | Per-session generation reservation, explicit child-process ownership/monitoring, stream-to-use-case delivery, Sessions/CLI/Prompt Hook published-API mapping, Operations lifecycle/log association, clock, and Tauri chat-event publication | Implemented and production-wired with exclusive generation leases, single-commit terminal handling, cancellation-safe partial stream persistence, atomic session/workflow lifecycle updates, redacted command diagnostics, and deterministic application/infrastructure tests | 9.5-9.6 |
| `contexts/agent_runtime/api.rs` | Published in-process agent query, workflow, readiness, launch, message, stop, and immutable agent/session result contracts; no repository or infrastructure exposure | Implemented agent-runtime application facade for commands and future communications callers | 9.3 |
| `commands/agent_runtime/*` | One-command Tauri handlers plus transport DTO and mapper boundaries for agent list/query, workflow selection/readiness/launch/details, chat send, and generation stop | Implemented thin interface adapters with stable command names, camel-case payloads, legacy-safe error strings, and frontend invoke registration tests | 9.6 |
| `bootstrap/agent_runtime.rs` | Concrete agent-runtime repositories, cross-context gateways, process/operation/log/event adapters, clock, and generation coordinator assembled into one `AgentRuntimeApi` state | Implemented composition root; session cleanup and IM startup both receive the published API without a legacy chat runtime | 9.6-9.7 |
| `contexts/agent_runtime/infrastructure/message_terminal_completions.rs` | One-shot terminal completion registry for in-process communications callers | Registered before provider launch and completed exactly once after persisted completed, failed, or cancelled state; dropped receivers clean up registrations without SQLite polling | 9.7, 10.5-10.6 |
| `contexts/communications/domain/*` | Connector identity/configuration, lifecycle status, routing/binding/dedup/checkpoint identities, QR authorization state, and inbound/final-delivery policy | Implemented pure domain model with typed invariant failures, stale-generation protection, terminal authorization rules, and deterministic tests | 10.1 |
| `contexts/communications/application/*` | Connector query/mutation/runtime use cases plus inbound claim/router orchestration; context-owned repository, credential, transport, agent-execution, session-binding, operation, clock, and logging ports | Implemented framework-independent application service with fake-port tests for ordering, credential compensation, operation terminal state, safe logging, dedup retention, binding, and Agent execution | 10.2 |
| `contexts/communications/api.rs` | Published in-process connector management, runtime, routing, binding, dedup, and WeChat authorization facade | Implemented and consumed by bootstrap, Tauri commands, desktop shutdown, and inbound delivery | 10.2, 10.6 |
| `contexts/communications/infrastructure/{schema,sqlite_repository}.rs` | Communications-owned additive SQLite migration plus explicit connector/routing/binding/dedup/checkpoint/WeChat-context row mapping | Implements scheduled dedup retention in batches of at most 512 rows and bounded per-chat WeChat secure-context metadata retention with restart, stale-refresh, and rollback coverage | 10.3 |
| `contexts/communications/infrastructure/credential_adapter.rs` | Context credential port over the platform keyring boundary | Implemented with zeroizing reads, stable account references, legacy WeChat migration/deletion, safe errors, and deterministic memory-store tests | 10.3 |
| `contexts/communications/infrastructure/transports/*` | Transport runtime contract, safe payload normalization, recorded fixtures, proxy-aware HTTP/WebSocket clients, and DingTalk, Feishu, Telegram, WeCom, and WeChat adapters | Includes expiry-aware single-flight token caches, immediate-empty-poll pacing, redacted bounded malformed-event diagnostics, and intentional acknowledgement/checkpoint advancement | 10.4 |
| `contexts/communications/infrastructure/{runtime_manager,transport_adapter,lifecycle_events}.rs` | Transactional connector replacement, bounded global/per-chat admission, lifecycle events, final delivery, concrete transport construction, and checkpoint/session-context adapters | Same-kind lifecycle mutations serialize; replacement restores the previous runtime on failure; clear stops the runtime before removing every tracked per-chat secure context; total pending work is capped at 64 and active Agent generations at 8; generation-safe idle lanes are reclaimed | 10.4, 10.6 |
| `contexts/communications/infrastructure/{application_adapters,runtime_bridge}.rs` | Explicit Agent, Sessions, Workspace, Operations, clock, logging, localization, dedup, and inbound-routing adapters | Implemented using published context APIs and an attach-once instance bridge; no `AppHandle` service lookup | 10.5-10.6 |
| `contexts/communications/infrastructure/wechat_authorization.rs` | QR transport, authorization state, and credential-persistence orchestration | Implemented behind fakeable network/credential ports with terminal-state and no-live-I/O tests | 10.6 |
| `bootstrap/communications.rs` | Communications repository, credential, transport, Agent, Sessions, Operations, clock, logging, inbound, and authorization composition | Implemented with explicit constructor dependencies and managed published APIs | 10.6, 11.1 |
| `commands/communications/*` | One-file handlers for all 12 existing IM commands plus explicit DTO mapping | Implemented over published Communications APIs with unchanged command names and JSON contracts | 10.6, 11.2 |
| `im/*` | Removed legacy IM command, storage, credential, model, runtime, and adapter compatibility tree | No callers or module registration remain | 10.6 |

## Tauri Command Contract Inventory

The names below are compatibility contracts and MUST remain registered until a separate approved change removes or renames them. Rust command DTOs map to the frontend service/types listed in the final column.

| Owner | Registered command names | Frontend contract owners |
| --- | --- | --- |
| `agent_runtime` | `list_agents`, `get_agent_by_id`, `get_workflow_state`, `select_agent`, `check_browser_readiness`, `launch_active_workflow`, `get_session_details`, `send_message`, `stop_generation` | `src/services/agent-service.ts`, `src/types/agent.ts`, `src/types/chat.ts`, `src/contracts/agent.ts`, `src/contracts/chat.ts` |
| `tooling::cli` | `list_cli_tools`, `refresh_cli_detections`, `install_cli_version`, `upgrade_all_cli_versions`, `list_cli_parameter_profiles`, `save_cli_parameter_profile`, `reset_cli_parameter_profile` | `src/services/agent-service.ts`, `src/types/agent.ts`, `src/contracts/agent.ts` |
| `sessions` | `create_session`, `list_sessions`, `list_archived_sessions`, `search_sessions`, `list_session_categories`, `create_session_category`, `rename_session_category`, `delete_session_category`, `assign_session_category`, `get_active_session`, `get_session_chat_config`, `save_session_chat_config`, `switch_session`, `rename_session`, `pin_session`, `unpin_session`, `archive_session`, `unarchive_session`, `export_session`, `delete_session`, `list_messages`, `get_usage_statistics` | `src/services/agent-service.ts`, `src/types/agent.ts`, `src/types/chat.ts`, `src/contracts/agent.ts`, `src/contracts/chat.ts` |
| `workspaces` | `list_known_projects`, `list_known_remote_workspaces`, `inspect_project`, `select_project_directory`, `list_session_directory`, `read_session_file`, `list_session_documents`, `get_session_git_status`, `get_session_git_diff`, `list_session_logs`, `export_session_logs`, `shell_create`, `shell_input`, `shell_cd`, `shell_resize`, `shell_kill` | `src/services/agent-service.ts`, `src/types/session-workspace.ts`, `src/contracts/session-workspace.ts` |
| `desktop` | `get_settings`, `save_setting`, `get_automatic_archival_settings`, `save_automatic_archival_settings`, `set_launch_on_startup`, `get_floating_assistant_runtime_info`, `get_floating_assistant_config`, `set_floating_assistant_enabled`, `set_floating_assistant_surface`, `start_floating_assistant_drag`, `save_floating_assistant_anchor`, `persist_floating_assistant_position`, `show_main_window`, `exit_application`, `test_network_proxy`, `scan_network_proxies`, `get_data_management_info`, `open_database_directory`, `open_log_directory`, `report_client_log_event`, `get_node_info` | `src/services/settings-service.ts`, `src/services/agent-service.ts`, `src/services/floating-assistant-service.ts`, corresponding `src/types/*` |
| `communications` | `list_im_connectors`, `get_im_routing`, `save_im_routing`, `save_im_connector`, `set_im_connector_enabled`, `restart_im_connector`, `test_im_connector`, `clear_im_connector`, `reset_im_bindings`, `begin_wechat_authorization`, `poll_wechat_authorization`, `cancel_wechat_authorization` | `src/services/im-service.ts`, `src/types/im.ts` |
| `tooling::mcp` | `list_mcp_servers`, `add_mcp_server`, `update_mcp_server`, `remove_mcp_server`, `toggle_mcp_server`, `test_mcp_connection`, `get_mcp_server_status`, `import_mcp_servers`, `export_mcp_servers` | `src/services/mcp-service.ts`, `src/types/mcp.ts`, `src/contracts/mcp.ts` |
| `tooling::sdk` | `list_sdk_definitions`, `list_sdk_statuses`, `check_sdk_environment`, `get_sdk_versions`, `check_sdk_updates`, `install_sdk_dependency`, `update_sdk_dependency`, `rollback_sdk_dependency`, `uninstall_sdk_dependency`, `get_sdk_operation_logs` | `src/services/sdk-service.ts`, `src/types/sdk.ts`, `src/contracts/sdk.ts` |
| `tooling::skills` | `list_skills`, `list_skill_mount_paths`, `update_skill_mount_path`, `create_skill`, `update_skill`, `delete_skill`, `restore_builtin_skill`, `set_skill_enabled`, `set_skill_agent_bindings`, `preview_skill`, `import_skill`, `detect_skill_drift`, `sync_skill_drift`, `select_workspace_directory` | `src/services/agent-service.ts`, `src/types/skill.ts`, `src/contracts/skill.ts` |
| `tooling::prompt_hooks` | `list_prompt_hooks`, `create_prompt_hook`, `update_prompt_hook`, `delete_prompt_hook`, `set_prompt_hook_enabled`, `set_prompt_hook_cli_bindings`, `preview_prompt_hook`, `preview_prompt_assembly`, `list_prompt_hook_traces` | `src/services/agent-service.ts`, `src/types/prompt-hook.ts` |
| `operations` | `list_operations`, `get_operation_status` | `src/services/operation-service.ts`, `src/types/operation.ts`, `src/contracts/operation.ts` |
| `tooling::extensions` | `get_extension_overview`, `refresh_extension_health`, `get_extension_install_preview`, `install_extension`, `uninstall_extension`, `set_extension_enabled`, `start_extension`, `stop_extension`, `test_extension` | `src/services/extension-service.ts`, `src/types/extension.ts` |
| `tooling::plugin_integrations` | `get_plugin_integration_overview`, `refresh_plugin_integrations`, `test_plugin_integration` | `src/services/plugin-integration-service.ts`, `src/types/plugin-integration.ts` |

Tauri event contracts are also compatibility boundaries: session-state events, chat stream events keyed by session id, shell events keyed by shell id, desktop/floating lifecycle events, and validated `im-connector:lifecycle` updates keyed by connector generation.

## SQLite Ownership and Migration Inventory

Migration order stays global; migration SQL moves to its owning context without changing version or name.

| Version | Name | Owned schema/columns | Target owner |
| --- | --- | --- | --- |
| 1 | `initial-schema` | `agents`, `agent_modes`, `agent_capability_tags`, `workflow_state`, `session_details`, `mcp_servers` | `agent_runtime`, `tooling::mcp` |
| 2 | `agent-managed-sdk-dependency` | `agents.managed_sdk_dependency_id` | `agent_runtime` published SDK dependency reference |
| 3 | `session-management` | `sessions`, `workflow_state.active_session_id` | `sessions`, `agent_runtime` reference |
| 4 | `chat-messages` | `messages` | `sessions` |
| 5 | `app-settings` | `settings` | `desktop` |
| 6 | `cli-tool-status` | `cli_tool_status` | `tooling::cli` |
| 7 | `skill-management` | `skills`, `skill_agent_bindings`, `skill_agent_mount_paths`, `deleted_builtin_skills`, `skill_drift_snapshots` | `tooling::skills` |
| 8 | `project-worktree-management` | `known_projects`; project/worktree columns on `sessions` | `workspaces`, `sessions` references |
| 9 | `session-runtime-metadata` | `sessions.runtime_session_id` | `sessions` |
| 10 | `im-connectors` | `im_connector_configs`, `im_credential_refs`, `im_routing_settings`, `im_session_bindings`, `im_inbound_dedup`, `im_connector_checkpoints`, additive `im_wechat_reply_contexts` | `communications` |
| 11 | `im-session-source` | source columns on `sessions` | `sessions` with communications values |
| 12 | `cli-parameter-settings` | `cli_parameter_settings` | `tooling::cli_parameters` |
| 13 | `session-chat-configuration` | `sessions.chat_preferences` | `sessions` |
| 14 | `floating-assistant-configuration` | `floating_assistant_config` | `desktop` |
| 15 | `local-extension-management` | `extension_framework_state` | `tooling::extensions` |
| 16 | `cli-local-environment-details` | environment/source/conflict/lifecycle columns on `cli_tool_status` | `tooling::cli` |
| 17 | `message-rich-blocks` | `messages.rich_blocks` | `sessions` |
| 18 | `session-management-organization` | `session_categories`, `sessions.category_id`, `messages.file_references` | `sessions` |
| 19 | `prompt-hook-management` | `prompt_hook_overrides`, `prompt_hooks_user`, `prompt_hook_traces` | `tooling::prompt_hooks` |
| 20 | `remote-workspace-sessions` | `known_remote_workspaces`; remote workspace columns on `sessions` | `workspaces`, `sessions` references |
| 21 | `sdk-operation-logs` | `sdk_operation_logs` | `tooling::sdk` |
| 22 | `session-usage-records` | `usage_records` plus message/session/agent ownership indexes and legacy positive-count backfill | `sessions` |

`schema_migrations` is owned by `platform::database`. Foreign-key references use stable ids across contexts; they do not grant repository access across boundaries.

## Background Job Inventory

| Job | Current trigger/executor | Target owner and boundary |
| --- | --- | --- |
| Session startup recovery | Bootstrap starts a worker and immediately returns; the worker invokes the `sessions` maintenance API | Implemented non-blocking sessions maintenance use case |
| Hourly automatic archival | Bootstrap worker reloads the published desktop archival policy before each sessions maintenance cycle | Implemented scheduled sessions maintenance adapter |
| Initial and requested CLI refresh | `commands::tooling::cli::background` schedules prepared `tooling::cli` application jobs | Implemented through status, detection, operation, logging, and clock ports |
| CLI install/upgrade/bulk upgrade | `commands::tooling::cli::background` schedules prepared jobs serialized by `CliMutationAdapter` | Implemented through package, process, operation, logging, and mutation ports |
| SDK install/update/rollback/uninstall | `commands::tooling::sdk::background` schedules prepared application jobs | Implemented through package, process, operation, repository, logging, and clock ports |
| MCP connection test | `commands::tooling::mcp::background` schedules the prepared application job | Implemented `tooling::mcp` use case through connection/operation ports |
| Extension refresh/install/start/stop/test | `commands::tooling::extensions::background` schedules prepared application jobs | Implemented through repository, installation, process, runtime, operation, logging, clock, and mutation ports |
| Agent generation and stream readers | `agent_runtime` process adapter and application event handling | Implemented through the agent-runtime generation use case/process adapter |
| IM connector startup/reconnect/inbound queues | Tokio tasks in IM runtime and transport adapters | Per-connector workers plus global pending/generation admission; startup and shutdown attempt every saved connector independently |
| IM inbound Agent completion wait | Agent runtime returns a registered one-shot terminal receiver before provider launch | Event-driven completion after terminal persistence; no blocking worker or periodic SQLite read |
| IM retention maintenance | Bootstrap Tokio interval runs immediately at startup and every six hours | Throttled dedup cleanup capped at 512 rows per run plus bounded WeChat per-chat secure-context retention |
| Shell output reader | worker thread in PTY adapter | `workspaces` shell infrastructure |
| Desktop delayed/lifecycle actions | Tauri async runtime in desktop lifecycle | `desktop` lifecycle adapter |

## External Adapter Inventory

| Technology | Current locations | Target |
| --- | --- | --- |
| SQLite/Rusqlite | `platform::database::NativeDatabase` owns app path/connection setup and centralized migrations; context SQL remains in infrastructure adapters | Implemented with typed `DatabaseError` and no root store alias |
| External process execution | Construction, command lookup, explicit requests, capture, timeout/kill, output draining, and audited command metadata live in `platform::process` | Implemented; no root process facade remains |
| Filesystem | Canonical containment and sibling worktree target primitives live in `platform::filesystem`; context adapters own bounded business-facing access | Implemented bounded platform paths |
| HTTP/SSE/MCP | Proxy policy and proxy-aware HTTP/WebSocket behavior live in `platform::network`; MCP and Communications consume it through infrastructure adapters | Implemented; no root proxy facade remains |
| IM HTTP/WebSocket/polling | `contexts/communications/infrastructure/transports/*` plus the context-owned transport/authorization adapters | Implemented over platform network policy with no legacy export |
| OS credentials | `platform::credentials::OsCredentialStore` keyring implementation behind `CommunicationsCredentialPort`, with stable references, zeroizing reads, legacy WeChat migration, and retry-safe deletion of legacy and per-chat session contexts | Implemented with deterministic memory-store coverage; no legacy wrapper remains |
| Clock and ids | `platform::clock::SystemClock` and per-instance `platform::ids::MonotonicIdGenerator`; operations infrastructure implements context-owned ports | Reuse through context-specific clock/id port implementations |
| PTY | `contexts/workspaces/infrastructure/portable_pty.rs` using `portable-pty` | Implemented workspace shell runtime adapter |
| Tauri dialog | Skill/workspace directory selection and workspace log export adapters | Tauri interface or infrastructure adapters implementing context selection ports |
| Tauri window/tray/autostart | Desktop infrastructure owns autostart, floating window geometry/actions, tray close fallback, startup initialization, and idempotent graceful exit; bootstrap adapts the published Communications API through `DesktopShutdownPort` | Implemented without importing communications infrastructure |
| Unified logs | `platform::logging` owns append persistence, redaction, rotation, and directory state; `operations` owns semantic diagnostic/operation ports and its adapter | Implemented with no feature-local or root log path |
| Observable tasks | All contexts consume the published `OperationsApi`; command queries use `commands/operations` | Implemented with no task registry facade |

## Final Guardrails

- There are no active architecture exceptions or compatibility allowlists for this migration.
- `lib.rs` may expose modules and delegate `run()` only. The parsed architecture test requires zero root business symbols.
- Every Tauri command implementation has a zero SQL/process/domain-decision budget. The complete registry is `commands/registry.rs` and is grouped by bounded context.
- Non-test external process construction is allowed only in `platform/process/mod.rs`; append-mode log persistence is allowed only in `platform/logging.rs`.
- Domain and application dependency checks parse Rust syntax and report forbidden framework, outer-layer, and private cross-context imports with file locations.
- The deleted `NativeAppState`, `RegistryStore`, root `AppError`, `tasks`, `im`, root logging/process/proxy facades, and architecture baseline files must not be reintroduced.

## Architecture Decisions

### ADR-001: Keep one crate with explicit module boundaries

The runtime remains a single Cargo crate. Module privacy plus parsed architecture tests enforce dependency direction without adding a dependency-injection framework or a multi-crate build during this refactor.

### ADR-002: Split semantic logging from log-store technology

`operations` owns diagnostic and operation log semantics. `platform::logging` owns redacted JSONL persistence, rotation, archival, and active-directory state. Context application code emits semantic records through ports; infrastructure adapters may consume the platform log store only at the outer edge.

### ADR-003: Keep CLI Parameters inside Tooling

CLI parameter catalogs and persisted selections remain a Tooling subdomain published through `CliParametersApi`. Sessions consumes immutable chat defaults and Agent Runtime consumes launch arguments through that API; neither context imports Tooling persistence or command DTOs.

### ADR-004: Centralize composition and command registration separately

`bootstrap/runtime.rs` owns construction, Tauri state registration, and background-job startup. `commands/registry.rs` owns the stable invoke surface. This keeps application assembly auditable without placing interface registration back in `lib.rs`.

### ADR-005: Generate TypeScript model contracts from Rust with `ts-rs`

Frontend service interfaces stay hand-authored because they express UI/runtime semantics, but command payload/result models are generated from Rust models with `ts-rs`. Each serializable Rust model that crosses the frontend/backend boundary derives `TS` alongside `Serialize` and `Deserialize`; generated files live under a stable frontend contract directory, and a verification command regenerates contracts and fails when committed contract files are stale. This keeps React components behind service interfaces while reducing silent drift between Rust command models and TypeScript types. `specta` / `tauri-specta` were rejected for tighter Tauri coupling and a broader integration surface; manual TypeScript-only was rejected because parallel Rust/TypeScript models drift silently as Agent, MCP, SDK, and operation models grow.

### ADR-006: The evidence journal records observations and never claims

`execution_observability` owns an append-only journal (migration 81) of events the runtime watched, plus a projection derived from it. Producers report their own work through ports in their own vocabulary; translation into journal shape happens in `bootstrap/evidence_bridge.rs` and nowhere else, over a bounded channel with a non-blocking send, so a producer never waits on the journal and a slow journal never becomes a latency spike in the work it is recording.

The rule that shapes everything else is that the journal holds only what was observed. Historical `message.toolUse` activity is *not* backfilled into it: a `toolUse` block is what an assistant said it was doing, and once that sits beside events the runtime witnessed, nothing downstream can separate them again — not the fidelity column, which a backfill would have to invent, and not the reader, who has no reason to suspect the question is worth asking. Historical activity is instead projected in the frontend from loaded messages, as its own list, always `inferred`, always partial coverage. Guards in `tests/evidence_bridge_architecture.rs` fail any file that both appends to the journal and reads the chat corpus, and no Tauri command may write evidence at all, since a client assertion is not an observation.

Coverage travels with every answer rather than being fetched separately. Dropped events, source conflicts, retention trimming, and a projection that is behind each degrade it to a named state with a stable reason code, so a short answer is never mistaken for a small session.

### ADR-007: The log index is a rebuildable projection, keyset-paged

`operations` owns a query index (migration 82) over the redacted JSONL files `platform::logging` retains. Nothing in it is a second source of truth: every row is derivable again from the files, which is what lets the schema be shaped for reading rather than for durability.

The index is brought up to date by a repair job started after the window exists, spawned rather than run, and handed to the blocking pool rather than executed on an async worker — synchronous SQLite on a runtime worker parks it for the length of the scan and queues every unrelated command behind it, with no symptom beyond "a few commands were slow once after launch". Progress is a persisted checkpoint keyed by directory generation rather than by path, so a rotated file resumes from zero instead of mid-file into bytes the offset was never written for. Reads happen outside any write transaction, batches commit rows, gaps, and checkpoint together, and every prune is bounded so a tidy-up never holds the write lock across the corpus.

Paging is keyset and stays keyset: this result set grows underneath the reader, and an offset renumbers with every insert, so offset paging silently skips exactly the rows that arrived while the reader was reading. Expiry — the one deletion here that cannot be undone, because the file it would be re-read from is gone — records a `log_source_expired` gap in the same transaction as the delete, so a query after a rotation reports partial coverage rather than a short answer that calls itself complete.

### ADR-008: Report is composed by the backend, from evidence

Session Report figures come from an aggregate query over the journal rather than from whatever `ChatMessage[]` happens to be mounted. The React aggregation it replaced made every number a function of scrolling: paging older messages in changed the totals, and a trimmed history reported a smaller session with no field anywhere saying so. Coverage is per section rather than per report, because a report stays useful while one of its sections cannot be substantiated, and the reader needs to know which one.

The retired aggregation is kept in `src/session-workspace/report-utils.ts`, uncalled, because `report-legacy-parity.test.ts` holds the divergences between the two as tests — those divergences are the reason for the replacement, and deleting the old code would delete the record of what its numbers meant, which is what invites somebody to make the backend agree with them again. A guard requires the only file mentioning it to be the one defining it.

### ADR-009: One workspace provider per session, capability by capability

`workspaces` answers file, search, diff, and shell questions for a session through one provider chosen by where the workspace is, and reports what that provider can do capability by capability rather than as a single flag. A remote host with Git but no ripgrep is ordinary, and one flag would either hide the search gap or disable the four things that work. Unsupported actions render disabled with a reason rather than vanishing: a control that disappears makes a reader think they misremembered where it was.

Session Shell has exactly one lifecycle owner, `SessionShellRegistry`, so a shell survives a tab switch, a session switch, and a remount, and stops only when someone says so. Its runtime descriptor is an internally tagged union — the frontend narrows on `kind`, and the string union it replaced let the UI offer resize to a simulated shell and reconnect to a PTY, because a string carries no constraints.
