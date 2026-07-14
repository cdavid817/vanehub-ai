## Why

VaneHub AI currently shows an MCP servers settings page backed by frontend demo data, but it has no real MCP client, persistence, connection testing, or import/export support. Developers need VaneHub to manage MCP server configurations alongside agent settings so local tools can be discovered consistently from the desktop app while the Web runtime remains usable through adapters.

## What Changes

- Add MCP server configuration management for `stdio`, `sse`, and reserved `streamable_http` transport types.
- Persist MCP server configuration, project scope, cached test status, and discovered tools in SQLite through the Tauri backend.
- Add user and project scopes, where project-scoped servers are tied to the current working directory absolute `project_path`.
- Keep MCP server names globally unique across all scopes and support renaming with conflict validation.
- Add real MCP test connection support for `stdio` and `sse` using the Rust MCP SDK (`rmcp`) with a fixed timeout.
- Cache test status and tool discovery results after explicit tests; status reads do not perform live network or process checks.
- Add Claude Desktop compatible import/export for `mcpServers`, with URL-only imports defaulting to `sse`.
- Add a frontend MCP service interface plus Tauri and Web adapters so React components do not call Tauri `invoke()` directly.
- Replace the demo MCP settings page with real configuration, testing, import, and export UI.
- Preserve `callTool` and `streamable_http` as future-facing interface/model concepts, but do not expose tool calling in the P1 UI.
- Document that secrets in `env` and `headers` are stored and exported in plaintext for P1, with encrypted storage deferred.

## Capabilities

### New Capabilities

- `mcp-client-management`: Defines MCP server configuration management, scoped persistence, connection testing, status caching, and Claude Desktop import/export.

### Modified Capabilities

- `settings-center-ui`: The MCP settings page changes from demo content to a service-backed management surface and settings page navigation must preserve mounted page state for stateful settings pages.

## Impact

- Frontend runtime: adds MCP types, service interface, Web mock adapter, Tauri adapter, and real MCP settings components.
- Desktop runtime: adds Tauri MCP commands, SQLite schema migration, and a modular Rust `mcp` backend.
- Web runtime: remains usable through a mock MCP adapter without native calls.
- Dependencies: adds `rmcp` and async runtime support required for real MCP client connection testing.
- Architecture: reinforces the existing Port/Adapter boundary by keeping `invoke()` inside Tauri service adapters only.
- Security: stores MCP `env` and `headers` plaintext in SQLite and exports them plaintext in Claude Desktop JSON until a later encryption change.
