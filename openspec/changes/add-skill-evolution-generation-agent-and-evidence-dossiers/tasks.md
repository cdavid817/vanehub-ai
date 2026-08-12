## 1. Generation Domain and Storage

- [ ] 1.1 Add the `skill_evolution_generation` Rust module and versioned enums for consent, job/stage status, dossier section/status, artifact kind, model/tool outcome, validation, quarantine, and handoff.
- [ ] 1.2 Define frozen input, thirteen-section dossier, job, stage attempt, structured plan/draft, rendered artifact, validation, export, quarantine, and review-package models.
- [ ] 1.3 Add SQLite migrations for policy, jobs, sources, dossiers, sections, links, stage attempts, model calls, tool receipts, structured results, drafts, validations, handoffs, quarantine, and exports.
- [ ] 1.4 Implement canonical serialization, witness hashing, optimistic versions, idempotency, immutable attempts, and safe normalized persistence.
- [ ] 1.5 Add migration, schema compatibility, corruption-boundary, and concurrent-request tests.

## 2. Thirteen-Section Evidence Dossiers

- [ ] 2.1 Implement the exact ordered thirteen-section dossier schema with status, source witnesses, truncation metadata, and per-section hashes.
- [ ] 2.2 Build identity/provenance, executive summary, candidate seed, signal inventory, attribution/target, and assessment/quality sections from authoritative sanitized records.
- [ ] 2.3 Build effective Skill snapshot, relevant guidance/resource context, outcome timeline, privacy report, mutation rationale, verification plan, and lineage/version sections.
- [ ] 2.4 Enforce stable ordering, 128 KiB canonical size, record/excerpt limits, and explicit partial/not-applicable/unavailable/redacted states.
- [ ] 2.5 Reject incompatible sanitizer or source versions before dossier persistence or model use.
- [ ] 2.6 Add byte-reproducibility, version drift, missing records, truncation, redaction, multi-Skill, no-target, and purge fixtures.

## 3. Dossier Queries and Export

- [ ] 3.1 Implement sanitized section pagination, completeness metadata, stable ordering, and source-link queries.
- [ ] 3.2 Implement deterministic canonical JSON and human-readable Markdown renderers with schema, redaction, and hash manifests.
- [ ] 3.3 Export only through the normal user-selected file boundary and track safe export metadata without feature-local logs or hidden files.
- [ ] 3.4 Add export parity, pagination, hash, path-boundary, sensitive-data exclusion, and oversized-section tests.

## 4. Generation Policy and Consent

- [ ] 4.1 Implement generation-specific default-off consent with disclosure version, compatible API profile/model, draft kinds, budgets, and policy revision.
- [ ] 4.2 Keep assessment consent and orchestration consent independent and refuse imported generation consent.
- [ ] 4.3 Stop new model/tool stages on revocation while preserving completed local dossiers and historical provenance.
- [ ] 4.4 Add conflict-safe updates, provider readiness, disclosure upgrade, revocation race, import, and disabled-mode tests.

## 5. Seven-Stage Job Runtime

- [ ] 5.1 Implement the fixed `freeze_input`, `inspect_target`, `build_dossier`, `plan_mutation`, `synthesize_structured_draft`, `validate_and_simulate`, and `package_for_governance` stages.
- [ ] 5.2 Persist immutable stage attempts with input/output hashes, counters, budgets, safe failures, timestamps, and supersession.
- [ ] 5.3 Enforce one job per workspace, two globally, 180-second wall time, three model calls, eight tool calls, token limits, daily limits, and one repair.
- [ ] 5.4 Implement cooperative cancellation, stale-witness supersession, restart reconciliation, and request idempotency.
- [ ] 5.5 Add state-machine, budget boundary, duplicate request, cancellation, crash recovery, supersession, and one-repair tests.

## 6. Structured Model Adapter

