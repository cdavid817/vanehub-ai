## Context

VaneHub consumes external MCP servers through three related paths:

1. Settings connection tests create a one-shot rmcp client, initialize the configured server, lists tools, and cache the result in SQLite.
2. Native API Agent tool calls build a provider catalog from cached tools, then create another one-shot connection when an approved tool is invoked.
3. Claude Code and Codex CLI can receive invocation-scoped configuration that starts the VaneHub executable in managed relay mode and forwards their MCP traffic to a configured stdio or URL server.

The current one-shot adapter wraps the operation future in `tokio::time::timeout`, but the rmcp service task owns the stdio transport. Dropping the outer future does not prove that the child and its descendants have exited. The current stdio relay only checks its timeout after its input thread finishes and later joins that thread, so a child or an open parent stdin can keep the relay alive indefinitely. The URL path labels Streamable HTTP as `sse`, while the reserved `streamable_http` value fails at runtime; the relay writes raw HTTP/SSE bodies to stdio rather than translating supported MCP framing.

Both stdio spawn paths inherit child stderr. Relay configuration files are written under a shared temporary directory with ordinary file creation and contain command environment values, HTTP headers, the database path, and tracing context. Protocol frames, HTTP bodies, imported documents, discovered catalogs, schemas, tool arguments, results, and stderr currently have no coherent application-level budget.

The design must preserve the React service boundary, the Tauri/Web adapter split, the existing rmcp and reqwest stack where it can enforce the required guarantees, and unified logging/redaction. SQLite and Claude-compatible export remain plaintext by the existing P1 contract; this change protects transient runtime copies but does not introduce a secret vault. Windows is the primary desktop target, so process-tree cleanup and current-user file access must work on Windows rather than relying only on Unix signals and modes.

## Goals / Non-Goals

**Goals:**

- Make completion mean that owned MCP work has stopped and all owned process, pipe, HTTP-session, task, and temporary-file resources have reached a terminal state.
- Apply one cancellation/deadline model to connection tests, native Agent tool calls, and managed relay requests.
- Give `sse` and `streamable_http` distinct, protocol-correct meanings while preserving the effective behavior of existing URL configurations through migration.
- Translate supported legacy SSE and Streamable HTTP responses into protocol-compatible stdio JSON-RPC messages in managed relay mode.
- Bound data at the earliest controllable ingress and isolate a malformed or oversized server from other servers and fixed Agent tools.
- Capture MCP child diagnostics without inheriting raw stderr and emit only bounded, redacted, correlated unified-log records.
- Protect and reliably clean all invocation-scoped configuration artifacts.
- Keep desktop and Web/mock service contracts compatible while leaving process and network ownership in Rust.

**Non-Goals:**

- Reusing persistent MCP connections or pooling processes across tests and tool calls.
- Redesigning workspace/project-scope identity, canonicalizing project paths, or changing current visibility rules.
- Solving stale status-cache invalidation, concurrent test last-writer races, or settings-page query fan-out except where invalid cached data must be bounded and isolated.
- Encrypting MCP environment variables or headers in SQLite or changing the explicit plaintext export contract.
- Removing the ability to configure arbitrary stdio commands or HTTP endpoints, or adding a new command/SSRF trust workflow.
- Building a real MCP backend for browser mode; Web remains a contract-faithful simulation.
- Adding a user-facing limits editor. Initial limits are centrally defined product safety limits.

## Decisions

### 1. Use an owned session and an absolute deadline, not a timeout around an unowned future

Introduce an infrastructure-owned `ManagedMcpSession` abstraction for stdio, legacy SSE, and Streamable HTTP. The owner retains every child/process-tree handle, rmcp service handle, stderr drain, request task, HTTP session id, and cancellation signal until shutdown finishes. Application ports receive an execution control value containing an absolute deadline and a clonable cancellation signal; each phase derives its remaining budget rather than starting a fresh timeout.

