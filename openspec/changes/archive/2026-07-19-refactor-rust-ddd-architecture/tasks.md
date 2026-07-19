## 1. Standards and Baseline

- [x] 1.1 Add a Rust native DDD section to `openspec/project.md` covering the bounded-context map, target module layout, dependency direction, context APIs, model mapping, ports, transactions, errors/logging, tests, and temporary-exception rules.
- [x] 1.2 Audit every current Rust module, Tauri command, SQLite table/migration, background job, and external adapter into a committed `src-tauri/ARCHITECTURE.md` migration matrix with one owning context and target layer.
- [x] 1.3 Record the current public Tauri command names and frontend adapter payload/result models, and add missing representative serialization/error characterization tests before moving handlers.
- [x] 1.4 Add empty, current, and representative older SQLite database fixtures that prove all existing migrations and persisted records remain readable.
- [x] 1.5 Add baseline tests for operation ids/state transitions/log association and unified-log redaction so those behaviors can be checked after each migration.

## 2. Module Skeleton and Architecture Guardrails

- [x] 2.1 Create `bootstrap`, `contexts`, and `platform` module roots plus context modules for `agent_runtime`, `sessions`, `workspaces`, `tooling`, `communications`, `desktop`, and `operations` without changing runtime behavior.
- [x] 2.2 Introduce a compatibility `NativeAppState` in the composition root that exposes assembled legacy services while contexts are migrated and does not leak into domain/application code.
- [x] 2.3 Add a Rust syntax-parsing architecture test that reports forbidden domain/application dependencies and private cross-context imports with file locations.
- [x] 2.4 Add architecture rules that reject new business implementations in `lib.rs` and SQL/process/domain decisions in Tauri command adapters.
- [x] 2.5 Add shared native test support for deterministic clocks/ids, fake application ports, temporary SQLite databases, filesystem fixtures, and captured command/log output.
- [x] 2.6 Run the Rust test suite after the skeleton lands and fix guardrail false positives before migrating the first context.

## 3. Native Platform and Operations Boundaries

- [x] 3.1 Introduce typed domain/application/infrastructure error categories and a single command-safe Tauri error mapper that preserves current serialized error behavior.
- [x] 3.2 Extract app-owned database path resolution, connection creation, and centralized versioned migration execution from `RegistryStore` into platform database adapters with compatibility tests.
- [x] 3.3 Define operations-context application ports for diagnostic and operation logging, then adapt the existing unified logging service with redaction parity tests.
- [x] 3.4 Move observable task lifecycle behavior behind operations-context use cases and adapters while preserving current task ids, states, timestamps, results, errors, and logs.
- [x] 3.5 Extract explicit external-process execution and network-proxy application into platform adapters usable through context-owned ports, retaining timeout and no-shell-interpolation tests.
- [x] 3.6 Extract bounded filesystem and Git execution helpers into platform adapters with canonical-path, traversal, worktree, and redacted-diagnostic tests.
- [x] 3.7 Extract OS credential access behind a platform adapter that can be replaced by the existing deterministic memory store in tests.
- [x] 3.8 Provide concrete clock and id adapters for application ports without introducing global mutable time/id helpers.
- [x] 3.9 Rewire legacy callers to the new platform and operations adapters through compatibility facades, then verify no new feature-local log or process path was introduced.

## 4. MCP Reference Slice

- [x] 4.1 Move MCP identities, scope, transport, configuration invariants, and connection outcome semantics into `tooling::mcp::domain` with pure unit tests.
- [x] 4.2 Define MCP management and connection-test use cases plus context-owned repository, connection, operation, clock, and logging ports.
- [x] 4.3 Implement the MCP SQLite repository and row/domain mappings without exposing `rusqlite::Connection` through the application boundary.
- [x] 4.4 Implement stdio and HTTP MCP connection adapters through the process/network platform boundaries, preserving validation, timeout, and command-safety behavior.
- [x] 4.5 Rewire each MCP Tauri command file to the published MCP application API while preserving command names, serialized DTOs, errors, task ids, logs, import/export, and Web adapter contracts.
- [x] 4.6 Add domain, fake-port application, SQLite/connection adapter, and Tauri contract tests for the MCP slice, then remove its legacy `service` entry points and architecture exceptions.

## 5. CLI and SDK Tooling

