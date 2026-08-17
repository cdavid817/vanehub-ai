## 1. Harness and Versioned Datasets

- [x] 1.1 Add the bounded manifest/result schema and deterministic small, medium, large repository, 100-session, 100/1,000-Run, long-terminal, and high-rate-stream dataset definitions.
- [x] 1.2 Implement manifest/result parsing, fixture-root safety validation, metric-class enforcement, deterministic comparison, and actionable regression formatting with focused unit tests.
- [x] 1.3 Add a known-over-budget negative fixture and prove it fails without mutating the accepted baseline.
- [x] 1.4 Expose repeatable `performance:check` and dedicated `performance:benchmark` npm commands and document metric classes, evidence provenance, baseline updates, and shared-CI policy.

## 2. Context and Run Evidence

- [x] 2.1 Extend the Context Engine benchmark with versioned phase, operation, candidate, selected-item, byte, Token, duplicate-saving, overflow, and occupancy evidence while preserving existing quality assertions.
- [x] 2.2 Add content-free context measurement safety tests for allowlisted performance metadata and rejection of prompts, messages, tool payloads, credentials, raw frames, and unrestricted paths.
- [x] 2.3 Add canonical Run lifecycle structural measurements covering 1,000 histories, legal/illegal transitions, terminal idempotency, cancellation, retained events, and bounded concurrent resource growth.
- [x] 2.4 Add metadata-only execution-observability correlation and non-blocking failure coverage for performance evidence without changing canonical Run outcomes.

## 3. Persistence, Code Intelligence, and Terminal Evidence

- [x] 3.1 Extend Mission Control persistence tests for deterministic 100/1,000-Run datasets, constant query count, indexed plans, bounded pages, lazy detail, and the N+1 negative case.
- [x] 3.2 Add LSP and Tree-sitter/index/search measurements over versioned repository scales, response/item caps, unavailable semantics, and dedicated P50/P95 output.
- [x] 3.3 Add long-terminal measurements for UTF-8 chunk bounds, retained bytes, loaded rows, indexed query/page bounds, cancellation, and gap behavior without raw output in evidence.

## 4. Frontend Structural Performance

- [x] 4.1 Add high-rate token and Mission Control event tests proving bounded reducer traversals, coalesced update batches, immediate terminal flushes, and stable untouched references.
- [x] 4.2 Add 100/1,000-Run long-list coverage through the existing Web service boundary and verify bounded rendering/lazy detail without introducing a new UI or adapter contract.
- [x] 4.3 Run the affected long-list Playwright scenarios at desktop and narrow widths under futuristic and minimal themes, checking clipping, overflow, blank panels, and accessible state presentation.

## 5. Security, Compatibility, and Benchmark Evidence

- [x] 5.1 Add parser, dataset determinism, budget comparison, malformed metadata, traversal, oversized fixture, duplicate id, unknown class/unit, and sensitive-field negative tests.
- [x] 5.2 Run the deterministic performance command and dedicated Windows benchmark, recording commit, platform, architecture, build profile, dataset version, metric, baseline, budget, delta, and actual platform status.
- [x] 5.3 Confirm no database migration, public Tauri command, frontend service contract, Web/Tauri adapter, bounded-context ownership, unified-log path, or user-visible UI behavior changed.

## 6. Verification and Delivery

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run test:coverage`, `npm run contracts:check`, and `npx playwright test`.
- [x] 6.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.3 Run `npm run desktop:unit:test` and `npm run test:desktop`, reporting Windows/macOS/Linux independently as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` without extrapolation.
- [x] 6.4 Run `openspec validate --specs --strict` and `openspec validate expand-runtime-performance-budgets --strict`, then record final performance, security, compatibility, and verification evidence for archive review.
