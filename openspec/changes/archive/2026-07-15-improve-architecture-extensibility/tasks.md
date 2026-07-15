## 1. Frontend Foundation

- [x] 1.1 Add selected frontend dependencies for routing, data fetching, lightweight UI state, error boundaries, and form validation.
- [x] 1.2 Create an app-level routing shell for workspace and settings surfaces while preserving the current default user flow.
- [x] 1.3 Add a shared query/client provider and route or feature error boundary wiring around the application shell.
- [x] 1.4 Replace implicit runtime detection with an explicit runtime adapter factory that can select Tauri desktop, Web mock, and future HTTP adapters.
- [x] 1.5 Add adapter conformance tests for at least one service family to prove Web adapters do not import Tauri APIs.

## 2. Service Data and Form Patterns

- [x] 2.1 Refactor the MCP settings page to use shared data-fetching mutations, invalidation, loading, and error states.
- [x] 2.2 Refactor the SDK settings page to use shared data-fetching mutations, invalidation, loading, and error states.
- [x] 2.3 Refactor the Agents settings page to use shared data-fetching behavior for agent list, workflow state, and session details.
- [x] 2.4 Add schema-backed validation for MCP server forms and map backend validation failures into form or page errors.
- [x] 2.5 Add typed service error normalization for Tauri and Web service adapters.

## 3. Contract Foundation

- [x] 3.1 Spike `specta`/`tauri-specta` versus `ts-rs` and document the selected Rust-to-TypeScript contract generation approach.
- [x] 3.2 Generate or verify TypeScript contracts for Agent registry and workflow models.
- [x] 3.3 Generate or verify TypeScript contracts for MCP configuration, status, test, import, and export models.
- [x] 3.4 Generate or verify TypeScript contracts for SDK catalog, status, version, operation, and log models.
- [x] 3.5 Add a verification command or test that fails when generated contracts differ from committed frontend types.

## 4. Native Storage and Migrations

- [x] 4.1 Add app-owned storage path resolution for the Tauri desktop runtime without using the process current working directory for database location.
- [x] 4.2 Introduce a versioned SQLite migration runner and move existing schema creation into explicit migrations.
- [x] 4.3 Add migration tests for an empty database and an upgraded existing database.
- [x] 4.4 Preserve project-scoped MCP path matching by storing canonical project paths independently from database location.
- [x] 4.5 Add structured native error and logging plumbing for storage, database, validation, command, and task failures.

## 5. Observable Task Model

- [x] 5.1 Define shared operation/task models for id, kind, status, related entity id, logs, result, error, and timestamps.
- [x] 5.2 Implement a native task registry that supports status lookup and completion/failure recording.
- [x] 5.3 Migrate SDK install, update, rollback, and uninstall operations to expose operation ids and retrievable logs.
- [x] 5.4 Extend MCP connection tests to expose operation status for long-running tests while preserving final test results.
- [x] 5.5 Extend Agent launch flows to expose observable operation state when launch cannot complete immediately.

## 6. Command Safety and Security

- [x] 6.1 Centralize external command construction behind native helpers that use explicit executable and argument values.
- [x] 6.2 Add validation and audit logging for user-configured MCP stdio commands before execution.
- [x] 6.3 Add command safety tests for SDK, Agent, and MCP command construction paths.
- [x] 6.4 Replace disabled Tauri CSP with an explicit CSP compatible with the packaged application.
- [x] 6.5 Review Tauri capability/security configuration and keep privileged operations behind declared commands and service adapters.

## 7. Workspace Modularization

- [x] 7.1 Split `MainLayout` into workspace navigation, conversation/workflow content, agent graph or chat view, composer, and runtime details modules.
- [x] 7.2 Move static workspace demo data behind a Web-compatible workspace service adapter.
- [x] 7.3 Add a Tauri-compatible workspace service interface stub for future desktop-backed workspace state.
- [x] 7.4 Add tests or smoke coverage that workspace rendering works in browser Web runtime without Tauri APIs.

## 8. Verification

- [x] 8.1 Run `npm run test` and update/add tests for changed frontend service, form, and routing behavior.
- [x] 8.2 Run `npm run build` and fix any TypeScript or Vite build issues.
- [x] 8.3 Run Rust tests and checks for `src-tauri` after native storage, migration, task, or command changes.
- [x] 8.4 Run `openspec validate "improve-architecture-extensibility" --strict`.
- [x] 8.5 Document any intentionally deferred tasks or follow-up changes before archiving.
