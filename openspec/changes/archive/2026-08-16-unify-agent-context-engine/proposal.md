## Why

VaneHub already has independent file references, workspace retrieval, Tree-sitter indexing, LSP intelligence, cross-session memory, context measurement, compaction, optimization, and quality evaluation, but each source currently reaches OnePiece through separate paths. A unified engine is needed to select the most valuable evidence for each new turn under a provider budget without duplicating those capabilities or conflating proactive evidence selection with compaction of existing conversation history.

## What Changes

- Add a provider-neutral Context Engine in the existing `agent_runtime` bounded context that plans candidate collection, normalizes source results, ranks deterministically, merges duplicate ranges, applies a versioned budget policy, and projects a bounded evidence set into OnePiece requests.
- Reuse published contracts from `retrieval`, `code_intelligence`, `workspaces`, session memory, plan/task state, and explicit file references; a source failure degrades only that source and never fails the owning generation.
- Protect explicit references and authoritative task state, preserve semantic code boundaries where available, and expose stable selection and rejection reason codes.
- Add a content-free evidence manifest and Context Inspector for advanced Session/OnePiece diagnostics through the shared frontend service contract, Tauri adapter, and deterministic Web/mock adapter.
- Add deterministic benchmark fixtures and measurements for recall, precision, useful-token ratio, duplicate savings, overflow rate, collection latency, and ranking latency.
- Keep existing context compaction and optimization contracts unchanged: they continue reducing an already assembled conversation when it is too large, while the Context Engine governs evidence proactively injected for a new turn.
- Persist and log only allowlisted manifest metadata, safe fingerprints, buckets, counts, reason codes, estimates, policy versions, correlations, and latency; source content, prompts, and memory bodies remain excluded.

## Capabilities

### New Capabilities

- `agent-context-engine`: Defines planning, multi-source candidate normalization, deterministic ranking, deduplication, semantic budgeting, evidence manifests, inspection, privacy, degradation, and benchmark behavior.

### Modified Capabilities

- `agent-context-evidence-projection`: Extends evidence projection from compaction-only cards to selection manifests inspectable through desktop and Web/mock runtimes.
- `retrieval-vector-search`: Makes bounded workspace-code and memory retrieval available as non-authoritative Context Engine candidate sources with explicit provenance and degradation.
- `lsp-code-intelligence`: Makes definition, references, and call-related results available as bounded Context Engine candidates while preserving trust and failure semantics.
- `agent-cross-session-memory`: Makes selected memory recall an independently budgeted Context Engine source without exposing memory content in diagnostics.
- `agent-context-measurement`: Accounts for injected evidence separately from existing request components so selection and later compaction share consistent occupancy provenance.
- `onepiece-native-agent`: Requires OnePiece generation assembly to invoke the Context Engine before provider request construction and to degrade safely when optional sources are unavailable.
- `unified-log-management`: Adds an allowlisted, content-free diagnostic event shape for Context Engine decisions and source timing.

## Impact

- Desktop runtime: extends `agent_runtime` domain/application/infrastructure, consumes existing bounded-context APIs through explicit ports, adds command-safe manifest queries, and may add a SQLite migration for bounded manifest metadata only.
- Frontend: extends `AgentService`, `tauri-agent-client.ts`, and `web-agent-client.ts` with one compatible inspector contract; Session/OnePiece advanced UI gains a localized compact inspector that supports futuristic/minimal themes and desktop/narrow layouts.
- Provider behavior: OnePiece request assembly receives bounded projected evidence; existing CLI agent adapters and provider protocol contracts are not rewritten.
- Compatibility: existing compaction, optimizer, retrieval, LSP tool, memory, service adapter, and message contracts remain compatible. No new bounded context, state library, UI library, or provider dependency is introduced.
- Verification: adds Rust domain/application/infrastructure tests, negative privacy and path-safety tests, Vitest and contract tests, Playwright behavior and visual coverage, desktop E2E, and deterministic benchmark evidence.
