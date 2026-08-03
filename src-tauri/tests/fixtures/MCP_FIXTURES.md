# MCP runtime fixtures

These Node fixtures are independent protocol peers for native MCP integration tests. They use only
Node built-ins and bind HTTP listeners to `127.0.0.1` on an ephemeral port when passed port `0`.

## Invocation

- stdio: `node mcp_stdio_server.cjs <mode>`
- Streamable HTTP: `node mcp_http_server.cjs 0 <mode>`
- legacy SSE: `node mcp_legacy_sse_server.cjs 0 <mode>`

HTTP fixtures print one `READY <url>` line. The stdio fixture reads newline-delimited JSON-RPC.
`mcp_fixture_probe.cjs` validates each peer independently under the Rust integration test's
20-second wall-clock guard.

## Behavior modes

All peers provide `normal`, `hang-*`, disconnect, malformed-frame, catalog boundary, and tool-result
boundary modes. Transport-specific modes add:

- stdio: oversized frame, bounded/secret stderr, and a launcher-created descendant. Set
  `VANEHUB_MCP_FIXTURE_DESCENDANT_PID_FILE` to capture the descendant pid. The test-only
  `fixture/shutdown` request cleans it up when probing the fixture itself.
- Streamable HTTP: JSON or SSE responses, notification `202`, session creation/reuse/DELETE,
  redirects, disconnects, oversized bodies/events, and delayed/failed/hanging DELETE.
- legacy SSE: endpoint negotiation, redirects, event-stream or message-endpoint disconnects,
  malformed/oversized events, and phase hangs.

The canonical mode inventory and reusable limit-plus-one payloads live in `mcp_fixture_data.cjs`.
That module covers every central contract boundary: import bytes/count, configuration
collections/bytes, protocol bytes, per-server and provider tool counts, tool name/description,
schema size/depth, argument size/depth, rendered result, and retained stderr.