The public operation deadline includes cleanup. Normal protocol work stops before a reserved cleanup interval. Shutdown then performs, as applicable: stop accepting work, request cooperative rmcp cancellation, close stdin or the HTTP stream, send a bounded Streamable HTTP session `DELETE`, wait briefly, terminate the owned process tree if it remains, wait/reap it, finish bounded pipe drains, and await all non-blocking tasks. The operation is not marked succeeded, failed, cancelled, or timed out until this sequence reaches a terminal result.

On Windows, the platform process layer will place an MCP stdio child in a kill-on-close Job Object. On Unix-like targets it will use an owned process group with graceful termination followed by forced termination. This covers launchers that spawn descendants; killing only the direct child is insufficient. If rmcp's `TokioChildProcess` cannot expose the required ownership boundary, the adapter will use a small VaneHub-owned bounded stdio transport around explicitly spawned pipes rather than depend on drop timing.

Tool names, argument shape, argument size, and JSON depth are validated before session creation. Native Agent generation cancellation is passed through `AgentMcpToolPort` to the MCP call instead of only changing the outer operation status. Connection-test operation cancellation uses the same signal. No detached MCP task may outlive its owning operation.

Alternatives considered:

- Keeping `tokio::time::timeout` around the existing future was rejected because it bounds caller waiting but does not establish child ownership or prove cleanup.
- Relying on `kill_on_drop` alone was rejected because it is a fallback, not an awaitable process-tree shutdown contract.
- Giving each protocol phase a fresh 15-second timeout was rejected because cumulative work could exceed the advertised operation deadline.

### 2. Represent failures as typed classifications and keep payloads out of diagnostics

Add an internal `McpRuntimeError` classification with at least `validation`, `spawn`, `timeout`, `cancelled`, `protocol`, `upstream_http`, `limit_exceeded`, `transport`, and `cleanup`. Domain/application code maps it to the existing result-as-data shapes. Tauri DTOs gain an additive optional safe error code while retaining concise display text; the Web/mock adapter produces the same codes for equivalent validation and limit failures.

Cleanup errors can change an apparent success into a failure because successful tool data must not be reported while an owned child is still unaccounted for. Telemetry stores the classification, transport, safe server identity, phase, outcome, duration, truncation flag, and correlation ids, never raw headers, environment values, JSON-RPC bodies, schemas, arguments, results, or database paths.

Alternatives considered:

- Parsing existing free-form error strings was rejected because classification would be unstable and could accidentally retain upstream payloads.
- Replacing all public result DTOs with a new error union was rejected as unnecessary contract breakage; the safe code is additive.

### 3. Make persisted transport values truthful and reject unknown values

The domain keeps three explicit transports:

- `stdio`: newline-delimited JSON-RPC over an owned child process.
- `sse`: legacy MCP SSE, with the server event stream and negotiated message endpoint.
- `streamable_http`: MCP Streamable HTTP using POST responses that may be JSON or SSE and an optional `Mcp-Session-Id`.

Existing rows persisted as `sse` are transactionally reclassified to `streamable_http`, because that is the protocol the current release actually executes. A migration journal records only rows changed by this migration. Newly imported URL entries default to `streamable_http` when they carry no transport marker; explicit Claude-compatible `type: "sse"` maps to legacy SSE, while `type: "http"` or `type: "streamable_http"` maps to Streamable HTTP. Export writes the corresponding interoperable type marker so a VaneHub round trip remains unambiguous.

`TransportType::from_persisted` becomes fallible. Unknown database or API values no longer silently become `stdio`. Tauri and Web adapters use the same string union and validation rules. The settings UI labels legacy SSE and Streamable HTTP distinctly and does not offer a reserved option that is guaranteed to fail.

The one-shot adapter selects rmcp's protocol-specific transport where it provides bounded ownership hooks. Otherwise, a VaneHub transport wrapper supplies the framing and ingress limits. The relay configuration also carries distinct `Sse` and `StreamableHttp` targets instead of one generic `Http` target.

Alternatives considered:

- Reinterpreting `sse` as Streamable HTTP forever was rejected because persisted names, UI, import/export, and wire behavior would remain misleading.
- Deleting legacy SSE from the model was rejected because the confirmed main specification requires real SSE compatibility and existing integrations can still use it.
- Treating unknown persisted values as `stdio` was rejected because it can launch an unrelated command path from corrupt data.

