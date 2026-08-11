## Context

See `proposal.md` for motivation. The native API Agent currently runs its provider/tool loop on a background thread and conditionally adds workspace-scoped `search_code` through an Agent-owned outbound port. The retrieval context now owns stable code-index workspaces, Tree-sitter manifests, symbols, FTS/vector data, and targeted `reconcile_paths`, but successful Agent file writes do not notify that index. The desktop runtime already provides Tokio, managed child-process primitives, unified logging, SQLite migrations, and a bounded shutdown adapter.

LSP differs from retrieval: it is a live bidirectional protocol over a long-running external process, it must work when persistent code indexing is disabled, and its server can access anything allowed by the operating-system account. React must continue to use `AgentService`; only the Tauri adapter may invoke native commands, while Web mode must remain deterministic without local process or filesystem access.

## Goals / Non-Goals

**Goals:**

- Establish a separately owned native code-intelligence context with bounded process, protocol, configuration, trust, document, diagnostics, and query lifecycles.
- Make Rust and TypeScript/JavaScript semantic queries available to native API Agents without changing the synchronous provider/tool-loop contract or blocking the UI thread.
- Keep disk content authoritative and make exact Agent file mutations invalidate both live LSP documents and an enabled persistent code index.
- Fail soft for optional intelligence while distinguishing no result from warming, timeout, unavailable, stale, and protocol failure.
- Preserve workspace isolation at every application boundary and expose only bounded normalized results.

**Non-Goals:**

- Providing an editor buffer, unsaved frontend documents, formatting, completion, rename, code actions, call/type hierarchy, or persistent LSP semantic projections.
- Supporting Python, Go, Java, C/C++, dynamically downloaded servers, remote/SSH workspaces, or arbitrary URI schemes.
- Claiming that `workspaceFolders` or application checks sandbox a language-server process at the operating-system level.
- Adding a filesystem watcher. Exact Agent file mutations are signalled immediately; other disk changes are detected before a document query, with existing code-index audits remaining the persistent recovery path.
- Reporting exact server memory or indexed-file counts, which are neither portable nor standardized by LSP.

## Decisions

### 1. A new `code_intelligence` context owns LSP

The Rust runtime will add a `contexts/code_intelligence` bounded context with domain models, application ports/services, infrastructure adapters, and an API facade. It owns LSP configuration, workspace trust, server discovery, process registry, JSON-RPC state, document leases, diagnostics snapshots, and normalized query results.

Agent Runtime defines and consumes `AgentCodeIntelligencePort`; bootstrap implements it over `CodeIntelligenceApi`. Retrieval remains independently owned and is reached only through its public API by a separate mutation adapter. LSP does not extend `AgentRetrievalPort`, read retrieval repositories, or require a code-index workspace to exist.

Alternative: place LSP in retrieval because Tree-sitter already models languages and workspaces. Rejected because disabled indexes would disable live intelligence and because process/protocol state is not retrieval data.

Alternative: place LSP under generic tooling. Rejected because the primary domain is workspace code intelligence, not installation or configuration of an interchangeable external tool.

### 2. Configuration and trust are independent from code-index modes

SQLite stores one host-level LSP configuration with a master switch and per-language entries. Each entry contains enabled state, optional absolute executable override, server-specific fixed/default arguments, and a bounded JSON object of initialization options. Rust and TypeScript/JavaScript are the only accepted language families in this change. Executable discovery reuses the native CLI executable-location pattern without starting an interactive session.

Workspace trust is stored separately by canonical local root. A server may start only when the master switch, language switch, executable availability, local-workspace check, and explicit workspace trust all pass. Enabling local or semantic code indexing is not LSP trust because rust-analyzer can execute build scripts and procedural macros and TypeScript servers can load project code/plugins.

Changing an executable, arguments, initialization options, or trust revision changes a configuration fingerprint. Matching processes drain and restart rather than continuing with stale configuration.

