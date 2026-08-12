## 1. Assessment Domain and Witnesses

- [ ] 1.1 Add the `skill_evolution_assessment` Rust module and versioned enums for attempt status, selection classification, check result, route, confidence, risk, and fallback reason.
- [ ] 1.2 Define sanitized assessment input, effective-target witness, ranked-target, lesson-shape, quality-check, evaluator-result, and assessment-output models.
- [ ] 1.3 Implement canonical hashing for seed revision, lineage, target universe, policies, consent, and evaluator configuration.
- [ ] 1.4 Add serialization and version-compatibility tests for every persisted and service-facing assessment model.

## 2. Target Catalog and Local Index

- [ ] 2.1 Project the effective Skill catalog with stable id, scope, type, revision, lifecycle, trust, capabilities, declared tools, and historical participation metadata.
- [ ] 2.2 Record shadowed, missing, malformed, and historical-only Skill revisions as explicit target exclusions.
- [ ] 2.3 Implement local Unicode-normalized lexical indexing for bounded descriptions, tags, capabilities, headings, and low-weight instruction tokens.
- [ ] 2.4 Treat all Skill content as untrusted data and add prompt-injection fixtures proving indexing never executes instructions or expands resource references.
- [ ] 2.5 Add catalog tests for four-scope precedence, revision drift, pinned and archived Skills, duplicate ids, and multi-Skill participation.

## 3. Deterministic Target Selection

- [ ] 3.1 Implement version-1 fixed-integer score components for attribution, repeated participation, capability/type compatibility, lexical relevance, and scope locality.
- [ ] 3.2 Implement stable candidate ordering and scope, Skill-id, and revision-hash tie breakers.
- [ ] 3.3 Implement selected, ambiguous, and no-target classifications using the versioned 60/45 score and 15-point margin thresholds.
- [ ] 3.4 Persist score components, matched feature classes, exclusions, alternatives, threshold witnesses, and attribution uncertainty.
- [ ] 3.5 Add reproducibility and boundary tests for clear, ambiguous, tied, irrelevant, historical-revision, correlated CLI, weak, and unattributed cases.

## 4. Structured Lesson Shape and Duplicate Index

- [ ] 4.1 Derive bounded trigger, behavior, prohibition, verification, environment, and content-kind fields from sanitized structured evidence without generating Skill text.
- [ ] 4.2 Build normalized guidance units from effective Skill content, trusted active Overlays, and current pending assessments.
- [ ] 4.3 Implement exact structural duplicate matching and conservative near-duplicate classification that cannot rely on lexical overlap alone.
- [ ] 4.4 Exclude untrusted Overlay content from canonical duplicates while retaining safe conflict or risk evidence.
- [ ] 4.5 Add duplicate tests for equivalent wording, shared terms with different behavior, scoped variants, pending candidates, and untrusted guidance.

## 5. Nine Deterministic Quality Checks

- [ ] 5.1 Implement the fixed registry and exactly-nine result contract with stable order, reason codes, evidence references, severity, and route constraints.
- [ ] 5.2 Implement privacy-residue detection and hard-stop behavior that prevents any model request.
- [ ] 5.3 Implement evidence-sufficiency rules for verified corrected feedback and independent nonduplicate runs.
- [ ] 5.4 Implement duplicate-knowledge and transient-incident checks with canonical references and durable-versus-local classification.
- [ ] 5.5 Implement guidance-specificity and evidence-consistency checks using structured lesson fields and scoped contradiction handling.
- [ ] 5.6 Implement target-compatibility checks that preserve attribution uncertainty and never treat participation as causality.
- [ ] 5.7 Implement executable-content-risk detection for scripts, commands, tool schemas, executable files, permissions, and expanded side effects.
- [ ] 5.8 Implement lifecycle-mutability checks for pinned, archived, missing, malformed, and changed target revisions.
- [ ] 5.9 Add a quality corpus covering pass, fail, review, not-applicable, multiple constraints, hard stops, and exactly-nine audit completeness.

## 6. Routing, Confidence, and Risk

- [ ] 6.1 Implement the versioned routing lattice for `advance`, `drop`, `record_memory_only`, `merge_duplicate`, and `needs_human_review`.
- [ ] 6.2 Implement deterministic system-confidence components, penalties, the 0.85 advance threshold, and the bounded model corroboration rule.
- [ ] 6.3 Implement low, medium, and high risk reduction that preserves the strictest deterministic or valid model result.
- [ ] 6.4 Record every route constraint, the winning policy rule, and why stricter alternatives did or did not apply.
- [ ] 6.5 Add table-driven tests for conflicting route conditions, executable risk, duplicates, transient evidence, pinned targets, and low-confidence all-pass results.

## 7. Optional Structured Model Evaluation

- [ ] 7.1 Add a provider-neutral `StructuredEvaluator` backed by compatible configured API profiles without launching or delegating to source CLI Agents.
- [ ] 7.2 Add default-disabled, versioned model-evaluation consent storage and sanitized outbound data disclosure.
- [ ] 7.3 Implement the ambiguous-target consultation schema with a maximum of five deterministic candidates, supplied-id enforcement, evidence citations, and confidence validation.
- [ ] 7.4 Implement the quality-judge schema for support, specificity, durability, actionability, contradiction, risk, citations, and safe routing advice.
- [ ] 7.5 Enforce no tools, two calls maximum, one attempt per stage, stage deadlines, token limits, strict JSON validation, and bounded sanitized rationales.
- [ ] 7.6 Ensure model results can only make outcomes stricter and cannot add targets, override hard gates, lower risk, author guidance, or trigger mutation.
- [ ] 7.7 Add fallback handling for disabled consent, missing provider, timeout, rate limit, invalid schema, invented target, missing citation, and provider failure.
- [ ] 7.8 Add adversarial tests for injected Skill content, model prompt leakage, unknown fields, oversized output, extreme confidence, and hard-gate override attempts.