### 4. Supervise relay traffic in both directions and translate HTTP framing

The stdio relay becomes a supervisor with separate bounded parent-to-child and child-to-parent pumps. It observes child exit, parent EOF, pump failure, cancellation, shutdown deadline, and the oldest in-flight request deadline independently; timeout checks never depend on the parent input reader finishing. JSON-RPC ids are tracked per direction so request telemetry closes on the corresponding response rather than when bytes are merely written. Notifications and server-initiated requests continue to pass through unchanged.

When any terminal condition occurs, the supervisor closes downstream input, completes the bounded shutdown sequence, flushes a protocol-compatible error when a request id is available, and exits helper mode. A real helper process is allowed to terminate without joining a worker blocked on its parent stdin after all owned child resources and telemetry are finalized; unit tests use injected cancelable pipes so they do not leak blocked threads into the test process. Helper mode never falls through into normal desktop bootstrap.

The Streamable HTTP relay performs the following translation per request:

- Preserves an established `Mcp-Session-Id` and sends a bounded session `DELETE` during orderly or cancellation shutdown.
- Treats `202 Accepted` with no response message as a successful notification acknowledgement and emits no blank JSON-RPC frame.
- Parses `application/json` as one bounded JSON-RPC message.
- Incrementally parses `text/event-stream`, enforcing line/event/total limits and forwarding each JSON `data` event as a newline-delimited stdio JSON-RPC message rather than copying raw SSE bytes.
- Converts timeout, invalid content type, malformed event data, oversized data, redirect, and non-success HTTP status into a safe JSON-RPC error associated with the originating id when possible, then records a typed diagnostic.

The legacy SSE relay maintains the negotiated event stream and message endpoint, forwards POST requests to that endpoint, parses SSE events incrementally, and applies the same request correlation, deadline, redirect, and size policies. No request or response body is buffered without a prior bound.

Alternatives considered:

- Continuing to copy raw HTTP bodies was rejected because SSE framing and HTTP error bodies are not stdio MCP messages.
- Applying one absolute 30-second lifetime to the entire relay helper was rejected because an idle but healthy Agent session may legitimately live longer. The deadline is per in-flight request plus a bounded shutdown deadline.
- Joining every forwarding worker was rejected because a standard-input read cannot be cancelled portably after the parent keeps the pipe open.

### 5. Enforce one centrally defined limits policy at every boundary

Create a Rust `McpLimits` value in the tooling application layer and matching TypeScript constants verified by contract tests. Initial fixed limits are:

| Data | Limit |
|---|---:|
| Import document | 1 MiB and 128 server entries |
| Args, env, or headers per server | 128 entries each; 256 KiB total serialized configuration |
| JSON-RPC frame, SSE event, or HTTP response body | 2 MiB |
| Discovered tools per server | 128 tools and 2 MiB serialized catalog |
| MCP tools admitted to one provider generation | 256 tools |
| Tool name / description | 256 bytes / 8 KiB |
| One input schema | 128 KiB and JSON depth 32 |
| Tool arguments | 256 KiB and JSON depth 32 |
| Rendered tool result returned to an Agent | 1 MiB |
| Retained child stderr for diagnostic summarization | 64 KiB |

Byte limits use UTF-8 encoded size; collection and JSON depth limits are checked separately. Lower aggregate limits override higher per-item limits. The stdio codec rejects an overlong frame while reading, and HTTP/SSE code checks `Content-Length` when present then streams at most limit plus one byte/event before failing. Frontend import checks the text length before `JSON.parse`; backend validation repeats the checks because frontend validation is not a trust boundary.

An oversized connection-test catalog fails that server's test and does not persist the oversized payload. Reading legacy cached catalogs validates each server independently: one corrupt or oversized cache is excluded and logged without suppressing valid servers or the fixed `shell`/`file`/`remember` tools. Aggregate provider-catalog overflow is resolved deterministically by stable server/tool ordering; omitted overflow entries produce one bounded warning. Oversized tool arguments fail before connecting, and oversized results become an error outcome instead of being silently truncated into apparently successful data. Stderr alone is truncated because it is diagnostic, with the truncation flag recorded.

