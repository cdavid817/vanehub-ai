## Context

The current MCP settings page is a frontend demo surface backed by static data. The Rust backend has no MCP code, and the existing `lib.rs` already owns agent registry, workflow state, migration, commands, and tests. The frontend already has a Port/Adapter pattern for agents: React depends on a TypeScript service interface, the Tauri adapter owns `invoke()` calls, and the Web adapter returns mock data.

This change adds real MCP client management while preserving those boundaries. The desktop runtime will own SQLite persistence, native process/network connection tests, and MCP SDK integration. The Web runtime will remain usable through a mock MCP service adapter.

## Goals / Non-Goals

**Goals:**

- Manage MCP server configurations from the settings UI.
- Persist MCP server configuration in SQLite with real user/project scope semantics.
- Treat project scope as the current working directory absolute `project_path`.
- Keep server names globally unique across user and project scopes.
- Support creating, editing, renaming, deleting, enabling, and disabling servers.
- Allow inactive servers to be manually tested.
- Test real MCP connectivity for `stdio` and `sse` transports using `rmcp`.
- Enforce a connection timeout for test operations.
- Cache the latest test status, discovered tools, errors, and duration in SQLite.
- Read server status from cached values rather than performing live checks.
- Import and export Claude Desktop `mcpServers` JSON, defaulting URL-only imports to `sse`.
- Keep React components behind a frontend MCP service interface and runtime adapters.

**Non-Goals:**

- Expose MCP tool calling in the P1 UI.
- Route MCP tools into agent launch or execution flows.
- Maintain persistent MCP connections or worker pools.
- Encrypt MCP `env` or `headers` secrets.
- Build structured key/value editors for `env` and `headers`; JSON text areas are sufficient for P1.
- Require `streamable_http` to pass real connection tests in P1.
- Manage MCP resources or prompts beyond preserving future extension points.

## Decisions

### Use the Existing Frontend Port/Adapter Pattern

React components will depend on a new `McpService` interface under `src/services/`. Runtime selection will mirror the agent service pattern:

```text
McpPage
  -> mcpService
    -> runtime-mcp-client
      -> tauri-mcp-client
      -> web-mcp-client
```

The Tauri adapter will call commands with `invoke()`. The Web adapter will provide mock server data and deterministic mock test results.

Rationale: the settings UI must run in both desktop and browser contexts, and the project already uses this pattern successfully for agent management.

Alternative considered: call Tauri commands directly from MCP React components. This was rejected because it would break the established frontend/backend isolation and make the Web runtime unusable.

### Store MCP Configuration and Cached Status in SQLite

The desktop backend will add an `mcp_servers` table:

```sql
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
```

`args`, `env`, `headers`, and `last_tools` will be JSON strings. Server names are globally unique by using `name` as the primary key. Renames update the primary key after validating the new name and checking for conflicts.

Rationale: this follows the existing local SQLite architecture and keeps status reads fast and deterministic.

Alternative considered: store Claude Desktop JSON files directly. This was rejected because the app needs scope, active state, status cache, descriptions, and future metadata that do not belong in the Claude Desktop format.

### Give Project Scope Real Semantics with `project_path`

User-scoped servers apply globally. Project-scoped servers apply only when their `project_path` matches the backend's current working directory absolute path. `listServers` returns user-scoped servers plus matching project-scoped servers. Export can include any user-selected server regardless of scope, but the exported Claude Desktop JSON excludes scope metadata.

Rationale: this gives project scope real behavior without introducing a larger workspace registry.

Alternative considered: allow the same server name in user and project scopes with override behavior. This was rejected for P1 because it complicates lookup, testing, export, rename, and status caching.

### Use Oneshot `rmcp` Connections for P1

Each explicit test operation creates a fresh MCP client connection, initializes it, lists tools, records the result, and drops the client:

```text
testConnection
  -> load config
  -> create stdio or sse transport
  -> initialize client
  -> list tools
  -> cache status and tools
  -> return result
```

The operation will use `tokio::time::timeout` around the full connection and list-tools flow. P1 must prove real connectivity for `stdio` and `sse`. `streamable_http` remains in the model for compatibility and later expansion, but it is not a P1 completion requirement.