## 8. Persistence, Idempotency, and Supersession

- [ ] 8.1 Add SQLite migrations for attempts, targets, score components, checks, evidence links, model calls, supersessions, policy, and queue state.
- [ ] 8.2 Implement transactional repositories that store normalized sanitized explanations and never store raw assembled model prompts or provider payloads.
- [ ] 8.3 Implement idempotency by complete assessment witness and coalesce concurrent identical requests.
- [ ] 8.4 Implement worker leases, heartbeat, expired-attempt recovery, and immutable completed attempts.
- [ ] 8.5 Implement witness recheck before commit and supersede stale attempts when seed, Skill revision, lifecycle, consent, or policy changes.
- [ ] 8.6 Integrate assessment retention and cascade purge with evidence deletion without resurrecting purged lineage.
- [ ] 8.7 Add migration, concurrency, crash recovery, stale witness, retention, purge, and database-failure tests.

## 9. Asynchronous Assessment Runtime

- [ ] 9.1 Add a bounded assessment queue separate from evidence ingestion with deterministic work prioritized over optional model stages.
- [ ] 9.2 Enqueue ready seed identifiers and witness hashes without blocking evidence workers or any Agent runtime.
- [ ] 9.3 Implement deterministic fallback under queue pressure before dropping ready-seed assessment work.
- [ ] 9.4 Add sanitized unified-log diagnostics and health counters for queue depth, stale attempts, fallback categories, model latency, and failures.
- [ ] 9.5 Add saturation, shutdown, worker panic, model stall, database lock, and disabled-feature tests proving fail-open behavior.

## 10. Service Boundary and Runtime Adapters

- [ ] 10.1 Add typed assessment summary, target, check, provenance, history, policy, consent, and reassessment contracts to `agent-service.ts`.
- [ ] 10.2 Add Rust/Tauri commands for scoped assessment queries, policy status, consent updates, and reassessment scheduling with `Result<T, String>` boundary errors.
- [ ] 10.3 Implement Tauri client methods so React components never call `invoke()` directly.
- [ ] 10.4 Implement behaviorally equivalent Web/mock methods and fixtures for deterministic, model-assisted, fallback, ambiguous, pending, failed, and superseded states.
- [ ] 10.5 Add adapter contract tests for pagination, stable ordering, error semantics, consent transitions, idempotent reassessment, and sanitized response fields.

## 11. Skill Evolution Assessment UI

- [ ] 11.1 Add assessment summary cards for target classification, alternatives, nine checks, confidence, risk, routing, and provenance.
- [ ] 11.2 Add ranked-target detail with component scores, threshold margin, revision, scope, type, matched feature classes, and uncertainty labels.
- [ ] 11.3 Add quality-check detail with safe evidence references, stable reasons, severity, and routing effect.
- [ ] 11.4 Add immutable assessment history and supersession explanations without raw model prompts or sensitive content.
- [ ] 11.5 Add default-disabled model-evaluation consent disclosure, provider availability, enable/disable controls, and deterministic fallback explanation.
- [ ] 11.6 Add safe reassessment scheduling that retains the last valid result and cannot edit evidence, override targets, or bypass checks.
- [ ] 11.7 Ensure no approve, reject, apply, Overlay, target override, memory write, unpin, archive, or automatic-evolution controls appear.
- [ ] 11.8 Add localization, responsive layout, dark theme, keyboard, focus, screen-reader, loading, empty, error, and Web/mock parity tests.

## 12. End-to-End Assessment Scenarios

- [ ] 12.1 Add an end-to-end fixture for verified corrected feedback selecting a clear low-risk Skill and recommending `advance` without mutation.
- [ ] 12.2 Add fixtures for ambiguous targets with evaluation disabled, valid model consultation, invalid model response, and deterministic fallback.
- [ ] 12.3 Add fixtures for privacy drop, canonical duplicate, transient memory-only result, contradiction review, executable high risk, pinned target, and archived target.
- [ ] 12.4 Add reassessment fixtures for changed evidence, changed effective revision, policy upgrade, consent revocation, and duplicate requests.
- [ ] 12.5 Verify native API, delegated Utility, managed CLI, and interactive CLI evidence retain their original attribution fidelity throughout assessment.

## 13. Verification

- [ ] 13.1 Run privacy and prompt-injection corpora and inspect SQLite and unified diagnostics for prohibited raw content.
- [ ] 13.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [ ] 13.3 Run `npm run build` and `npx playwright test` for assessment, consent, fallback, history, and reassessment behavior.
- [ ] 13.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 13.5 Run `openspec validate add-skill-evolution-target-selection-and-quality-gates --strict`, `openspec validate --specs --strict`, and repository documentation checks.
- [ ] 13.6 Verify model-disabled, provider-unavailable, queue-saturated, database-unavailable, and rollback scenarios leave evidence collection and every Agent family operational.