Alternatives considered:

- Limiting values only after deserialization was rejected because it does not prevent allocation-based denial of service.
- Silently truncating schemas, requests, or successful tool results was rejected because it changes protocol meaning and can mislead the model.
- Making all limits configurable in settings was deferred because it expands the public contract and makes safety guarantees harder to test.

### 6. Capture child stderr through a bounded unified-logging path

All MCP process constructors set stderr to a pipe. A concurrent bounded drain prevents child blockage and retains at most the configured diagnostic budget. The drain never writes directly to native stderr. On spawn, timeout, cancellation, protocol failure, non-zero exit, or cleanup failure, it sends a structured diagnostic to the unified logging service, which redacts before every configured sink. Successful noisy servers produce at most a rate-limited debug summary, not a transcript.

Command execution audit becomes structured: executable classification, argument count, transport, server identity, operation/run correlation, and outcome are recorded, while environment values and raw arguments are excluded unless a future explicit safe-field policy allows them. This satisfies MCP command auditing without using a string-concatenated command log that can expose secrets embedded in arguments.

The relay observer must finish and flush its bounded span metadata before helper termination. If normal unified logging is unavailable, the emergency path receives only an already-redacted fixed classification, never captured stderr or serialized configuration.

Alternatives considered:

- Inheriting stderr was rejected because it bypasses redaction and configured log routing.
- Redirecting stderr to null was rejected because it removes the diagnostics needed to investigate spawn and protocol failures.

### 7. Store relay artifacts in a private per-invocation directory with RAII cleanup

Relay artifacts move from the shared OS temp folder to a VaneHub application cache subdirectory. Each Agent invocation receives a `create_new` directory named with a process id and cryptographically unpredictable id. The platform filesystem adapter applies current-user-only access (Windows DACL; Unix directory mode `0700` and file mode `0600`) before any secret-bearing bytes are written; failure to apply the policy fails closed.

A `PreparedMcpRelayGuard` owns the per-server relay files and provider configuration from the moment each path is created. Partial `prepare_servers` failure, provider-argument construction failure, launch failure, normal exit, cancellation, and timeout all execute idempotent recursive cleanup after verifying the canonical target remains inside the dedicated relay root. The helper opens its own configuration and unlinks it immediately before parsing/connecting. Writes use `create_new`, bounded serialization, flush, and close; names are never reused.

At desktop startup, a scavenger examines only versioned VaneHub-owned invocation directories. It removes directories older than 24 hours after the same canonical-root check and emits metadata-only counts. It never scans or deletes arbitrary files directly under the system temp directory. Database and exported plaintext behavior remain unchanged.

Alternatives considered:

- Keeping random files in a shared temp directory was rejected because random names do not provide access control or reliable grouped cleanup.
- Passing the entire configuration in command-line arguments or environment variables was rejected because those values may be visible in process inspection or inherited by descendants.
- Best-effort cleanup from a collected vector was rejected because early iterator failure can lose ownership of already-created paths.

### 8. Preserve service boundaries and make Web behavior contract-faithful

Rust remains the authority for process, network, SQLite migration, limits enforcement, temporary files, and logging. Tauri commands return DTOs through `src/services/tauri-mcp-client.ts`; React settings components continue to call `McpService` and never import `invoke`. Shared TypeScript models retain `"stdio" | "sse" | "streamable_http"`, add optional safe error codes, and apply the same input validation and transport labels.

The Web/mock adapter does not create real transports. It must nevertheless reject unknown transports and oversized configuration/import/tool simulation values with the same safe codes, keep `sse` distinct from `streamable_http`, and clean up its in-memory mock status on rename/remove/disable so contract tests do not encode desktop-only assumptions. Native-only cleanup and ACL details are represented as successful simulated lifecycle events, not browser filesystem work.

Alternatives considered:

- Putting transport migration or limits in React was rejected because React is not a trust boundary and browser logic cannot own native resources.
- Adding transport-specific branches directly to settings pages was rejected in favor of typed service/adapters and reusable validation.