Alternative: one global opt-in authorizes every repository. Rejected because opening an untrusted repository would silently start a process with the user's full OS permissions.

### 3. The process registry is keyed by workspace, project root, server, and configuration

One workspace can hold multiple projects of the same language. A process key contains canonical session root, detected project root, server kind, and configuration fingerprint. Rust roots are detected from `Cargo.toml`; TypeScript/JavaScript roots use the nearest `tsconfig.json`, `jsconfig.json`, or `package.json`. Upward traversal stops at the canonical session boundary. Files without a marker use the session root.

The lifecycle is:

```text
absent -> starting -> initializing -> ready -> stopping -> absent
                    \-> backoff -> starting
                    \-> failed
```

An active trusted local session may prewarm enabled servers when a bounded language inventory or an available code-index manifest indicates a matching language. A tool call also starts its required server on demand. The manifest is only a hint; LSP activation never depends on code-index enablement.

Unexpected exit fails pending requests, clears document/diagnostics state, and enters exponential backoff with a bounded restart budget and cooldown. Ten minutes without an active request or document lease initiates idle shutdown.

Alternative: one server per language per session. Rejected because nested projects can require different roots and server state.

### 4. Tokio actors implement bounded bidirectional JSON-RPC over stdio

The implementation uses `tokio::process`, `serde_json`, the existing managed-process containment primitives, and `lsp-types` 0.97 for typed protocol payloads. It implements LSP Content-Length framing directly rather than adding a second process/runtime abstraction. Every frame, header block, stderr capture, pending-request map, outbound queue, and notification queue has a hard bound.

One reader task parses server output, one writer task serializes outbound messages, and an actor owns monotonically increasing numeric request ids plus pending responders. Responses are matched by id; requests can complete out of order. Cancellation sends `$/cancelRequest` for a real pending id and removes the local waiter after the bounded cleanup path. A ten-second default request deadline includes a cleanup reserve and remains cancellable by the Agent generation token.

Server notifications update diagnostics, progress, log counters, and safe status. Required server-to-client requests include `workspace/configuration`, capability registration/unregistration, and work-done progress creation. `workspace/applyEdit` is rejected in this read-only foundation, show-message requests receive a non-interactive response, and unknown methods receive MethodNotFound. Raw stdout is protocol only; bounded stderr is drained to prevent deadlock but is not persisted.

Alternative: treat the server as request-only. Rejected because TypeScript and other conforming servers issue client requests during normal initialization and operation.

### 5. Initialize capabilities are minimal and negotiated

The client advertises only capabilities it implements. It sends process id, client info, canonical workspace folder, supported position encodings, synchronization, definition, references, hover, diagnostics publication, configuration, dynamic registration, and work-done progress capabilities. It records the server's selected position encoding, synchronization mode, and supported semantic methods.

A successful `initialize`/`initialized` exchange means protocol-ready, not fully indexed. Work-done progress may set an observable `indexing` detail, but absence of progress does not block queries. A requested unsupported method returns `unavailable` without writing a protocol request.

Testing a configured server uses an isolated temporary minimal project, performs initialize/initialized and shutdown/exit, and reports discovery, spawn, initialize, and cleanup phases. It does not use `$/cancelRequest` or an arbitrary document request as a heartbeat.

### 6. Disk-authoritative document leases avoid an editor-buffer model

The client has no unsaved frontend buffer. Before a text-document query it resolves and canonicalizes the relative path, rejects hidden, non-file, oversized, binary, symlink-escaping, or outside-workspace targets consistently with existing Agent file safety, and reads a bounded UTF-8 snapshot from disk.

The first query sends `didOpen` with language id and version. Later queries hash the current disk snapshot. If content changed, the lease increments its version and sends a change matching the negotiated sync mode. Incremental synchronization uses one bounded contiguous replacement derived from the common prefix and suffix; full synchronization sends the bounded snapshot. Idle leases send `didClose` and release retained text.

