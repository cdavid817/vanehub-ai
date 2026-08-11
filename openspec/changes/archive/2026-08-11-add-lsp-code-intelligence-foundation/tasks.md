## 1. Contracts, dependencies, and persistence

- [x] 1.1 Add failing Rust architecture tests for the new `code_intelligence` context boundary, its public API-only cross-context access, and the prohibition on direct retrieval infrastructure imports
- [x] 1.2 Add the reviewed `lsp-types` dependency and create the domain/application/infrastructure/API module skeleton without starting any process
- [x] 1.3 Define Rust domain models for language/server identity, configuration fingerprints, workspace trust, process states, negotiated capabilities, document versions, diagnostics snapshots, normalized locations, and fail-soft query outcomes with unit tests
- [x] 1.4 Add an additive SQLite migration for disabled-by-default LSP configuration and canonical-workspace trust records, including migration fixture and schema-version tests
- [x] 1.5 Implement configuration and trust repositories with validation for language ids, absolute executable overrides, bounded initialization-option objects, canonical roots, and atomic preservation of the last valid configuration

## 2. Bounded JSON-RPC transport

- [x] 2.1 Add protocol tests for Content-Length framing across partial reads, multiple frames, mixed header casing, invalid headers, unexpected EOF, and exact/over-limit payloads
- [x] 2.2 Implement the bounded asynchronous LSP frame reader and serialized writer over child stdout/stdin
- [x] 2.3 Add actor tests for monotonic request ids, out-of-order responses, notifications, server-to-client requests, unknown methods, queue bounds, and pending-map cleanup
- [x] 2.4 Implement the JSON-RPC actor with concurrent correlation, bounded channels, typed payload conversion, and safe protocol-error categories
- [x] 2.5 Implement and test server-to-client handling for workspace configuration, capability registration/unregistration, work-done progress creation, non-interactive messages, rejected workspace edits, and MethodNotFound responses
- [x] 2.6 Add timeout and generation-cancellation tests, then implement real-id `$/cancelRequest` delivery and bounded local cleanup
- [x] 2.7 Add a managed stdio child adapter that contains process trees, drains bounded stderr without persisting it, detects exit, and tears down all protocol tasks on failure

## 3. Discovery, project roots, and server lifecycle

- [x] 3.1 Add executable discovery and command-preset tests for `rust-analyzer` and `typescript-language-server --stdio`, including missing and invalid manual overrides
- [x] 3.2 Implement server discovery through the native executable-location boundary without launching persistent or interactive sessions
- [x] 3.3 Add project-root tests for nearest Cargo, tsconfig, jsconfig, and package markers, nested projects, marker-less files, Windows canonicalization, and attempts to traverse above the session root
- [x] 3.4 Implement bounded Rust and TypeScript/JavaScript project-root resolution and process-key construction from workspace, project root, server kind, and configuration fingerprint
- [x] 3.5 Add process-pool state-machine tests for on-demand start, prewarm hints, concurrent acquisition, config replacement, trust revocation, unexpected exit, exponential backoff, restart exhaustion, cooldown, and ten-minute idle closure
- [x] 3.6 Implement the process registry and lifecycle coordinator with bounded restart policy, idle cleanup, status snapshots, and optional code-index manifest hints that never become an activation dependency
- [x] 3.7 Add initialize negotiation tests for declared client capabilities, UTF-16 fallback, sync modes, unsupported methods, background progress, and malformed initialize results
- [x] 3.8 Implement initialize/initialized capability negotiation and preserve protocol-ready state separately from optional warming/indexing progress
- [x] 3.9 Implement isolated minimal-project server testing with phase-specific results and guaranteed shutdown/exit/process cleanup
- [x] 3.10 Integrate concurrent graceful LSP shutdown into the desktop lifecycle adapter and test idempotence, global deadline enforcement, and forced process-tree termination

## 4. Disk-authoritative documents and diagnostics

