## Why

Native API agents (`launch_kind = "api"`, built up across `add-custom-agent-registration` → `add-agent-tool-execution` → `add-agent-context-compaction` → `add-agent-skill-support` → `add-agent-cross-session-memory` → `add-agent-lifecycle-management`) can currently only call a fixed 3-tool catalog: `shell`, `file`, and `remember`. Meanwhile VaneHub already has a full MCP server management feature (`mcp-client-management`) that lets users configure stdio/SSE MCP servers per user or per project — but those servers are only wired into CLI-based agents' subprocess launch flags. A user who configures an MCP server today gets nothing extra when talking to a native API agent, even though the whole point of MCP is to extend what any given agent can do. This change closes that gap: native API agents gain the ability to discover and call tools exposed by the user's configured, active MCP servers, alongside their existing fixed tool catalog.

## What Changes

- Add a live MCP tool-invocation path (`tools/call`) to the existing MCP connection infrastructure. Today `McpConnectionPort` only supports `test()` (a one-shot connect → `list_all_tools()` → disconnect used by the "Test Connection" button); this change adds a second one-shot operation, `call_tool()`, following the identical connect → `tools/call` → disconnect pattern — no connection pooling or persistent sessions in this phase.
- Extend the native API agent's tool catalog (`tool_catalog()`) to include dynamically-named entries sourced from the MCP servers visible to the current session's project folder (same visibility rule the CLI relay already uses: `scope = 'user' OR (scope = 'project' AND project_path = ?)`, filtered to `active` servers only). Catalog entries are built from each server's last cached "Test Connection" tool list, not a fresh live connection, so generation start-up time is unaffected by MCP server reachability.
- Extend `execute_tool_call()`'s dispatch to recognize MCP-sourced tool names (prefixed `mcp__<server-name>__<tool-name>`, matching the convention already used elsewhere in the MCP ecosystem) and route them to a live one-shot `call_tool()` invocation against the owning server, translating the result back into the existing `ToolExecutionOutcome` vocabulary — including surfacing connection/tool-call failures as a normal `is_error: true` outcome the model can see and react to, not a generation failure.
- Classify all MCP-sourced tool calls as `RequiresApproval` unconditionally (no auto-approve carve-out), since VaneHub has no way to know what an arbitrary third-party-defined MCP tool actually does — consistent with this codebase's existing fail-closed handling of unrecognized tool/operation names.
- Bridge the synchronous native-agent tool-execution thread (a raw `std::thread::spawn` OS thread, not a tokio task) to the async-only `rmcp` client using `tauri::async_runtime::block_on`, the same shared runtime `tauri::async_runtime::spawn`/`spawn_blocking` already use elsewhere in this codebase.
- Extend at least one MCP test fixture (`src-tauri/tests/fixtures/mcp_stdio_server.cjs` or `mcp_http_server.cjs`) to also implement `tools/call`, so the new call path has real subprocess-level test coverage (today both fixtures only implement `initialize`/`tools/list`).

## Capabilities

### New Capabilities
- `agent-mcp-tools`: Native API agents discover MCP-sourced tools alongside their fixed catalog and can invoke them through a live, one-shot MCP `tools/call`, gated by mandatory approval.

### Modified Capabilities
(none — this change is purely additive on top of `agent-tool-execution`'s tool catalog/dispatch integration points and `mcp-client-management`'s existing server configuration/visibility rules; no existing requirement changes.)

## Impact

- **Affected code (desktop runtime only)**: `contexts::agent_runtime` tool catalog and tool-execution dispatch (`application/tool_catalog.rs`, the `execute_tool_call` dispatch in the API process adapter); `contexts::tooling::mcp` application ports and infrastructure (`McpConnectionPort`, `RmcpConnectionAdapter`, `McpServerRepository::list_visible`).
- **Web/mock runtime**: `web-agent-client.ts` gains a deterministic simulated MCP tool-call entry in its tool-call simulation, matching the existing shell/file/remember simulation pattern, so the frontend contract stays identical across runtimes.
- **Test infrastructure**: one or both Node.js MCP test fixtures gain a `tools/call` handler.
- **No impact** to CLI-based agents' own MCP relay mechanism, to MCP connection pooling/caching (deliberately out of scope, one-shot only), to the `StreamableHttp` transport stub, or to the MCP servers management UI (add/edit/test/remove already fully built, untouched).
