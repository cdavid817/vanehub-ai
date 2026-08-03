## Why

MCP connections currently cross untrusted process and network boundaries without consistently bounded lifecycle or payload handling: timed-out stdio children can outlive an operation, relay forwarding can hang, transport names do not match the protocol actually used, and child diagnostics or temporary relay configuration can expose secrets. These defects can leak resources and credentials, produce protocol-incompatible behavior, and let a faulty or malicious MCP server exhaust the desktop runtime, so the runtime contract needs to be hardened before MCP usage expands.

## What Changes

- Make MCP connection tests, native Agent tool calls, and managed relay sessions obey an end-to-end deadline, cancel pending I/O, terminate owned child processes, and await cleanup on success, failure, cancellation, and timeout.
- Correct the URL transport contract so legacy SSE and Streamable HTTP are represented and executed consistently across persisted configuration, import/export, the frontend service model, and native adapters. **BREAKING**: the existing `sse` value will no longer implicitly mean Streamable HTTP; existing persisted URL configurations must be migrated or explicitly classified so their effective behavior is preserved.
- Return protocol-compatible, bounded failures when relay upstreams time out, disconnect, return invalid responses, or exceed resource limits; relay shutdown must not wait indefinitely on an open input stream.
- Capture MCP child stderr instead of inheriting the native process sink, then pass bounded diagnostic summaries through unified logging and redaction without persisting raw credentials, headers, environment values, request bodies, or tool results.
- Protect invocation-scoped relay configuration files with unpredictable names, restrictive access, and cleanup on partial setup, normal completion, failure, cancellation, timeout, and stale-startup recovery. This does not change the existing P1 contract that user-saved SQLite and exported MCP `env`/`headers` values are plaintext.
- Enforce explicit size/count/depth budgets for discovered tool catalogs, tool names and descriptions, JSON schemas, tool arguments and results, HTTP response bodies, and imported configuration documents, with stable user-facing errors and safe telemetry classifications.
- Keep the frontend/backend service boundary intact. The desktop runtime performs real process, network, cleanup, and logging behavior; the Web/mock runtime and shared TypeScript contracts mirror transport validation and bounded-result semantics without launching real MCP servers.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `mcp-client-management`: Strengthen connection and relay lifecycle guarantees, define truthful SSE and Streamable HTTP semantics, protect transient relay secrets, and bound MCP configuration, discovery, and protocol data.
- `agent-mcp-tools`: Require native MCP tool invocations to validate bounded inputs and outputs, honor end-to-end cancellation and deadlines, and fully release their connection or child-process resources before completion.

## Impact

- Affects both runtime surfaces: shared MCP configuration/contracts and Web/mock validation change, while real protocol execution, process ownership, relay forwarding, temporary files, and persistent diagnostics remain desktop-runtime responsibilities.
- Native impact centers on `src-tauri/src/contexts/tooling/mcp/`, managed relay bootstrap/runtime adapters, platform process spawning, native Agent MCP tool dispatch, SQLite migration/normalization for URL transports, and unified-log integration.
- Frontend impact centers on MCP service types and Tauri/Web adapters, settings transport choices and validation, import/export diagnostics, and bounded error presentation; React components continue to use service interfaces and never call Tauri directly.
- Existing plaintext SQLite/export behavior remains compatible, but URL transport values and imported/exported transport semantics require migration and contract-test coverage.
- Verification requires lifecycle tests that assert process cleanup rather than only timeout return values, relay deadlock and protocol fixtures, size-limit and secret-redaction tests, Web/Tauri contract parity tests, and strict OpenSpec validation.