- [x] 4.1 Add document-admission tests for relative paths, canonical containment, hidden components, symlink escape, non-files, binary content, invalid UTF-8, and hard file-size limits
- [x] 4.2 Implement bounded disk snapshot resolution and language identification without importing code-index infrastructure
- [x] 4.3 Add position-conversion tests for 1-based Agent coordinates, 0-based LSP coordinates, UTF-8/UTF-16 encodings, surrogate pairs, combining characters, exclusive end positions, and invalid ranges
- [x] 4.4 Implement negotiated position conversion and normalized 1-based result ranges
- [x] 4.5 Add document-lease tests for didOpen, unchanged reuse, full synchronization, single-contiguous incremental synchronization, version increments, exact mutation invalidation, external disk changes, didClose, and server restart
- [x] 4.6 Implement bounded disk-authoritative document leases with hashing, negotiated synchronization, idle closure, and retained-text release
- [x] 4.7 Add diagnostics-cache tests for replacement, empty current snapshots, missing server versions, stale local versions, bounded waiting, related outside URIs, count/message caps, and cleanup after process exit
- [x] 4.8 Implement version-aware publishDiagnostics caching and bounded current/stale/warming/timeout query behavior

## 5. Agent tools and workspace mutation fan-out

- [x] 5.1 Define `AgentCodeIntelligencePort` and `AgentWorkspaceMutationPort` consumer-side contracts with no model-selectable workspace or server scope
- [x] 5.2 Add tool-catalog tests for conditional registration of `find_definition`, `find_references`, `get_hover`, and `get_diagnostics` in normal and Plan Mode generations, including unavailable, untrusted, remote, and code-index-disabled workspaces
- [x] 5.3 Add provider-neutral schemas for the four tools and map them to workspace/file read permission actions while rejecting unknown or mutating LSP operations
- [x] 5.4 Add execution tests for null/single/array/location-link definitions, deterministic reference ordering and top-50 truncation, bounded previews, hover markup normalization, diagnostic snapshots, outside-workspace filtering, and every fail-soft status
- [x] 5.5 Implement synchronous Agent-port adapters over asynchronous actor responders with generation cancellation, hard output limits, visible/persisted tool outcomes, and no UI-thread blocking
- [x] 5.6 Add Plan Mode regression tests proving all four read-only tools execute while shell, file writes, edit, MCP, workspace edits, and unadvertised mutating LSP operations remain rejected
- [x] 5.7 Add failing tests that successful `file` writes and `edit` calls publish one normalized mutation while failed/denied operations publish none and notification failure never changes the tool result
- [x] 5.8 Implement the non-blocking bootstrap mutation fan-out to LSP lease invalidation and the public code-index API
- [x] 5.9 Add a bounded coalescing code-index mutation queue that maps canonical roots to enabled workspaces and invokes targeted reconciliation on the background worker with generation and admission semantics preserved

## 6. Native commands and frontend service parity

- [x] 6.1 Add command DTO and serialization tests for configuration, trust, discovery, server tests, status, language ids, process states, safe reason codes, timestamps, and optional negotiated capabilities
- [x] 6.2 Implement one-file-per-command Tauri endpoints for get/save configuration, list/update workspace trust, discover servers, test a server, and list server status, then register them in the command registry
- [x] 6.3 Add strict TypeScript contracts and runtime normalizers for every LSP configuration, trust, discovery, test, and status payload without `any` or unchecked assertions
- [x] 6.4 Extend `AgentService` and `tauri-agent-client.ts` with the LSP operations, keeping all `invoke()` calls inside the Tauri adapter
- [x] 6.5 Implement bounded in-memory Web/mock configuration and trust plus deterministic discovery, test, and lifecycle-status transitions without filesystem, process, or network access
- [x] 6.6 Add adapter-conformance tests proving Tauri and Web clients expose the same method and contract surface and invalid native payloads fail closed

## 7. Settings and status experience

- [x] 7.1 Add localized LSP settings copy for all supported locales, including explicit workspace-trust and non-sandboxed-process explanations
- [x] 7.2 Build service-backed LSP configuration controls for the master switch, two language switches, discovered/manual executables, and schema-validated bounded initialization options using Tailwind and existing UI primitives
- [x] 7.3 Build trusted-workspace management and isolated server-test feedback with loading, success, safe failure, revocation, and retry states
- [x] 7.4 Build the runtime status panel for safe server/language identity, relative project root, lifecycle, restarts, last response, diagnostics, capability summary, and unsupported metric messaging
- [x] 7.5 Add component tests for loading/error/empty/configured states, invalid initialization JSON, trust disclosure, adapter-only calls, polling cleanup, keyboard use, and accessible labels
- [x] 7.6 Add Web/mock integration tests covering configuration persistence, trust transitions, deterministic server testing, and status refresh without native side effects
- [x] 7.7 Add Playwright coverage for enabling LSP, validating configuration, trusting/revoking a workspace, testing a server through Web mode, and observing deterministic status transitions