Agent file-mutation signals invalidate a matching lease immediately. Changes made by shell commands, Git, or external editors are detected by the next query's hash check. This preserves query correctness without requiring a watcher in the foundation.

Model-visible positions and returned ranges are 1-based. The adapter converts to/from LSP's 0-based positions with the negotiated character encoding, defaulting to UTF-16 when the server does not select one. End positions remain exclusive internally and are normalized explicitly in tool results.

Alternative: maintain persistent editor-style buffers for every indexed file. Rejected because VaneHub does not own unsaved code and the memory cost would duplicate the code index.

### 7. Diagnostics are versioned snapshots, not synchronous RPC results

`textDocument/publishDiagnostics` replaces the cached snapshot for its URI. The cache records optional server version, matching local document version, receive time, counts, and whether the snapshot is stale. `get_diagnostics` first refreshes the disk-authoritative document lease, then returns a matching snapshot or waits within its request deadline for a newer publication. Timeout, warming, and stale snapshots remain distinguishable from a valid empty diagnostic list.

Diagnostics are bounded by count and message bytes. Related information is accepted only for file URIs inside the current workspace. The feature does not send diagnostic text to the embedding pipeline or persist it in unified logs; like other Agent tool results, a returned bounded diagnostic is visible to the current Agent/model and persisted with that chat tool call under existing tool-history behavior.

Alternative: invoke `textDocument/documentSymbol` as a health or diagnostic proxy. Rejected because it does not represent diagnostics and can be unsupported independently.

### 8. Agent tools use one normalized, fail-soft envelope

The fixed provider-neutral catalog gains conditional definitions for `find_definition`, `find_references`, `get_hover`, and `get_diagnostics`. They are offered when LSP is configured for a trusted current local workspace, even if a process is still absent or warming, so the first tool call can start it. They are included in normal and Plan Mode catalogs and classified as workspace/file read actions. Execution still rejects unknown or future mutating LSP operations in Plan Mode.

Every result includes `status`, server/language identity when resolved, document version, stale state, total, returned count, and truncation. Semantic locations normalize `Location`, location arrays, and `LocationLink`; only `file:` targets within the canonical session root survive. Definitions are capped at 20 and references at 50 while preserving the pre-truncation total. Hover markup, diagnostic messages, previews, and the entire serialized result have hard byte limits.

`ready` with an empty list means no result. `warming`, `timeout`, `unavailable`, or `failed` never masquerades as that success. Optional LSP failures produce a successful tool round-trip containing the degraded status rather than terminating the whole Agent generation.

The existing provider/tool loop remains synchronous. Its background generation thread waits on the actor responder with timeout and cancellation; Tokio owns the process I/O, so neither Tauri's UI thread nor the LSP actor is blocked by the synchronous Agent port.

### 9. Exact Agent writes publish non-blocking workspace-mutation signals

Agent Runtime defines an outbound mutation port. After a successful `file` write or `edit`, it publishes canonical workspace plus normalized relative path. Bootstrap fans the signal out to LSP lease invalidation and a new best-effort `CodeIndexApi` targeted-change signal.

The code-index signal uses a bounded coalescing queue consumed by its background worker; reconciliation never runs on the Agent tool thread. It maps canonical root to an enabled code-index workspace, ignores absent/disabled indexes, coalesces duplicate paths, and retains existing workspace generation cancellation. A failure cannot turn a successful file write into an Agent tool error.

Shell execution does not emit a fabricated path set. LSP detects relevant disk changes before its next query, while existing manual/periodic code-index reconciliation remains the broad recovery mechanism.

Alternative: call retrieval infrastructure directly from the file tools. Rejected because it violates context ownership and would block the tool path on parsing and storage.

### 10. Service adapters preserve desktop/Web isolation

Shared TypeScript contracts add LSP configuration, language settings, workspace trust, discovery result, test result, and server status models. `AgentService` exposes read/save configuration, trust mutation, discovery, server test, and status methods. `tauri-agent-client.ts` is the only frontend layer that invokes the corresponding commands.