- [ ] 6.1 Add a generation purpose to the provider-neutral structured API adapter without invoking any CLI or ordinary chat session.
- [ ] 6.2 Assemble versioned control prompts with untrusted dossier and Skill data kept in delimited structured fields.
- [ ] 6.3 Enforce strict JSON response schemas, no freeform fallback, bounded rationale, and no chain-of-thought field.
- [ ] 6.4 Persist only provider/model identifiers, schema/template versions, safe result hash, token counts, latency, and outcome category.
- [ ] 6.5 Add missing provider, timeout, rate limit, malformed JSON, unknown fields, oversized output, consent loss, and provider-failure tests.

## 7. Five Constrained Read-Only Tools

- [ ] 7.1 Implement bounded dossier-section lookup with cursor and citations.
- [ ] 7.2 Implement bounded frozen effective-Skill excerpt lookup and exact-anchor search.
- [ ] 7.3 Implement local draft-structure validation and preview-simulation tools without mutation authority.
- [ ] 7.4 Register exactly the five allowed tools and reject shell, network, file read/write, generic retrieval, Skill loading, sub-Agent, and unknown tool requests.
- [ ] 7.5 Bind every tool call to job/input witnesses, sanitize arguments/results, enforce budgets, and persist safe receipts.
- [ ] 7.6 Add stale witness, injection, path escape, oversized argument/result, unknown tool, budget, and citation tests.

## 8. Structured Plans, Citations, and Local Renderers

- [ ] 8.1 Define strict `MutationPlanV1` and `StructuredDraftV1` schemas with one artifact, lesson shape, evidence citations, expected behavior, and verification plan.
- [ ] 8.2 Validate every trigger/action/verification claim against frozen dossier ids and reject invented citations, targets, scopes, or fields.
- [ ] 8.3 Implement deterministic learned-guidance and exact-patch renderers with canonical Markdown, escaping, size limits, and `replace_all=false`.
- [ ] 8.4 Implement deterministic single-file `SKILL.md` rendering with valid frontmatter, Role/Utility type, built-in dependencies, and concise body limits.
- [ ] 8.5 Reject hidden comments, raw HTML/script constructs, embedded files, executable content, external dependencies, and unsupported metadata.
- [ ] 8.6 Add renderer byte-stability, YAML/Markdown escaping, Unicode, collision-like ids, multiple mutations, and malicious structured-output tests.

## 9. Existing-Skill Draft Validation

- [ ] 9.1 Lock existing-Skill plans to the assessed target, effective revision, and server-owned Overlay scope.
- [ ] 9.2 Require one unique current anchor for exact patches and reject unrelated deletion, multiple matches, or target drift.
- [ ] 9.3 Run sanitizer, injection scanner, schema, size/token, compatibility, duplicate, nine quality gates, optional stricter judge, Overlay preview, and verification-plan checks.
- [ ] 9.4 Permit one bounded repair using only safe reason codes and rerun the entire validation pipeline.
- [ ] 9.5 Add safe learn block, contradictory guidance patch, pinned target, stale Overlay, duplicate, hard gate, failed repair, and preview fixtures.

## 10. Quarantined New Skill Proposals

- [ ] 10.1 Enforce no-target, 0.90 uncovered-capability confidence, three independent runs, passing non-target checks, and explicit user/Curator request.
- [ ] 10.2 Restrict scope to canonical Project or User and validate ids against effective, shadowed, reserved, quarantined, archived, and recently rejected inventories.
- [ ] 10.3 Store validated `SKILL.md` bytes and hash in SQLite quarantine outside every Skill discovery path.
- [ ] 10.4 Reject scripts, tools, references, templates, assets, configuration, executable content, credentials, and external dependencies.
- [ ] 10.5 Implement safe creation preview with id, scope, type, frontmatter, instructions, tokens, tools, collision report, and catalog witnesses.
- [ ] 10.6 Add no-target eligibility, broad-capability, collision, scope, forbidden content, quarantine discovery, and stale catalog tests.

## 11. Curator Handoff and Skill Creation

