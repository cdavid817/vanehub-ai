# Make workspace inspection cancellation generation-safe and resource-bounded

## Why

Workspace file, path, document, and content inspection already exposes result caps and cancellation flags, but several current mechanisms do not bound the work that occurs before a result is returned.

The cancellation registry is keyed only by `search_id`. Starting request B with the same id replaces and cancels request A, but A's unconditional `finish(search_id)` can later remove B's new registration. B continues running while a subsequent cancel can no longer find it. If the async caller is aborted or dropped before the explicit finish call, the registration can also remain stale and its blocking worker is not reliably signalled.

Result limits do not equal resource budgets. A traversal may visit, canonicalize, stat, or reject a very large number of entries while producing few matches. Content search can collect an entire candidate-file vector before opening files. Directory pagination and path search can materialize and sort a complete directory/candidate set for every page. Recursive document discovery can enter dependency/generated trees that are not useful to the user. Multiple windows and fast repeated searches can queue many independent blocking tasks before any workspace-level backpressure applies.

On a large monorepo, generated workspace, network mount, or adversarial directory tree, these behaviors can cause high memory consumption, blocking-pool saturation, delayed cancellation, stale UI results, and incomplete scans that look indistinguishable from a true “no matches” result.

## What Changes

- Replace id-only cancellation registration with a generation-qualified RAII registration. A superseded request is cancelled immediately; completion or drop removes only its own generation and can never remove a newer registration.
- Propagate one cancellation token from the async service boundary into blocking/local/remote work. Dropping or aborting the owning future signals cancellation even when normal completion code is not reached.
- Introduce a shared `WorkspaceInspectionBudget` that accounts for actual work: directories visited, entries visited, files opened, bytes read, canonicalizations/metadata operations, retained candidates, results, depth, and an injected monotonic deadline.
- Standardize complete/partial/unavailable coverage and stable reason codes for cancellation, supersession, each exhausted budget, unreadable entries, admission pressure, provider failure, invalid/stale cursor, and deadline.
- Stream content traversal and file reading instead of collecting all candidate files first. Check cancellation and budget at directory, entry, open, chunk-read, match, and serialization boundaries.
- Replace full retained candidate vectors with bounded selection structures. Path search and immediate directory pagination may still scan eligible entries without a persistent index, but retained memory SHALL be proportional to configured page/result limits rather than total directory size.
- Version and fingerprint directory cursors. A cursor that cannot be safely applied to the requested directory/order is rejected with a typed stale/invalid result so the UI can restart pagination instead of mixing pages.
- Introduce one shared recursive-discovery ignore policy using repository ignore files plus bounded default dependency/generated exclusions. Explicit file reads and direct user navigation remain available; the ignore policy governs recursive discovery/search, not authorization.
- Add global and per-workspace inspection admission/backpressure before `spawn_blocking` or remote process launch. Repeated search generations supersede older work instead of accumulating an unbounded queue.
- Keep Tauri, remote, and Web/mock service contracts aligned. Web/mock simulates deterministic budget/cancellation semantics without claiming to inspect the native filesystem.
- Add deterministic structural performance tests based on operation counters and retained-set size rather than unreliable absolute CI latency.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `session-project-inspection`: Define generation-safe cancellation, actual-work budgets, standardized coverage, bounded-memory path/content/directory/document inspection, recursive ignore policy, stable cursors, adapter parity, and frontend presentation.
- `runtime-performance-governance`: Define workspace/global inspection admission, bounded blocking/remote work, structural metrics, cancellation latency checkpoints, and deterministic performance gates.

## Impact

- Affects `workspaces` application search/cancellation services, local file/path/content/document query adapters, immediate directory listing, remote inspection adapters/process control, runtime/API `spawn_blocking` boundaries, bootstrap composition, frontend workspace service types, Tauri/Web adapters, UI result/coverage presentation, i18n, and performance tests.
- Existing public maximum result/file-size values remain upper compatibility bounds unless current main already defines lower product limits. This change adds independent work/memory/deadline limits; it does not raise existing caps to preserve old implementation behavior.
- Directory/list/search DTOs may gain optional coverage, reason-code, budget-summary, cursor-version, and request-generation fields. Adapters SHALL preserve backward-compatible decoding where persisted UI state or fixtures require it.
- No SQLite migration is required for the baseline implementation. A future indexed-search change can add persistence separately.
- The shared ignore policy may use the repository's existing ignore dependency or add a focused dependency after license/supply-chain review. It SHALL not become a second authorization system.

## Non-Goals

- Solving path check-to-open TOCTOU with root directory handles, `openat2`, or Windows handle confinement. That belongs to `harden-workspace-file-handle-confinement`.
- Building a persistent file index, watcher-backed search database, trigram index, or semantic code-search engine.
- Redesigning Git queries, terminal FTS search, LSP, Tree-sitter, regex syntax, or Unicode case-fold offset mapping.
- Hiding explicit directories such as `node_modules` when a user directly navigates to or reads them. Defaults apply to recursive discovery/search and remain configurable through existing product policy.
- Claiming exact filesystem snapshot consistency while a directory is concurrently mutating. This change detects cursor incompatibility where possible and reports incomplete coverage rather than fabricating a snapshot.
- Using higher thread counts, larger result caps, or longer timeouts as a substitute for bounded algorithms.
