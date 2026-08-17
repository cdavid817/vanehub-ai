## Why

VaneHub already protects several isolated hot paths, but it lacks one repeatable, versioned way to measure runtime performance across Agent runs, context selection, code intelligence, terminal search, and large persisted histories. Without shared measurement metadata and budget comparison, regressions are hard to reproduce and latency-only gates would be flaky on shared CI runners.

## What Changes

- Add a repository-owned performance harness with versioned deterministic datasets, normalized result records, baseline comparison, and actionable over-budget diagnostics.
- Classify metrics as deterministic CI gates, dedicated benchmark evidence, or informational device telemetry; fixed wall-clock latency is not introduced as a shared-runner hard gate.
- Extend Context Engine evidence with phase latency, candidate/selection counts, and byte/Token occupancy while preserving content-free diagnostics.
- Add structural and benchmark evidence for Run lifecycle transitions, cancellation, event coalescing, concurrent resource growth, LSP/Tree-sitter queries, terminal search, and large persisted Run/session histories.
- Preserve the existing bounded contexts, service contracts, Tauri/Web adapters, unified logging path, and optional developer-only UI boundary. No end-user performance UI is added in this change.
- Record the existing `harden-runtime-lifecycle-and-boundaries` work as a reused prerequisite for batched relationship reads, short lock scopes, and terminal frame batching rather than duplicating it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `runtime-performance-governance`: Define the versioned harness, dataset/result contract, metric classes, baseline-derived budgets, regression reporting, and platform evidence policy.
- `agent-context-engine`: Require phase and occupancy evidence for candidate collection, ranking, budgeting, projection, and index-backed sources.
- `agent-context-measurement`: Require bounded, content-free performance measurements alongside existing occupancy provenance.
- `agent-run-state-management`: Require deterministic lifecycle/cancellation/concurrency budgets and dedicated latency evidence.
- `agent-execution-observability`: Require performance evidence correlation without making telemetry failures block the owning Run.
- `agent-mission-control`: Require 100/1,000-Run structural query and frontend list/update budgets without N+1 behavior.
- `remote-terminal-runtime`: Require long-terminal search and buffer-bound evidence.
- `lsp-code-intelligence`: Require versioned Tree-sitter, LSP, indexing, and search performance evidence with P50/P95 only in dedicated results.

## Impact

- Repository tooling: new Node-based performance manifest/parser/comparator and deterministic fixtures, exposed through npm scripts.
- Native runtime tests: existing `agent_runtime`, `operations`, `sessions`, `workspaces`, `code_intelligence`, `retrieval`, and SSH/terminal infrastructure are measured through their current public or test boundaries; no new bounded context is created.
- Frontend tests: existing chat and Mission Control coalescers plus long-list rendering receive deterministic structural coverage; React continues to use `agent-service.ts` and existing Web/Tauri adapters.
- CI and evidence: deterministic gates can run on shared CI, while latency/throughput/memory records include commit, platform, build profile, and dataset version for dedicated comparison.
- Compatibility and migration: no public command, service DTO, database schema, persisted user data, dependency, or Web/mock behavior change is planned.