### 9. Verify terminal resource state, not only returned error shape

Lifecycle fixtures expose their child pid and deliberately hang during initialization, tool execution, response streaming, and shutdown. Tests assert that timeout/cancellation returns the stable error classification and that the process tree is gone within the public deadline. Relay tests use controllable open pipes to cover child exit while parent stdin remains open, parent EOF while a child hangs, request timeout, server-initiated messages, malformed frames, and shutdown races.

Protocol fixtures separately implement legacy SSE and Streamable HTTP JSON/SSE responses, session creation/deletion, notification `202`, redirects, non-success bodies, disconnects, and limit-plus-one payloads. Security tests verify per-platform access policy, cleanup after every injected failure point, stale-directory containment, stderr redaction, and absence of raw secrets in all sinks. Contract tests cover migration, import/export round trips, Tauri/Web transport parity, safe error codes, and per-server cache isolation.

The required project validation remains `npm run test`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `openspec validate --specs --strict`, with focused lifecycle tests run under a wall-clock guard so a deadlock fails rather than hanging CI.

## Risks / Trade-offs

- [Process-tree termination differs across operating systems and Job Object assignment can fail for unusual launchers] → Centralize ownership in the platform process layer, fail the MCP operation when containment cannot be established, and run Windows-specific descendant tests.
- [Legacy SSE behavior varies among older servers] → Implement the protocol-defined handshake and framing, use independent conformance fixtures, refuse redirects, and report a typed compatibility failure rather than falling back silently.
- [Migrating every historical `sse` row assumes it used the current Streamable HTTP implementation] → This preserves the only behavior VaneHub previously executed; journal exactly which rows changed and expose the corrected label after migration.
- [Fixed limits can reject a legitimate large catalog or result] → Use limits above normal provider payloads, return an explicit `limit_exceeded` code with the measured category, and tune constants only through a reviewed spec change and regression evidence.
- [Forced helper exit can skip ordinary Rust destructors] → Complete child shutdown, telemetry flush, and file unlink before the helper's terminal exit path; keep a parent guard and startup scavenger as independent cleanup layers.
- [Additional parsing and correlation increase relay complexity and CPU use] → Parse incrementally, retain metadata only, bound all maps/queues by the same policy, and keep protocol-specific relay components isolated behind one supervisor interface.
- [Changing public transport semantics can affect exports and automation] → Use an idempotent database migration, explicit import/export type markers, additive error fields, and contract fixtures for old and new configurations.
- [Failing success when cleanup fails may surface more errors to users] → Prefer resource integrity over false success, provide concise remediation text, and retain safe correlated diagnostics for investigation.

## Migration Plan

1. Add the typed error and limits models, owned process/session primitives, safe logging path, and protocol fixtures without switching production call sites.
2. Add an idempotent SQLite migration that journals and converts pre-change `sse` rows to `streamable_http`; make persisted transport decoding fallible and cover old/new database fixtures.
3. Switch one-shot tests and native tool calls to the owned session, propagate cancellation, validate before connection, and require terminal cleanup before operation completion.
4. Introduce distinct relay target variants and protocol translators, private per-invocation directories, RAII ownership, helper terminal behavior, and startup scavenging; then switch Claude/Codex invocation preparation.
5. Update TypeScript contracts, Tauri/Web adapters, settings labels/validation, and import/export mapping. Deploy backend migration and frontend contract changes in the same desktop release.
6. Enable limits at ingress, cache, catalog, and result boundaries and run the complete fault-injection and validation matrix.

Rollback requires stopping VaneHub, using the migration journal to revert only rows converted by this change from `streamable_http` to `sse`, and restoring the previous binary. Invocation cache directories are disposable and may be removed by either version after canonical-path verification. Additive DTO fields can be ignored by an older frontend, but a previous native binary must not be run against migrated transport rows without the down-migration because it treats `streamable_http` as reserved.

## Open Questions

No blocking design questions remain. The initial numeric limits are deliberate contract values; changing them after implementation requires benchmark/interoperability evidence and a corresponding spec review rather than an untracked constant edit.