- [x] 5.1 Move CLI tool definitions, installation/source/conflict states, lifecycle eligibility, stable-version comparison, and mutation invariants into `tooling::cli::domain` with pure tests.
- [x] 5.2 Add CLI list/refresh/install/upgrade use cases and ports for status persistence, detection, package execution, operations, logging, clock, and mutation serialization.
- [x] 5.3 Implement CLI SQLite status repositories and bounded detection/package adapters while preserving candidate limits, timeouts, source validation, and explicit executable arguments.
- [x] 5.4 Rewire CLI Tauri commands and initial background refresh through the CLI application API, retaining command contracts, cached startup behavior, operation logs, and post-mutation refresh.
- [x] 5.5 Remove CLI detection/package/database logic from `lib.rs` after domain, application, infrastructure, concurrency, and command contract tests pass.
- [x] 5.6 Move SDK definition, status, version, operation, and lifecycle rules into `tooling::sdk::domain` with pure unit tests.
- [x] 5.7 Add SDK query/mutation use cases and repository, package-execution, operation, and logging ports with fake-port application tests.
- [x] 5.8 Implement SDK SQLite and process adapters through the shared platform boundaries, preserving rollback, environment checks, version lookup, and unified operation logs.
- [x] 5.9 Rewire SDK Tauri commands to the published SDK API, add serialization/error compatibility tests, and remove the legacy SDK service path.

## 6. Extension, Skill, and Prompt Tooling

- [x] 6.1 Extract local-extension catalog, health, enablement, installation, drift, and removal rules into `tooling::extensions` domain/application modules.
- [x] 6.2 Implement extension filesystem/process/persistence adapters and rewire extension commands and background operations with contract, safety, and log parity tests.
- [x] 6.3 Extract plugin-integration configuration and lifecycle rules into `tooling::plugin_integrations` domain/application modules with explicit external-tool ports.
- [x] 6.4 Implement plugin integration adapters and rewire Tauri commands while preserving serialized results, unified diagnostics, and current process safety.
- [x] 6.5 Move Skill identity, source, mount, binding, drift, validation, and mutation invariants into `tooling::skills::domain` with pure tests.
- [x] 6.6 Add Skill management/preview/import/sync use cases and behavior-oriented persistence, workspace-selection, filesystem, clock, and logging ports.
- [x] 6.7 Implement Skill SQLite/filesystem adapters and rewire all Skill Tauri commands with atomic mutation, traversal, contract, and diagnostic tests.
- [x] 6.8 Move Prompt Hook identity, category, ordering, binding, template, and immutable built-in invariants into `tooling::prompt_hooks::domain` with pure tests.
- [x] 6.9 Add Prompt Hook management, preview, assembly, and trace use cases plus repository, clock, and logging ports; publish the effective-prompt API required by agent runtime.
- [x] 6.10 Implement Prompt Hook persistence/rendering adapters and rewire commands and provider integration with atomicity, no-script-execution, ordering, trace, and contract tests.

## 7. Desktop and Workspace Contexts

- [x] 7.1 Move application settings, archival settings, startup preferences, and validation/default rules into `desktop` domain/application modules.
- [x] 7.2 Implement desktop settings repositories and Tauri adapters for settings, database/log directories, autostart, Node information, and network proxy actions with compatibility tests.
- [x] 7.3 Move floating-assistant enablement, surface, anchor, position, and visibility rules into desktop use cases with context-owned persistence/window ports.
- [x] 7.4 Adapt tray, window, floating-assistant, startup, and shutdown behavior at the desktop infrastructure/interface edge with lifecycle and redacted-error tests.
- [x] 7.5 Move project, remote-workspace, worktree-name, path-containment, and project-inspection rules into `workspaces::domain` with pure tests.
- [x] 7.6 Add workspace selection/history/worktree use cases and repositories, then adapt SQLite, dialog, filesystem, and explicit Git command implementations.
- [x] 7.7 Move session-directory, document, file-read, Git-status/diff, log-list, and log-export behavior behind bounded workspace query use cases and thin command files.
- [x] 7.8 Move shell lifecycle and working-directory rules behind workspace application ports and a portable-PTY adapter, preserving resize/input/kill safety and command contracts.
- [x] 7.9 Add workspace domain, fake-port application, SQLite/Git/filesystem/PTY adapter, traversal, and Tauri serialization tests before removing legacy workspace helpers.

## 8. Sessions Context

- [x] 8.1 Move session/message/category/chat-configuration identities, lifecycle transitions, activation, ownership, pin/archive, and file-reference invariants into `sessions::domain` with pure tests.
- [x] 8.2 Define session creation/query/search/category/configuration/message/export/maintenance/usage use cases and context-owned repository, transaction, clock, file-content, operation, and logging ports.
- [x] 8.3 Implement session, message, category, configuration, and usage SQLite repositories with explicit row/domain mapping and atomic transaction tests.
- [x] 8.4 Migrate session creation, switching, rename, pin/archive, category, configuration, deletion, and active-session commands to published session application APIs.
- [x] 8.5 Migrate message persistence, validated file-reference composition, list/search, JSON/Markdown export, and usage aggregation to session use cases and adapters.
- [x] 8.6 Migrate startup recovery and scheduled archival into session maintenance use cases injected with clock and repository ports without blocking Tauri startup.
- [x] 8.7 Preserve local-calendar usage semantics, message-owned usage deletion, export filenames, bounded search/query behavior, state events, and unified diagnostics through focused adapter tests.
- [x] 8.8 Add Tauri DTO/error compatibility tests for every migrated session command, then remove session/database/business helpers from root and legacy modules.

