## Context

VaneHub AI currently builds and tests successfully across the React/Vite frontend and Rust/Tauri backend. The current codebase already has useful boundaries for Agent, MCP, and SDK services, but the workspace UI still contains static prototype data, settings pages repeat local request/error orchestration, Rust and TypeScript models are maintained manually, SQLite migrations are embedded in startup code, and long-running native operations return only immediate command results.

This change establishes architecture foundations before adding more workflow, orchestration, provider, and runtime features. It affects both desktop and Web runtimes: desktop keeps full local capabilities through Tauri commands, while Web keeps mock/runtime adapters now and can later attach to an HTTP backend.

## Goals / Non-Goals

**Goals:**

- Keep React components isolated from Tauri details through service interfaces and runtime adapters.
- Add shared frontend foundations for routing, server-state fetching, form validation, error boundaries, and workspace modularization.
- Add native foundations for app-owned storage paths, versioned SQLite migrations, structured logging, task tracking, command safety, and a basic security baseline.
- Generate or verify shared Rust/TypeScript contracts so command payloads and frontend service types do not drift.
- Represent long-running SDK, MCP, and Agent operations as observable tasks with progress/log visibility where operations can exceed a short immediate command.

**Non-Goals:**

- Replacing Tauri commands with a remote API service in this change.
- Removing the Web/mock runtime adapter.
- Rewriting all UI screens or redesigning the visual system.
- Implementing cloud sync, multi-user collaboration, accounts, or remote execution.
- Changing the supported Agent catalog solely for this architecture foundation.

## Decisions

1. Use service interfaces plus runtime adapter factories as the frontend boundary.

   React components will continue to call `AgentService`, `McpService`, and `SdkService` style interfaces. Tauri `invoke()` stays only in Tauri-specific adapters. The factory should become explicit and testable instead of relying only on `window.__TAURI_INTERNALS__`.

   Alternatives considered: calling `invoke()` directly from components is simpler but couples UI to desktop runtime; introducing a full HTTP API now is premature because local desktop capabilities remain primary.

2. Use TanStack Query for server state and keep Zustand for lightweight UI state.

   TanStack Query should own async load/mutation/cache/retry/invalidation behavior for service calls. Zustand, if introduced, should only hold runtime-local UI selections such as active shell state, layout preferences, and transient UI mode.

   Alternatives considered: Redux Toolkit is more comprehensive but heavier than needed; page-local `useState` scales poorly for repeated async workflows.

3. Use React Router for top-level app surfaces.

   Workspace, settings, and future detail pages should be routable. This reduces ad hoc view switching and makes Web runtime deep links and desktop navigation easier to support.

   Alternatives considered: keeping local `view` state is enough for a prototype but becomes difficult once sessions, settings pages, and future detail views need shareable locations.

4. Use React Hook Form plus Zod for forms and validation.

   MCP server forms, SDK operation inputs, provider credentials, and future Agent configuration should share schema-driven validation. Backend validation remains authoritative; frontend validation improves UX and reduces duplicate component logic.

   Alternatives considered: custom validators per form avoid dependencies but duplicate behavior and make error display inconsistent.

5. Generate TypeScript contracts from Rust models where practical.

   `specta`/`tauri-specta` or `ts-rs` should be evaluated and one path selected. The target is generated TypeScript types for Tauri command payloads/results and service models, while service interfaces remain hand-authored if they express frontend semantics.

   Alternatives considered: manually maintaining `src/types` is acceptable early but becomes a source of subtle runtime bugs as models grow.

6. Move SQLite schema changes into versioned migrations.

   Startup should run migrations from explicit migration files or embedded migrations, using a tool such as `refinery` or a small controlled migration runner. The app should store data under an app-owned user data directory, not the process current directory.

   Alternatives considered: continuing `CREATE TABLE IF NOT EXISTS` and ignored `ALTER TABLE` statements is low ceremony but has poor rollback/debug behavior and weak guarantees.

7. Add a native task registry for long-running operations.

   SDK install/update/rollback/uninstall, MCP connection tests, and Agent launch flows should be eligible for task tracking. Tasks should expose id, kind, status, progress/logs where available, result/error, timestamps, and cancellation capability when the underlying operation supports it. Tauri events can publish progress; SQLite can persist final task/log records when needed.

   Alternatives considered: synchronous command results are easier to implement but cannot support progress, cancellation, durable logs, or concurrent operation visibility.

8. Establish command safety and security baseline in the native layer.

   External commands should be constructed from backend-owned definitions or validated user configuration without shell string interpolation. Command execution should be audited, and sensitive capabilities should be constrained by allowlists, user-confirmed configuration, and Tauri capability/security settings. CSP should be explicit rather than disabled.

   Alternatives considered: broad local execution is flexible, but VaneHub manages developer tooling and therefore needs clear controls before more integrations are added.

## Risks / Trade-offs

- Dependency growth -> Introduce libraries only where they replace repeated patterns and add tests around adapter behavior.
- Migration bugs -> Add migration tests against empty and previously seeded databases before moving production storage paths.
- Generated type tooling friction -> Start with one model family, then expand after the generator fits Tauri 2 and the repository workflow.
- Task model overengineering -> Begin with SDK operations and MCP tests, then extend to Agent launch once the task API proves useful.
- Web/runtime divergence -> Keep service interface conformance tests for Tauri and Web adapters.
- Security restrictions blocking legitimate workflows -> Design allowlists and confirmations around user-configured local tools, with clear error messages and audit logs.

## Migration Plan

1. Add architecture dependencies and provider shells without changing existing behavior.
2. Introduce generated/shared contracts for one service family, then migrate Agent, MCP, and SDK types.
3. Add versioned SQLite migrations while preserving the existing database schema.
4. Move storage path resolution to an app-owned user data directory with a compatibility path for existing local development data if needed.
5. Refactor settings pages to use shared query/form/error primitives.
6. Add the task registry and migrate SDK operations first, then MCP tests, then Agent launch flows.
7. Split workspace UI into service-backed modules, leaving mock Web data behind adapters.
8. Tighten CSP and command execution safeguards after the runtime boundaries and tasks are in place.

Rollback strategy: each step should be additive. If a new adapter, generated type layer, or task path fails, keep the existing direct service command path until the replacement is verified and covered by tests.

## Open Questions

- Which type generation tool should be selected after a small spike: `specta`/`tauri-specta` or `ts-rs`?
- Should task logs be fully persisted in SQLite, or should only final task summaries be persisted while live logs stream through Tauri events?
- What compatibility behavior is required for existing `.vanehub` development databases created under the repository current directory?
- Which command categories require explicit user confirmation versus backend allowlist validation only?
