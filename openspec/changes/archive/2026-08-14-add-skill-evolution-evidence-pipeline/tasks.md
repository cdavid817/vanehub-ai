## 1. Evidence Domain and Source Contracts

- [x] 1.1 Add the `skill_evolution_evidence` Rust module boundary and versioned enums for source families, signal categories, attribution strength, feedback state, and seed readiness.
- [x] 1.2 Define the closed structured source-envelope variants for native execution, Skill loading, delegated Utility outcomes, Plan verification, managed CLI, interactive CLI, and explicit feedback.
- [x] 1.3 Add bounded-field validation that rejects unregistered free-text fields, oversized correction notes, invalid timestamps, and malformed source identifiers.
- [x] 1.4 Add serialization compatibility tests for every envelope and domain enum version.

## 2. Privacy Sanitization

- [x] 2.1 Implement the sanitizer registry for the twelve required sensitive-data classes and deterministic non-reversible replacement markers.
- [x] 2.2 Add installation-scoped HMAC marker derivation without persisting or logging original matched values.
- [x] 2.3 Sanitize registered bounded text before fingerprinting, persistence, diagnostics, or query projection.
- [x] 2.4 Add a privacy corpus covering private keys, tokens, authorization data, credentials, credential URLs and connection strings, environment secrets, local paths, contact data, network identifiers, and cloud account identifiers.
- [x] 2.5 Add regression tests for source-code-like text, repeated secrets, overlapping matches, Unicode input, and already-redacted markers.

## 3. Attribution and Eligibility

- [x] 3.1 Implement verified attribution from exact native effective Skill revision observations and preserve all participating revisions.
- [x] 3.2 Implement correlated attribution from CLI launch mount snapshots and execution correlation identifiers.
- [x] 3.3 Implement weak and unattributed classifications for binding-only or incomplete CLI evidence without inventing Skill participation.
- [x] 3.4 Implement targeting-eligibility rules that exclude weak and unattributed evidence from targeted seed hints while retaining inspectable lineage.
- [x] 3.5 Add attribution tests for multi-Skill participation, revision changes during a run, missing observations, stale CLI bindings, and mixed-fidelity evidence.

## 4. Deterministic Signal Extractors

- [x] 4.1 Implement the explicit-feedback extractor for helpful, unhelpful, and corrected message outcomes.
- [x] 4.2 Implement the execution-and-tool-failure extractor using structured terminal states, error classes, and safe counters.
- [x] 4.3 Implement the verification extractor for test, build, lint, review, and Plan verification outcomes.
- [x] 4.4 Implement the retry-and-recovery extractor that links failed attempts to subsequent verified recovery without copying raw output.
- [x] 4.5 Implement the delegated-Utility-outcome extractor for invocation, completion, failure, cancellation, and verification facts.
- [x] 4.6 Implement the usage-and-lifecycle-anomaly extractor for structured count and state anomalies.
- [x] 4.7 Add pure-function fixtures proving that only the six registered extractor families emit signals and that unknown envelopes fail closed.
- [x] 4.8 Add extractor tests for positive, negative, neutral, duplicate, incomplete, and out-of-order source events.

## 5. SQLite Persistence and Idempotency

- [x] 5.1 Add migrations for receipts, signals, Skill associations, source links, candidate seeds, seed-signal lineage, feedback current state and events, and pipeline state.
- [x] 5.2 Implement repository transactions that persist sanitized signals, associations, lineage, and receipt state atomically.
- [x] 5.3 Implement idempotent ingestion keys and replay handling for duplicate producer delivery and worker restart.
- [x] 5.4 Add indexes for workspace, Skill revision, category, attribution, readiness, created time, and source lookup queries.
- [x] 5.5 Add migration, rollback-compatibility, transaction-failure, corruption-boundary, and concurrent-ingestion tests.

## 6. Fingerprints and Candidate Seeds

- [x] 6.1 Implement post-sanitization deterministic task fingerprints with explicit versioning.
- [x] 6.2 Implement seed grouping by workspace, category, fingerprint, compatible Skill cohort, evidence strength, and fourteen-day window.
- [x] 6.3 Implement readiness rules for verified corrected feedback and independent nonduplicate-run evidence.
- [x] 6.4 Implement positive recovery attachment to compatible negative evidence without treating recovery as a separate improvement request.
- [x] 6.5 Persist complete seed-to-signal lineage, counts, first and last observation times, attribution summary, and readiness reason.
- [x] 6.6 Implement dirty-group rebuild after feedback replacement or source supersession.
- [x] 6.7 Add reproducibility tests proving identical sanitized inputs produce identical fingerprints, groups, readiness, and lineage ordering.

## 7. Bounded Fail-Open Ingestion