Rationale: oneshot avoids persistent process lifecycle management, stale connections, and worker cleanup complexity. MCP configuration and discovery are low-frequency settings operations.

Alternative considered: maintain a persistent worker pool. This was rejected for P1 because it introduces process leak, lifecycle, cancellation, and shared-state complexity before the product needs high-frequency tool calls.

### Cache Status Instead of Performing Live Status Checks

`getServerStatus` returns data from `mcp_servers` cache fields. It does not start a process, open an HTTP connection, or call MCP. Explicit test operations are the only P1 path that refreshes connection status and tools.

Status mapping:

```text
active = false
  -> connectionStatus = disabled

active = true and last_connection_status = connected
  -> connectionStatus = connected

active = true and last_connection_status = error
  -> connectionStatus = error

active = true and no last test
  -> connectionStatus = disconnected
```

Inactive servers can still be tested manually; inactive only means "not participating in use."

Rationale: status reads should be cheap, predictable, and free of side effects. Live checks belong to explicit user actions.

### Import and Export Claude Desktop Format

Import accepts:

```json
{
  "mcpServers": {
    "server-name": {
      "command": "...",
      "args": [],
      "env": {},
      "url": "...",
      "headers": {}
    }
  }
}
```

Entries with `command` import as `stdio`. Entries with `url` and no `command` import as `sse`. Name conflicts are skipped. Imports create servers in the selected scope; project imports use the current absolute `project_path`.

Export is based on user-selected server names and excludes internal fields: scope, project path, active state, description, status cache, timestamps, and transport type. It writes only transport-relevant Claude Desktop fields.

Rationale: Claude Desktop compatibility makes migration easy while SQLite remains the internal source of truth.

### Keep P1 Secret Handling Plaintext and Explicit

MCP `env` and `headers` may contain tokens. P1 stores them as plaintext JSON in SQLite and exports them as plaintext JSON. The UI should avoid pretending they are protected.

Rationale: encrypted local secret storage is valuable but separate from validating the MCP management and connection path.

Alternative considered: block secret fields until encryption exists. This was rejected because many MCP servers require environment variables or headers to test real connectivity.

### Split Rust MCP Code into a Dedicated Module

The backend will add:

```text
src-tauri/src/mcp/
  mod.rs
  models.rs
  service.rs
  connection.rs
  commands.rs
```

`lib.rs` will only declare the module, add migration SQL, add error variants, and register commands. CRUD, validation, import/export, and connection logic live under `mcp`.

Rationale: `lib.rs` is already large, and MCP introduces enough models and behavior to justify a dedicated boundary.

## Risks / Trade-offs

- [The `rmcp` API and feature flags may differ from the draft dependency example] -> Start implementation with a dependency spike that selects the actual crate version and compiles `stdio` and `sse` test paths before building the UI around them.
- [SSE support may require different `rmcp` features than stdio] -> Treat stdio and SSE as separate verification tasks and do not mark the change complete until both are real.
- [Plaintext secrets can leak through SQLite or export] -> Document the limitation in the UI/design and defer encrypted storage to a follow-up change.
- [Oneshot tests may be slow for heavy stdio servers] -> Use a fixed timeout and show duration; introduce persistent connections only when tool routing requires them.
- [Project path identity can change if the app is launched from a different directory] -> Use the backend current working directory for P1 and document that a future workspace registry may replace it.
- [Global unique names prevent user/project overrides] -> Prefer predictable P1 behavior over scope precedence rules; reconsider overrides later if users need them.
- [JSON text areas are less ergonomic than structured editors] -> Keep the data model structured so the UI can be upgraded without backend schema changes.

## Migration Plan

The change is additive. Existing users receive a new `mcp_servers` table on startup migration. Existing demo MCP data is replaced by service-backed UI data. No existing user data requires conversion.

Rollback removes the MCP settings implementation and commands; the unused `mcp_servers` table can remain without affecting existing agent or settings behavior.

## Open Questions

- Which concrete `rmcp` version and features will be selected after the dependency spike?
- What local MCP server should be used as the repeatable stdio verification target in development and CI?
- What SSE MCP endpoint should be used for repeatable verification, or should the test suite include a lightweight local SSE MCP server fixture?
