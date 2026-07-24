## 1. Runtime and persistence foundations

- [x] 1.1 Review and select a cross-platform Rust SSH dependency that supports host-key callbacks, password/key authentication, PTY channels, exec channels, resize, keepalive, and multiplexing; record the pinned choice in the design.
- [x] 1.2 Define and test bounded constants for pool capacity, idle/drain timeouts, capture queue/chunk sizes, output retention, global content capacity, and search pagination.
- [x] 1.3 Add an additive migration for SSH profile revision, host trust, remote session profile binding, command templates, command runs, output chunks, capture settings, and the FTS5 index.
- [x] 1.4 Add empty, current, and pre-change migration tests covering FTS availability, existing remote sessions, foreign-key/delete behavior, and multilingual/path search fixtures.

## 2. SSH profile and session binding

- [x] 2.1 Extend Rust SSH profile domain, repository, DTOs, and Web models with revision and host-trust metadata without exposing credentials.
- [x] 2.2 Increment profile revisions only for endpoint or authentication changes and add tests for rename, credential replacement, update failure, and deletion.
- [x] 2.3 Extend session domain, persistence, commands, TypeScript contracts, and Web adapter with nullable SSH profile id and revision binding.
- [x] 2.4 Bind newly created remote sessions to selected or newly saved profiles while preserving independent workspace snapshots.
- [x] 2.5 Implement and test explicit remote-session bind/rebind behavior, stale revision detection, profile deletion behavior, and migration of unbound historical sessions.

## 3. Authenticated SSH and connection pooling

- [x] 3.1 Add native SSH application ports and infrastructure adapters for credential loading, host-key verification, password/key authentication, PTY channels, exec channels, keepalive, and close.
- [x] 3.2 Implement first-seen and changed-host-key challenges with deduplication, explicit confirmation commands, trust persistence, and redacted diagnostics.
- [x] 3.3 Implement a single-flight connection pool keyed by connection id and revision with leases, health state, bounded capacity, and idle eviction.
- [x] 3.4 Implement profile-edit draining, profile-delete closure, transport-failure propagation, and application-shutdown cleanup.
- [x] 3.5 Add Rust unit and integration tests proving compatible transport reuse, incompatible-profile isolation, independent channels, eviction, draining, and failure propagation.

## 4. Remote Shell integration

- [x] 4.1 Extend the Shell workspace projection to return remote endpoint, path, binding, and policy context rather than a remote boolean alone.
- [x] 4.2 Route Shell creation to the existing local PTY runtime or the new SSH PTY runtime behind the workspace application boundary.
- [x] 4.3 Implement remote input, resize, reset-directory, output/state events, disconnect, session cleanup, and transport-lease release.
- [x] 4.4 Preserve responsive live output and bounded in-memory reattach content without writing raw PTY input to persistence.
- [x] 4.5 Add Rust and frontend adapter tests for connected, unbound, stale, trust-required, authentication-failed, disconnected, and shared-transport Shell states.

## 5. Command templates and quick execution

- [x] 5.1 Add Rust domain validation and SQLite repositories for global, connection, and workspace command templates with secret-pattern rejection.
- [x] 5.2 Add service operations and Tauri commands for template list, create, update, delete, scope filtering, and insertion.
- [x] 5.3 Implement SSH exec-channel quick runs with immutable command snapshots, working directory, status, exit code, cancellation, and shared-transport leases.
- [x] 5.4 Add bounded paginated run-history queries and preserve run snapshots after template deletion.
- [x] 5.5 Implement interface-compatible Web template, insertion, execution, cancellation, and history simulation.
- [x] 5.6 Add Rust and TypeScript tests for validation, scopes, history pagination, cancellation, template deletion, and concurrent PTY/exec channels.

## 6. Output capture and full-text search

- [x] 6.1 Implement UTF-8-safe terminal-control normalization and ordered output chunk models for PTY and quick-command sources.
- [x] 6.2 Implement a bounded non-blocking capture queue and batched SQLite writer that emits one gap marker after dropped capture content.
- [x] 6.3 Implement FTS5 indexing and bounded search with session, connection, Terminal, run, and time filters, snippets, relevance, and stable pagination.
- [x] 6.4 Implement capture enablement, age and capacity maintenance, per-session purge, transactional FTS deletion, and startup/shutdown handling.
- [x] 6.5 Implement deterministic Web capture, search, retention, capacity, gap, and purge simulation.
- [x] 6.6 Add performance and correctness tests for burst output, slow persistence, UTF-8 splits, ANSI/control sequences, paths, CJK text, pagination, retention, capacity, and deletion.

## 7. Frontend service and UI

- [x] 7.1 Add strict TypeScript remote Terminal, trust, binding, template, run, capture, and search contracts to the Agent service boundary.
- [x] 7.2 Implement matching Tauri and Web adapters and extend contract-conformance tests without direct `invoke()` usage in components.
- [x] 7.3 Refactor the Shell tab into sub-300-line components for Terminal status, trust/rebind prompts, command templates, run history, capture status, and output search.
- [x] 7.4 Add template insert/run/cancel controls, paginated history, search filters/snippets, capture toggle, and confirmed purge behavior.
- [x] 7.5 Add synchronized Chinese and English locale resources, keyboard access, accessible names, loading/empty/error states, and simulated-Web labels.
- [x] 7.6 Add focused component tests and Playwright coverage for profile binding, host trust, pooled multi-channel use, command execution, search, capture gaps, and purge.

## 8. Diagnostics, verification, and documentation

- [x] 8.1 Route all remote Terminal lifecycle, pool, trust, command-run, capture, search, and maintenance diagnostics through unified redacted logging with rate limits.
- [x] 8.2 Add redaction tests proving passwords, credential refs, key paths, template commands, stdout, stderr, PTY output, and interactive input do not enter diagnostic sinks.
- [x] 8.3 Update architecture documentation for the SSH runtime, connection pool, Terminal content store, FTS schema, cleanup jobs, and Web behavior.
- [x] 8.4 Run `npm run lint`, `npm run test`, and `npm run build` and fix all failures.
- [x] 8.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` and fix all failures.
- [x] 8.6 Run `openspec validate add-remote-terminal-management --strict` and `openspec validate --specs --strict`, recording final verification results.
