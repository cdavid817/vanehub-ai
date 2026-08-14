## 1. Settings and assessment domain

- [x] 1.1 Add the 7/30/90-day context-quality retention setting to frontend settings types, defaults, normalization, validation, reset behavior, and unit tests.
- [x] 1.2 Add the retention setting to native desktop settings domain, DTO mapping, persistence round trips, and tests without introducing a parallel settings store.
- [x] 1.3 Define allowlisted assessment outcome, path, reason, measurement-quality, invariant, correlation, and policy-version domain types with bounded serialization.
- [x] 1.4 Add domain tests for compacted, bypassed, fallback, and failed assessments, saturated savings, stable attempt correlations, and prohibited-content absence.

## 2. Native persistence and aggregation

- [x] 2.1 Add an additive SQLite migration for context-quality assessments with bounded columns and timestamp, outcome, policy-version, and correlation indexes.
- [x] 2.2 Add repository port operations for best-effort append, cursor history, aggregate summary, and oldest-first pruning by retention and the 10,000-row hard ceiling.
- [x] 2.3 Implement SQLite repository operations with transaction, migration-upgrade, rollback-compatibility, empty-range, mixed-quality, pagination, and pruning tests.
- [x] 2.4 Emit redacted unified warnings on assessment persistence/pruning failures without changing compaction or generation outcomes.

## 3. Runtime assessment and regression evaluation

- [x] 3.1 Generate exactly one assessment at the native automatic-compaction coordinator after final optimizer, compatibility, bypass, or failure resolution.
- [x] 3.2 Reuse one stable attempt correlation in successful evidence cards and assessment records while keeping assessment persistence best effort.
- [x] 3.3 Add integration tests for all final outcomes, evidence correlation, persistence failure isolation, measurement provenance, and sensitive-content exclusion.
- [x] 3.4 Build a versioned content-safe structural regression corpus covering protocol rounds, retention classes, reinjection, large tool results, multilingual sizes, unavailable tokens, and arithmetic boundaries.
- [x] 3.5 Implement deterministic active-versus-candidate policy evaluation by reusing production planner, reducer, reinjection, and verifier domain boundaries without provider calls.
- [x] 3.6 Add regression tests proving repeatability, invariant failures overriding savings, baseline-success regression detection, bounded aggregate comparisons, and non-authoritative results.

## 4. Service contracts and runtime adapters

- [x] 4.1 Add typed frontend requests, cursor pages, assessment records, summaries, quality coverage, distributions, and safe errors to the agent service boundary.
- [x] 4.2 Add Rust application queries, Tauri commands, DTO mappers, command registration, and contract tests for context-quality history and summaries.
- [x] 4.3 Implement Tauri frontend adapter mappings without exposing `invoke()` to React components.
- [x] 4.4 Implement deterministic capped Web/mock assessment history, pruning, history queries, summaries, and typed failure behavior with parity tests.
- [x] 4.5 Run `npm run contracts:check` and add/update explicit frontend/native contract fixtures when the shared command surface changes.

## 5. Context health UI and documentation

- [x] 5.1 Add a localized OnePiece context-policy health section with range selection, aggregate cards, quality/path/outcome distributions, recent history, and operational-quality disclosures.
- [x] 5.2 Add accessible loading, empty, pagination, retention-saving, and independent error states using semantic Tailwind tokens and the service/settings boundaries.
- [x] 5.3 Add all supported locale strings plus component tests for data, empty, mixed-quality, error, range, pagination, retention, and narrow-layout states.
- [x] 5.4 Add Playwright coverage for desktop-contract mock behavior, Web/mock behavior, keyboard controls, persistence reload, English minimal theme, and narrow viewport.
- [x] 5.5 Update `docs/zh/src/02-architecture/native-agent.md` to describe delivered evidence/settings behavior and the new quality-evaluation boundary without claiming provider-native cache edits are active.

## 6. Verification and completion

- [x] 6.1 Run `openspec validate add-onepiece-context-quality-and-policy-evaluation --strict` and `openspec validate --specs --strict`.
- [x] 6.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, `npm run docs:check`, and `npm run build`.
- [x] 6.3 Run `npx playwright test` for the context-health UI behavior change.
- [x] 6.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.5 Run OpenSpec implementation verification, resolve all findings, and record final validation evidence before archive.