## 9. Agent Runtime Context

- [x] 9.1 Move agent registry, launch metadata, interaction mode, availability, workflow, readiness, lifecycle, and generation transition rules into `agent_runtime::domain` with pure tests.
- [x] 9.2 Define agent registry/query/selection/readiness/launch/message/stop use cases and repositories/gateways for sessions, CLI profiles, effective prompts, processes, tasks, logging, clock, and events.
- [x] 9.3 Implement agent registry/workflow SQLite adapters and publish only the immutable agent/session contracts required by other contexts.
- [x] 9.4 Move provider-specific invocation builders, model/parameter mapping, prompt delivery, session-resume tokens, and output parsers into agent-runtime infrastructure adapters with fixture tests for all supported stable agent ids.
- [x] 9.5 Move generation reservation, process lifecycle, stream parsing, cancellation, completion, failure, session updates, and operation/log association into application use cases and process adapters.
- [x] 9.6 Rewire agent/workflow/readiness/launch/chat/list/stop Tauri commands to the published agent-runtime API without changing serialized frontend contracts or starting sessions during availability checks.
- [x] 9.7 Add concurrency, cancellation, parser, command-safety, redaction, session-integration, and Tauri contract tests, then remove agent/chat/provider implementation from `lib.rs`.

## 10. Communications Context

- [x] 10.1 Move connector identity, configuration, status, routing, binding, deduplication, checkpoint, authorization, and delivery invariants into `communications::domain` with pure tests.
- [x] 10.2 Define connector management/runtime/router use cases and context-owned repository, credential, transport, agent-execution, session-binding, operation, clock, and logging ports.
- [x] 10.3 Implement connector SQLite and secure-credential adapters with additive migration, atomic mutation, deletion, deduplication, and memory-store tests.
- [x] 10.4 Adapt DingTalk, Feishu, Telegram, WeCom, and WeChat transports at the infrastructure edge with normalized fixture, retry, proxy, authorization, and final-delivery tests.
- [x] 10.5 Replace direct root chat/session calls in IM routing with published sessions and agent-runtime application APIs while preserving nonblocking startup and inbound completion behavior.
- [x] 10.6 Rewire all IM Tauri commands and runtime bootstrap with contract, fake-port, status, binding, log-redaction, and no-live-credential/network tests.

## 11. Composition Root and Legacy Removal

- [x] 11.1 Construct all context repositories, gateways, use cases, background jobs, and Tauri-facing state in `bootstrap` with explicit dependencies and no domain/application service-location.
- [x] 11.2 Centralize the invoke handler in auditable bounded-context command groups while retaining every existing command name and one-command-per-file interface implementation.
- [x] 11.3 Replace all remaining `RegistryStore`, root `AppError`, root logging/time helpers, and direct cross-context imports with context APIs and platform/operations adapters.
- [x] 11.4 Reduce `src-tauri/src/lib.rs` to module exposure and delegation to bootstrap `run()`, with no domain models, SQL, command handlers, process code, or application orchestration.
- [x] 11.5 Remove compatibility facades, legacy modules, and temporary architecture exceptions only after `rg` and the architecture test confirm no callers or forbidden dependencies remain.
- [x] 11.6 Update `src-tauri/ARCHITECTURE.md` and `openspec/project.md` to match the implemented context map and record any nonblocking boundary refinements as ADRs.

## 12. Verification

- [x] 12.1 Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and resolve all formatting differences.
- [x] 12.2 Run `cargo test --manifest-path src-tauri/Cargo.toml` including architecture, domain, application, migration, adapter, and command contract tests.
- [x] 12.3 Run `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml` and resolve all errors and warnings introduced by the refactor.
- [x] 12.4 Run `npm run lint`, `npm run test`, `npm run contracts:check`, and `npm run build` to confirm frontend/Tauri contract and Web/mock runtime compatibility.
- [x] 12.5 Smoke-test packaged desktop startup plus representative settings, MCP task, CLI refresh, session/chat, workspace shell, and IM-status flows without blocking the main window.
- [x] 12.6 Smoke-test the browser Web/mock runtime and confirm it neither imports nor invokes the Rust native runtime.
- [x] 12.7 Run `openspec validate "refactor-rust-ddd-architecture" --strict` and `openspec validate --specs --strict`, then record implementation verification and any explicitly deferred follow-up before archive.