- [x] 7.1 Implement the bounded priority queue with the documented capacity, reserved high-value lanes, and deterministic pressure policy.
- [x] 7.2 Implement an asynchronous worker that performs sanitization, extraction, attribution, persistence, and seed rebuilding outside execution critical paths.
- [x] 7.3 Ensure producer enqueue, worker, database, sanitizer, and quota failures cannot fail or delay Agent execution, CLI sessions, verification, or delegation.
- [x] 7.4 Record pipeline health, queue depth, sanitized failure categories, and per-priority drop counters through the unified logging and diagnostic boundaries.
- [x] 7.5 Add saturation, crash recovery, shutdown drain, database lock, and malformed-envelope tests that verify fail-open runtime behavior.

## 8. Retention, Quotas, and Purge

- [x] 8.1 Implement the fixed ninety-day evidence retention sweep with deterministic deletion ordering and orphan-lineage cleanup.
- [x] 8.2 Enforce workspace limits of 10,000 signals, 2,000 seeds, and 64 MiB while preserving the documented priority order.
- [x] 8.3 Enforce per-seed and per-signal lineage limits of 100 signals, 32 Skill associations, and 16 source links with visible truncation metadata.
- [x] 8.4 Implement transactional purge by workspace, Skill, conversation, and all evidence without deleting source messages, runs, logs, Skills, usage, or Overlays.
- [x] 8.5 Add retention, quota pressure, priority preservation, partial-scope purge, full purge, and interrupted-purge tests.

## 9. Runtime Evidence Projections

- [x] 9.1 Project safe native execution and tool outcome envelopes with exact run, task, message, and observed Skill revision references.
- [x] 9.2a Project Role Skill load lifecycle envelopes without copying prompts, tool arguments, or tool output.
- [x] 9.2b Project delegated Utility lifecycle envelopes from the authoritative Utility runtime once that runtime provides canonical Utility revision and terminal facts.
- [x] 9.3 Project Plan verification summaries and retry/recovery relationships from their authoritative structured stores.
- [x] 9.4 Project managed and interactive CLI lifecycle facts with launch snapshots and honest correlated, weak, or unattributed fidelity.
- [x] 9.5 Add producer contract tests proving runtime projections contain only registered metadata and remain no-ops when evidence collection is disabled.

## 10. Explicit Message Feedback

- [x] 10.1 Extend message service models with helpful, unhelpful, and corrected feedback state plus bounded optional correction guidance.
- [x] 10.2 Add Rust commands and repository operations for create, replace, and clear feedback using compare-and-swap version checks.
- [x] 10.3 Persist feedback state and its evidence transition in one transaction, then rebuild affected seed groups deterministically.
- [x] 10.4 Add matching Tauri and Web/mock adapter methods without direct component-level `invoke()` calls.
- [x] 10.5 Add chat controls with keyboard access, localization, pending state, replacement confirmation, and actionable save-conflict or save-failure feedback.
- [x] 10.6 Add service, component, and E2E tests for feedback create, replace, clear, conflict, retry, sanitization, and pipeline-disabled behavior.

## 11. Evidence Query Service

- [x] 11.1 Add service-boundary models and paginated read-only queries for signal summaries, candidate seed summaries, lineage detail, pipeline health, retention, and quota state.
- [x] 11.2 Implement Rust query commands that enforce workspace and Skill scope and return only sanitized evidence projections.
- [x] 11.3 Implement matching Tauri and Web/mock adapter behavior, including deterministic mock evidence and empty states.
- [x] 11.4 Add authorization-scope, pagination, stable ordering, truncation disclosure, and prohibited-field contract tests.

## 12. Skill Evolution Evidence UI

- [x] 12.1 Add a per-Skill Evolution evidence area showing counts, categories, readiness, attribution fidelity, revision participation, and recent activity.
- [x] 12.2 Add signal and candidate-seed detail views with sanitized lineage, source type, independent-run counts, readiness explanation, and uncertainty labels.
- [x] 12.3 Add pipeline health, retention, quota, dropped-event, and collection-disabled states with clear evidence-only wording.
- [x] 12.4 Add scoped purge confirmation flows that enumerate what evidence is removed and what source records remain.
- [x] 12.5 Ensure the UI exposes no approve, target-selection, Overlay, apply, or automatic-evolution actions in this change.
- [x] 12.6 Add responsive, dark-theme, keyboard, focus, screen-reader, loading, empty, error, and Web/mock parity tests.

## 13. Integration and Verification

- [x] 13.1 Run the privacy corpus and inspect SQLite fixtures and unified diagnostic output to confirm prohibited raw content is absent.
- [x] 13.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 13.3 Run `npm run build` and `npx playwright test` for the chat feedback and Skill evidence UI behavior.
- [x] 13.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 13.5 Run `openspec validate add-skill-evolution-evidence-pipeline --strict`, `openspec validate --specs --strict`, and the repository documentation checks.
- [x] 13.6 Verify collection-disabled, queue-saturated, database-unavailable, and rollback scenarios leave every Agent family operational and preserve source data.