## 8. Security, observability, and regression guards

- [x] 8.1 Add unified-log tests proving lifecycle, protocol limit, timeout, cancellation, crash, restart, and shutdown events retain safe metadata while raw payloads, diagnostics, hover/source content, stderr, environment, arguments, credentials, and absolute paths are redacted or omitted
- [x] 8.2 Implement rate-limited correlated LSP diagnostics through the unified logging service without feature-local files or raw stderr persistence
- [x] 8.3 Add adversarial tests for absolute/traversal/hidden/symlink paths, non-file URI results, oversized frames and results, response-id spoofing, duplicate ids, notification floods, malicious initialization options, and workspace-scope injection
- [x] 8.4 Add native architecture tests ensuring React never invokes LSP commands directly, Web mode cannot reach native process/filesystem adapters, and code intelligence/retrieval communicate only through public or consumer-owned ports
- [x] 8.5 Add deterministic lifecycle and data-structure tests for queue, cache, pool, and truncation bounds without shared-host wall-clock performance assertions

## 9. Documentation and full validation

- [x] 9.1 Document installation, trust, supported servers, Rust/TypeScript setup, lifecycle states, limitations, troubleshooting, and the distinction between Tree-sitter `search_code` and live LSP tools in English and Chinese user/developer guides
- [x] 9.2 Run `npm run docs:check` and fix every Markdown/CommonMark issue
- [x] 9.3 Run `npm run lint:ci` and fix every lint or 300-line production-file violation
- [x] 9.4 Run `npm run test` and `npm run test:coverage`, then satisfy the enforced coverage thresholds
- [x] 9.5 Run `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`
- [x] 9.6 Run `npm run build` and fix all strict TypeScript or production-bundle failures
- [x] 9.7 Run `npx playwright test` and fix all UI behavior regressions
- [x] 9.8 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 9.9 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 9.10 Run `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 9.11 Run `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 9.12 Run `openspec validate add-lsp-code-intelligence-foundation --strict` and `openspec validate --specs --strict`

## 10. Pre-archive production wiring remediation

- [x] 10.1 Add failing native production-wiring tests proving a configured trusted workspace injects a concrete code-intelligence responder, advertises all four read-only tools in normal and Plan Mode generations, and sends the first semantic request to the managed stdio fixture
- [x] 10.2 Implement a production `AgentCodeIntelligenceResponderPort` backed by the code-intelligence API, inject it through bootstrap into `RuntimeAgentApiAdapter`, and keep the unavailable responder only as an explicit fallback for runtimes without native LSP support
- [x] 10.3 Wire on-demand and hinted prewarm process creation through project-root detection, the process registry, initialize negotiation, active-process shutdown registration, configuration replacement, trust revocation, restart backoff, and idle cleanup
- [x] 10.4 Wire the four semantic query paths through disk snapshot admission, invalidation draining, document leases, negotiated position conversion and synchronization, cancellable JSON-RPC requests, workspace filtering, and bounded result normalization
- [x] 10.5 Route server notifications into version-aware diagnostics caches, consume document invalidations before queries, and expose real bounded server-status snapshots instead of an unconditional empty list
- [x] 10.6 Emit lifecycle, protocol-limit, timeout, cancellation, crash, restart, diagnostics-count, and shutdown events from the production coordinator through the unified rate-limited redacted LSP logger
- [x] 10.7 Add native end-to-end regression coverage using the managed stdio fixture for tool availability and execution, diagnostics, status transitions, configuration replacement, trust revocation, process cleanup, and graceful desktop shutdown
- [x] 10.8 Re-run every repository validation command required by `AGENTS.md`, the LSP Playwright suite, strict change validation, and strict main-spec validation

## 11. Pre-archive verification remediation

- [x] 11.1 Replace the native LSP end-to-end fixture's fixed short readiness and cleanup polling loops with bounded deadline-based waits that report the last observed status, then prove the test remains stable under repeated execution
- [x] 11.2 Define the shared serialized LSP tool-result contract and add deterministic Web/mock stubs for all four read-only tools, with contract tests proving unavailable empty results and no host-side effects
