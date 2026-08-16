## 1. Domain foundation

- [x] 1.1 Add `agent_runtime` domain value types for context requests, candidates, source provenance, safe fingerprints, budgets, evidence, manifests, source outcomes, reason codes, and versioned policies without transport or infrastructure dependencies.
- [x] 1.2 Implement deterministic scoring and stable tie-breaking for explicitness, relevance, symbol relation, path proximity, freshness, authority, duplication, and estimated cost.
- [x] 1.3 Implement exact-fingerprint collapse and canonical overlap merging with combined provenance and duplicate-savings accounting.
- [x] 1.4 Implement protected/source-class budgeting, semantic range clipping, checked occupancy accounting, emergency reserve, protected-overflow, and final invariant verification.
- [x] 1.5 Add domain tests for explicit preservation, deterministic ordering, definition/tests/callers, three-source deduplication, budget pressure, semantic boundaries, malformed candidates, overflow, and arithmetic limits.

## 2. Application orchestration and source contracts

- [x] 2.1 Add application-owned candidate-source, manifest-repository, clock/measurement, and diagnostic ports plus a Context Engine service that plans, collects with cancellation and bounds, normalizes, ranks, budgets, verifies, and projects.
- [x] 2.2 Add source-isolation tests proving unavailable, warming, timed-out, failed, and cancelled optional sources degrade independently and LSP fallback retains retrieval/Tree-sitter results.
- [x] 2.3 Add projection tests proving sources cannot append arbitrary prompt text and provider projection contains only compact labels, admitted ranges, and selected content.
- [x] 2.4 Integrate evidence occupancy with the existing context snapshot and preserve all compaction/optimizer compatibility behavior in application tests.

## 3. Existing bounded-context adapters

- [x] 3.1 Extend published retrieval contracts to return bounded workspace-code and memory candidates with provenance, estimates, safe fingerprints, staleness, and truncation metadata without changing index ownership.
- [x] 3.2 Extend published code-intelligence contracts for bounded definition/reference/call-related candidates while preserving workspace trust, capabilities, deadlines, cancellation, and normalized degradation states.
- [x] 3.3 Adapt confined explicit file references, relevant tests, recent Git changes, memory recall, and authoritative plan/task state through existing public APIs or application-owned ports; do not import another context's private repository or infrastructure.
- [x] 3.4 Assemble concrete source adapters only in bootstrap and add architecture tests for dependency direction and Web/Tauri service parity.

## 4. Native persistence, logging, and generation

- [x] 4.1 Add an idempotent centralized SQLite migration and repository for bounded content-free manifests with session/turn/generation lookup, retention, count caps, and migration compatibility tests.
- [x] 4.2 Add unified diagnostic events for collection, selection, degradation, verification, persistence failure, and timing using only allowlisted redacted metadata.
- [x] 4.3 Add adversarial negative tests proving source code, prompt/message text, memory bodies, tool payloads, credentials, headers, environment values, escaping paths, and raw provider frames never enter logs or persisted manifests.
- [x] 4.4 Invoke the Context Engine in OnePiece generation before final request construction, preserve cancellation/accounting/tool/compaction behavior, and fall back to the unchanged safe request on engine failure.
- [x] 4.5 Add native integration tests for successful projection, protected overflow, source partial failure, persistence failure, full provider-budget verification, and existing compaction/optimization regressions.

## 5. Native commands and shared frontend contract

- [x] 5.1 Add command-safe paginated manifest list/detail DTOs and Tauri commands through the `agent_runtime` API facade; register commands without domain policy in handlers.
- [x] 5.2 Add shared TypeScript manifest types and `AgentService` methods, implement Tauri adapter invokes, and implement deterministic in-memory Web/mock parity with explicit mock provenance.
- [x] 5.3 Add Rust DTO/command tests and TypeScript contract tests for serialization, pagination, empty history, unknown turn, bounded rejected summaries, source degradation, and adapter parity.

## 6. Context Inspector UI

- [x] 6.1 Add an advanced Context Inspector entry to the existing Session/OnePiece evidence surface without crowding ordinary chat or directly invoking Tauri.
- [x] 6.2 Implement localized compact budget, selected evidence, source/range/estimate/reason, top rejected, source outcome, duplicate savings, latency, and compaction-correlation views using shared semantic tokens and accessible controls.
- [x] 6.3 Add Vitest component tests for loading, empty, success, degraded source, rejected summaries, errors, keyboard/focus behavior, and all registered locale resource parity.
- [x] 6.4 Add Playwright behavior and stable screenshots for futuristic and minimal styles at desktop and narrow widths, checking overlap, clipping, contrast, focus, and blank-panel regressions.

## 7. Benchmark and performance evidence

- [x] 7.1 Add a deterministic synthetic benchmark dataset for definition, cross-file references, tests, explicit refs, duplicates, LSP fallback, budget pressure, and memory relevance.
- [x] 7.2 Calculate and assert Recall@budget, Precision@budget, useful-token ratio, duplicate savings, overflow rate, stable selections, and operation-count budgets.
- [x] 7.3 Record repeatable native candidate-collection and ranking latency measurements separately from deterministic CI budgets and document the observed local platform evidence.

## 8. Ordered verification and acceptance

- [x] 8.1 Run focused Rust domain/application/infrastructure, migration, command, security-negative, compatibility, and benchmark tests; fix all failures before broader gates.
- [x] 8.2 Run focused Vitest, contract, Playwright behavior, and four-way visual tests; verify futuristic/minimal and desktop/narrow acceptance before broader gates.
- [x] 8.3 Run `npm run lint:ci`, `npm run test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml` in that order.
- [x] 8.4 Run `npm run test:coverage`, `npm run contracts:check`, `npx playwright test`, `npm run desktop:unit:test`, and `npm run test:desktop`; record Linux actual status and mark Windows/macOS `NOT RUN` unless native evidence is actually obtained.
- [x] 8.5 Run `openspec validate --specs --strict` and `openspec validate unify-agent-context-engine --strict`, then verify every requirement/scenario and task against implementation evidence with no critical or warning gaps.
- [x] 8.6 Archive only after every prior task passes using `openspec archive unify-agent-context-engine`, run `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`, and rerun strict main-spec and archived-change validation.