The Web adapter stores bounded in-memory mock configuration and trust, returns deterministic discovery/test/status transitions, and never reads the host filesystem or starts a process. React settings components depend only on `AgentService`, use the existing data-fetching/form-validation foundations, and localize all visible text. Status polling is a correctness fallback; no unbounded event subscription is required.

Semantic tool execution remains internal to the Agent runtime rather than becoming a React-callable `AgentService` operation. Web mode provides a separate pure mock tool-result adapter for the four read-only tool names; it returns deterministic `unavailable` envelopes with the same serialized metadata and payload keys as native Agent tool results, without inspecting a requested path or performing filesystem, process, or network access.

### 11. Shutdown and logging use existing native boundaries

The desktop shutdown adapter asks the code-intelligence API to stop accepting requests, cancel pending calls, send `shutdown`, await its response, send `exit`, close stdin, and wait for each child. Servers are stopped concurrently under a global bounded deadline; remaining process trees are terminated through the managed-process primitive. Cleanup is idempotent and config disable/trust revoke uses the same per-process path.

Unified logging records only level, category, safe server/language id, lifecycle transition, request method category, duration, counts, restart attempt, timeout/cancellation category, exit code, and safe workspace identifier when available. It excludes protocol payloads, hover/diagnostic/source content, raw stderr, environment, executable arguments, credentials, and private absolute paths. Repeated crash/timeout diagnostics are rate limited.

## Risks / Trade-offs

- [A trusted language server can execute or inspect more than the workspace] -> Default the master switch off, require per-workspace trust, show the trust boundary in settings, validate Agent-visible URIs, and avoid claiming OS sandboxing.
- [rust-analyzer or TypeScript initialization can remain busy after protocol readiness] -> Expose warming/indexing detail, prewarm trusted active workspaces, keep requests bounded, and distinguish transient status from empty results.
- [A malformed or noisy server can exhaust memory] -> Bound frames, queues, pending ids, retained documents, diagnostics, stderr, and normalized tool output; terminate the offending process on protocol-limit violation.
- [Crash restart loops consume resources] -> Use exponential backoff, a restart budget, cooldown, and explicit status/test controls.
- [No watcher means background diagnostics can lag external edits] -> Hash before every requested document operation, invalidate exact Agent writes immediately, mark version-mismatched diagnostics stale, and defer watcher-based continuous diagnostics.
- [Code-index targeted reconciliation adds contention] -> Coalesce into a bounded worker queue and never parse or write SQLite on the Agent tool thread.
- [Project-root heuristics are imperfect in monorepos] -> Choose the nearest bounded marker, expose the resolved root in safe relative form, and key processes so later root-selection improvements do not change stored source data.
- [Tool schemas increase provider prompt size] -> Limit the foundation to four high-value read-only tools and register them only for configured trusted local workspaces.

## Migration Plan

1. Add additive SQLite tables/keys for LSP configuration and workspace trust with the master switch and every language disabled by default; migration starts no process and reads no source file.
2. Add the context, process/protocol implementation, and bootstrap lifecycle behind disabled configuration.
3. Add service contracts, commands, Tauri/Web adapters, settings UI, and isolated server testing.
4. Add Agent tools and mutation fan-out only after workspace isolation, protocol bounds, Plan Mode rejection, and Web parity tests pass.
5. Rollback disables tool registration and startup while leaving additive configuration rows ignored. Shutdown remains callable so running processes are terminated before an older runtime takes over.

## Open Questions

- The initial default initialization options for rust-analyzer and TypeScript will be conservative constants and can be tuned after measured startup/resource testing without changing the behavioral contracts.
- The exact frame and aggregate tool-output byte limits will reuse the nearest existing native protocol/tool limits where practical and can be lowered by implementation tests without changing the requirement that they remain hard bounded.
