## 1. Runtime Contracts and Safety Primitives

- [x] 1.1 Add the internal `McpRuntimeError` classification and safe display/error-code mappings for `validation`, `spawn`, `timeout`, `cancelled`, `protocol`, `upstream_http`, `limit_exceeded`, `transport`, and `cleanup`, with unit tests that prove upstream payload text is not used as a code.
- [x] 1.2 Add the central Rust `McpLimits` policy and reusable byte-count, collection-count, serialized-size, and JSON-depth validators for every value defined in the delta specs, with boundary and limit-plus-one tests.
- [x] 1.3 Add a clonable MCP execution-control model carrying an absolute deadline and cancellation signal, then update application ports so tests, tool calls, and relay requests derive remaining time from that value rather than creating independent timeouts.
- [x] 1.4 Extend Rust MCP result/DTO mappings with additive optional safe error codes while preserving existing concise error text and result-as-data behavior; add mapper serialization tests.

## 2. Truthful Transport Model and Persistence Migration

- [x] 2.1 Make persisted and API transport decoding fallible, preserve distinct `stdio`, legacy `sse`, and `streamable_http` values, and add tests proving unknown values never fall back to `stdio`.
- [x] 2.2 Add the next idempotent SQLite migration and migration journal that transactionally reclassify historical `sse` rows as `streamable_http`; update migration-count assertions and database schema fixtures.
- [x] 2.3 Add migration tests for old databases, already-migrated databases, mixed transports, transaction failure, journal-based down migration, and reopen behavior.
- [x] 2.4 Update Claude-compatible native import mapping so command entries remain `stdio`, explicit `sse` stays legacy SSE, `http`/`streamable_http` becomes Streamable HTTP, and untyped URL entries preserve historical behavior as Streamable HTTP; cover conflicts and invalid types.
- [x] 2.5 Update native export mapping to emit unambiguous compatible URL type markers while excluding VaneHub and migration metadata; add stdio/SSE/Streamable HTTP round-trip tests.

## 3. Owned Process Trees and Diagnostic Containment

- [x] 3.1 Introduce a platform managed-child API that owns piped stdin/stdout/stderr, exposes bounded wait and shutdown, always reaps the direct child, and replaces MCP uses of `spawn_piped` or hidden child ownership.
- [x] 3.2 Implement Windows Job Object containment with kill-on-close semantics for MCP process descendants and add Windows-specific tests for launcher-created child processes and containment failure.
- [x] 3.3 Implement Unix process-group containment with bounded graceful termination followed by forced termination, guarded by target-specific tests.
- [x] 3.4 Implement the concurrent 64 KiB stderr drain with truncation metadata and prove that a noisy child cannot block while raw stderr is never inherited by the VaneHub process.
- [x] 3.5 Route MCP spawn, exit, timeout, cancellation, protocol, and cleanup diagnostics plus structured command audit through unified logging with correlation and redaction; test that raw args, env, headers, bodies, schemas, tool data, and relay configuration never reach normal or emergency sinks.

## 4. Managed One-Shot MCP Sessions

- [x] 4.1 Implement `ManagedMcpSession` ownership and the bounded shutdown sequence for service handles, child/process-tree handles, pipe drains, HTTP streams, session ids, and spawned tasks, including success-to-cleanup-error conversion.
- [x] 4.2 Implement or wrap a newline-delimited stdio transport that rejects frames above 2 MiB while reading and remains explicitly owned by `ManagedMcpSession`.
- [x] 4.3 Implement a real legacy SSE one-shot client path with negotiated message endpoint, incremental bounded event parsing, redirect refusal, and absolute deadline/cancellation support.
- [x] 4.4 Harden the Streamable HTTP one-shot client path with bounded streamed bodies, JSON and SSE response handling, `Mcp-Session-Id` lifecycle, bounded `DELETE`, redirect refusal, and absolute deadline/cancellation support.
- [x] 4.5 Refactor connection testing and direct MCP tool invocation onto managed sessions, validate tool names/argument shape/size/depth before connecting, and require cleanup before returning a terminal result.
- [x] 4.6 Connect MCP test-operation cancellation and native Agent-generation cancellation to execution control, and add tests proving no detached task, HTTP request, child, or descendant remains after success, failure, cancellation, or timeout.

## 5. Configuration, Discovery, Cache, and Catalog Limits

- [x] 5.1 Enforce the 128-entry and 256 KiB per-server configuration limits at the Rust domain/application boundary before persistence or process/network launch, with exact-boundary tests.
- [x] 5.2 Enforce tool count, name, description, schema size/depth, and 2 MiB serialized discovery limits before a successful test result is cached; verify an oversized result records `limit_exceeded` without replacing the prior valid cache.
- [x] 5.3 Validate cached catalogs per server on read so one malformed or oversized row is excluded with one safe diagnostic while valid MCP servers and fixed Agent tools remain available.
- [x] 5.4 Apply stable server-name/tool-name ordering and the 256-entry aggregate MCP provider-catalog cap, preserving all fixed tools and emitting one bounded overflow warning; add deterministic ordering tests.

## 6. Private Relay Artifact Lifecycle