- [ ] 11.1 Package immutable review data with dossier, job/attempt, citations, rendered artifact, validation, preview, model usage, consent, and permanent auto-exclusion provenance.
- [ ] 11.2 Attach existing-Skill packages idempotently as Curator draft revisions without approval or application.
- [ ] 11.3 Add specialized Curator creation candidates for quarantined new Skills with interactive review and audit history.
- [ ] 11.4 Preserve model provenance and permanent auto exclusion on every user-edited derivative.
- [ ] 11.5 Commit approved new Skills through the normal conflict-safe Skill creation transaction with current proposal, workspace, catalog, scope, and id witnesses.
- [ ] 11.6 Add handoff idempotency, edited derivative, auto-gate rejection, stale preview, creation collision, transaction recovery, and provenance tests.

## 12. Orchestration Integration, Retention, and Purge

- [ ] 12.1 Add optional bounded generation dispatch inside the orchestration `route_governance` stage without changing the fixed eight-stage contract.
- [ ] 12.2 Keep failed, blocked, cancelled, or budget-limited jobs non-blocking for manual Curator drafting and Agent execution.
- [ ] 12.3 Implement 180-day failed/cancelled and 365-day completed-package retention with workspace policy bounds.
- [ ] 12.4 Cascade evidence purge through uncommitted dossiers, jobs, drafts, and quarantine while retaining minimal committed governance tombstones.
- [ ] 12.5 Add orchestration budget, duplicate dispatch, manual fallback, retention, purge, exported-file disclosure, and fail-open tests.

## 13. Service Boundary and Web Adapter

- [ ] 13.1 Add typed generation policy, job, stage, dossier, attempt, model/tool provenance, draft, validation, quarantine, export, cancel, regenerate, and handoff contracts to `agent-service.ts`.
- [ ] 13.2 Add Rust/Tauri commands with typed boundary errors and all native invocation mappings isolated in `tauri-agent-client.ts`.
- [ ] 13.3 Implement deterministic Web/mock seven-stage jobs, three artifact kinds, repair, cancellation, supersession, cost counters, export, and handoff with mock provenance.
- [ ] 13.4 Add adapter contract tests for pagination, bounded payloads, consent, status/error parity, cancellation, regeneration, export, and quarantine behavior.

## 14. Generation and Dossier UI

- [ ] 14.1 Add generation disclosure, consent, provider readiness, budgets, usage, and permanent Curator/manual status controls.
- [ ] 14.2 Add job queue/detail with seven stages, attempts, budgets, costs, cancellation, supersession, safe failures, and handoff status.
- [ ] 14.3 Add the ordered thirteen-section dossier inspector with completeness, redaction, truncation, source links, version hashes, pagination, and JSON/Markdown export.
- [ ] 14.4 Add immutable attempt comparison and rendered learned-guidance, exact-patch, or new-Skill proposal review with citations and validation matrix.
- [ ] 14.5 Add Overlay diff or new-Skill creation preview and navigation to Curator without direct apply/install controls.
- [ ] 14.6 Add regeneration as a linked new attempt while preserving prior dossiers and drafts.
- [ ] 14.7 Keep production modules below 300 lines and add localization, responsive, dark-theme, keyboard, focus, screen-reader, loading, empty, error, and Web/mock tests.

## 15. Notifications and Full Verification

- [ ] 15.1 Publish deduplicated safe review-ready, attention-required failure, cancellation, and supersession notifications with navigation-only actions.
- [ ] 15.2 Add notification privacy, deduplication, localization, navigation, routine-stage suppression, and failure-isolation tests.
- [ ] 15.3 Run privacy, prompt-injection, citation-integrity, structured-output, renderer, exact-anchor, new-Skill, quarantine, provenance, and auto-exclusion corpora.
- [ ] 15.4 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [ ] 15.5 Run `npm run build` and `npx playwright test` for generation, dossier, export, cancellation, draft review, and Curator handoff.
- [ ] 15.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 15.7 Run `openspec validate add-skill-evolution-generation-agent-and-evidence-dossiers --strict`, `openspec validate --specs --strict`, and repository documentation checks.
- [ ] 15.8 Verify generation-disabled, provider-unavailable, database-unavailable, cancellation, stale input, Curator-unavailable, quarantine-failed, and rollback scenarios leave every Agent and existing evolution stage operational.
