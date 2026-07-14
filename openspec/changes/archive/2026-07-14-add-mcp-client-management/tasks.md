## 1. Dependency and API Spike

- [x] 1.1 Select the actual `rmcp` crate version and feature flags needed for client, stdio child process transport, and SSE transport.
- [x] 1.2 Add required Rust async/runtime dependencies and confirm `cargo check --manifest-path src-tauri\Cargo.toml` compiles with the selected MCP SDK features.
- [x] 1.3 Build a minimal backend spike or test helper that initializes a stdio MCP server and lists tools through `rmcp`.
- [x] 1.4 Build a minimal backend spike or test helper that initializes an SSE MCP server and lists tools through `rmcp`.
- [x] 1.5 Record any `streamable_http` limitations in code comments or follow-up notes without making it a P1 completion blocker.

## 2. Rust Data Model and Persistence

- [x] 2.1 Add `src-tauri/src/mcp/` module structure with `mod.rs`, `models.rs`, `service.rs`, `connection.rs`, and `commands.rs`.
- [x] 2.2 Add MCP Rust models for server config, partial update config, tool info, server status, test result, import/export payloads, transport type, scope, and connection status.
- [x] 2.3 Add `mcp_servers` SQLite migration fields for config, scope, `project_path`, cached status, cached tools, errors, duration, and timestamps.
- [x] 2.4 Add AppError variants for MCP not found, validation failure, and MCP connection failure.
- [x] 2.5 Implement current project path resolution using the backend current working directory absolute path.

## 3. Rust MCP Service Behavior

- [x] 3.1 Implement server validation for kebab-case names, global uniqueness, valid scope, transport-specific required fields, and JSON field shape.
- [x] 3.2 Implement `list` to return all user-scoped servers and current-project scoped servers sorted active-first then by name.
- [x] 3.3 Implement `add`, `update`, rename, `remove`, and `toggle` service functions while preserving cached status across renames.
- [x] 3.4 Implement `getServerStatus` from cached SQLite fields only, including disabled status for inactive servers.
- [x] 3.5 Implement Claude Desktop import with conflict skipping, selected scope assignment, and URL-only entries defaulting to `sse`.
- [x] 3.6 Implement Claude Desktop export for selected server names while excluding VaneHub-only metadata.
- [x] 3.7 Add focused Rust tests for validation, project-scope filtering, rename conflicts, import conflict skipping, export field filtering, and status-cache mapping.

## 4. Rust MCP Connection Behavior

- [x] 4.1 Implement oneshot stdio MCP connection using configured command, args, and env.
- [x] 4.2 Implement oneshot SSE MCP connection using configured URL and headers.
- [x] 4.3 Wrap initialize and list-tools flow in a fixed timeout and return timeout errors as failed test results.
- [x] 4.4 Map MCP tool name, description, and input schema into the VaneHub MCP tool model.
- [x] 4.5 Update cached status, tools, error, last connected timestamp, and duration after every test attempt.
- [x] 4.6 Keep `callTool` model/service interface reserved without exposing a P1 UI workflow.

## 5. Tauri Command Integration

- [x] 5.1 Add Tauri command wrappers for listing, adding, updating, removing, toggling, testing, status lookup, importing, and exporting MCP servers.
- [x] 5.2 Register MCP commands in the Tauri invoke handler.
- [x] 5.3 Keep `lib.rs` changes limited to module declaration, migration wiring, error variants, and command registration.

## 6. Frontend Service Boundary

- [x] 6.1 Add `src/types/mcp.ts` with MCP TypeScript models aligned to Rust serialization names.
- [x] 6.2 Add `McpService` interface with list, add, update, remove, toggle, test, status, import, export, and reserved callTool signatures.
- [x] 6.3 Add Tauri MCP adapter that owns all `invoke()` calls for MCP commands.
- [x] 6.4 Add Web MCP adapter with mock data, mock status, mock import/export behavior, and deterministic mock test results.
- [x] 6.5 Add runtime MCP service factory that selects Tauri or Web adapter using the existing runtime detection pattern.

## 7. Settings UI

- [x] 7.1 Update the settings shell to preserve mounted state for stateful settings pages while showing only the active page.
- [x] 7.2 Replace the demo MCP page with a service-backed MCP management container.
- [x] 7.3 Add MCP server card UI with transport badge, active status, cached test status, command or URL summary, and actions for edit, test, toggle, and delete.
- [x] 7.4 Add MCP server form UI for create, edit, and rename with dynamic fields for stdio and SSE.
- [x] 7.5 Use JSON text areas for `env` and `headers`, and simple args editing that persists as an array.
- [x] 7.6 Add test result UI that shows success or failure, tools, error text, duration, and last connected time.
- [x] 7.7 Add import/export UI with JSON paste, conflict feedback, selected scope for import, server checkbox selection for export, and copyable output.
- [x] 7.8 Add empty state with an action to add the first MCP server.
- [x] 7.9 Ensure the MCP UI uses the MCP service interface and does not import Tauri `invoke()` directly.

## 8. Verification

- [x] 8.1 Run `openspec validate "add-mcp-client-management" --strict` and fix any artifact issues.
- [x] 8.2 Run frontend tests with `npm run test`.
- [x] 8.3 Run frontend build with `npm run build`.
- [x] 8.4 Run Rust check with `cargo check --manifest-path src-tauri\Cargo.toml` using the selected dependency mode.
- [x] 8.5 Manually verify a real stdio MCP server can be added and tested from the desktop runtime.
- [x] 8.6 Manually verify a real SSE MCP server can be added and tested from the desktop runtime.
- [x] 8.7 Verify inactive servers can still be manually tested and status reads do not initiate live connections.
- [x] 8.8 Verify Web runtime renders MCP mock data without native Tauri commands.