- [x] 6.1 Add a platform filesystem helper that creates the versioned per-invocation relay cache directory and secret-bearing files exclusively with current-user-only Windows ACLs or Unix `0700`/`0600` permissions, failing closed when access control cannot be applied.
- [x] 6.2 Implement `PreparedMcpRelayGuard` so ownership is acquired immediately for each created path and canonical-root-checked cleanup is idempotent on partial preparation, provider-argument failure, launch failure, normal completion, cancellation, and timeout.
- [x] 6.3 Update Claude/Codex invocation preparation and relay helper startup to use bounded `create_new` files, unlink a consumed relay file before connecting, finalize telemetry before helper termination, and never fall through into desktop bootstrap.
- [x] 6.4 Add the 24-hour startup scavenger for versioned VaneHub-owned relay directories plus permission, symlink/junction containment, partial-failure, helper-consumption, stale-cleanup, and unrelated-file preservation tests.

## 7. Protocol-Correct Managed Relay

- [x] 7.1 Split relay configuration targets into explicit stdio, legacy SSE, and Streamable HTTP variants and update provider argument serialization/deserialization tests.
- [x] 7.2 Add bounded JSON-RPC frame parsing and bidirectional id/method correlation so spans finish on matching responses while notifications and server-initiated requests pass through unchanged.
- [x] 7.3 Replace the stdio relay join loop with a supervisor that independently observes parent EOF, child exit, pump failure, cancellation, oldest in-flight request deadline, and shutdown deadline, then terminates/reaps the upstream tree without blocking on open parent stdin.
- [x] 7.4 Implement Streamable HTTP relay translation for bounded `application/json`, incremental `text/event-stream`, notification `202`, session headers, and newline-delimited stdio output without forwarding raw SSE control lines.
- [x] 7.5 Implement legacy SSE relay negotiation, bounded event-stream parsing, message endpoint POST forwarding, bidirectional correlation, and deterministic teardown.
- [x] 7.6 Map timeout, disconnect, redirect, non-success HTTP status, invalid content type/framing, and size overflow to request-associated protocol-compatible errors and safe telemetry; cover open-stdin deadlocks, server requests, `DELETE`, malformed data, and limit-plus-one fixtures.

## 8. Native Agent MCP Tool Integration

- [x] 8.1 Thread the existing Agent generation cancellation signal through `AgentMcpToolPort`, `RuntimeAgentMcpToolAdapter`, `McpApi`, and the connection port without changing explicit approval or call-time visibility checks.
- [x] 8.2 Enforce the 256 KiB/depth-32 argument budget before connection and the 1 MiB rendered-result budget before success, returning safe error outcomes rather than silently truncating protocol data.
- [x] 8.3 Make timeout, cancellation, limit, transport, tool-level, and cleanup failures remain tool execution data so an uncancelled generation can continue, while generation cancellation prevents further tool-loop work.
- [x] 8.4 Add native Agent regression tests for approval denial with no connection, out-of-scope rejection, malformed arguments with no connection, cancellation during each MCP phase, cleanup failure after remote success, non-text placeholders, catalog isolation, and oversized results.

## 9. Frontend Service and Web/Mock Parity

- [x] 9.1 Update shared TypeScript MCP types, limit constants, safe error-code union, Tauri adapter result mappings, and contract-conformance tests while keeping every `invoke()` call confined to `tauri-mcp-client.ts`.
- [x] 9.2 Update reusable MCP form/service validation for truthful transport labels, unknown types, collection counts, serialized configuration size, tool argument depth, and string-only args/env/header values.
- [x] 9.3 Enforce the 1 MiB import-text limit before `JSON.parse`, the 128-server limit after parsing, explicit URL transport mapping, and concise per-entry validation/storage feedback through the service boundary.
- [x] 9.4 Fix the Web/mock MCP adapter to distinguish legacy SSE from Streamable HTTP, return matching validation/limit codes, and correctly migrate or remove in-memory status on rename, disable, remove, import, and re-add without simulating native side effects.
- [x] 9.5 Update MCP settings labels, import/export presentation, safe failure rendering, and translations; add focused Vitest coverage for both runtime adapters, validation boundaries, Web state transitions, and user-visible transport/error behavior.

## 10. Integration Verification and Release Evidence

- [x] 10.1 Add independent stdio, legacy SSE, and Streamable HTTP fixtures covering initialize/list/call, JSON and SSE responses, notification `202`, session creation/deletion, redirects, disconnects, invalid frames, stderr secrets, hangs, descendants, and every limit-plus-one boundary.
- [x] 10.2 Add invocation-scoped integration tests for Claude- and Codex-shaped provider configuration through the VaneHub relay helper to each supported upstream transport, asserting protocol output, correlation, process-tree exit, and artifact cleanup under a wall-clock guard.
- [x] 10.3 Add failure-injection tests at every relay artifact creation, session startup, protocol phase, persistence, logging, cancellation, and cleanup boundary, and record evidence that no secret-bearing file, child, descendant, task, or raw diagnostic remains.
- [x] 10.4 Run `npm run lint`, `npm run test`, `npm run contracts:check`, and `npm run build`, fixing all failures without weakening strict TypeScript or lint rules.
- [x] 10.5 Run `cargo fmt --all -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml`, `openspec validate harden-mcp-runtime-reliability --strict`, and `openspec validate --specs --strict`, then record the final command results in the change verification evidence before requesting archive.
