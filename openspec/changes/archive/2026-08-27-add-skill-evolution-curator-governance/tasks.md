## 1. Curator Domain and State Machine

- [x] 1.1 Add the `skill_evolution_curation` Rust module and versioned enums for candidate state, staleness, draft kind, decision, rejection/defer reason, application status, actor class, and policy.
- [x] 1.2 Define candidate snapshot, draft revision, draft assessment, preview, decision, audit event, application, outbox, policy, and service projection models.
- [x] 1.3 Implement the allowed candidate transitions and terminal-state invariants as pure functions.
- [x] 1.4 Implement canonical witness hashing and optimistic revision checks for candidates, drafts, assessments, targets, Overlays, policies, and previews.
- [x] 1.5 Add serialization, compatibility, invalid-transition, and concurrent-version tests.

## 2. SQLite Schema and Audit Persistence

- [x] 2.1 Add migrations for candidates, sources, drafts, draft assessments, previews, decisions, events, applications, outbox, policy, and notification receipts.
- [x] 2.2 Implement transactional repositories with candidate compare-and-swap and action idempotency keys.
- [x] 2.3 Implement per-candidate ordered hash-chained audit events with locally derived actor class and timestamp.
- [x] 2.4 Store only validated draft bodies and safe normalized snapshots; exclude rejected unsafe content, raw prompts, provider payloads, and secrets.
- [x] 2.5 Add migration, rollback-compatibility, audit-chain corruption, transaction-failure, and concurrent-mutation tests.

## 3. Assessment Intake and Supersession

- [x] 3.1 Consume current assessment completion envelopes idempotently and reload authoritative sanitized assessment and evidence records.
- [x] 3.2 Enqueue only `advance` and `needs_human_review` routes and record non-approvable routes without creating mutable candidates.
- [x] 3.3 Persist immutable intake snapshots with source, target, risk, confidence, checks, policy, and witness hashes.
- [x] 3.4 Supersede open candidates when a new current assessment changes route, target, or source revision.
- [x] 3.5 Add duplicate-delivery, concurrent-intake, non-current, purged-evidence, and reassessment fixtures.

## 4. Constrained Draft Lifecycle

- [x] 4.1 Implement bounded learned-guidance drafts with target and scope inherited from the candidate.
- [x] 4.2 Implement single exact-match patch drafts with bounded old/new strings and `replace_all` defaulting to false.
- [x] 4.3 Reject target overrides, System-scope escalation, supporting files, scripts, tool registration, permission expansion, commands, and direct base edits.
- [x] 4.4 Run privacy sanitization, injection scanning, UTF-8 and Markdown validation, size checks, and Overlay dry validation before draft persistence.
- [x] 4.5 Create immutable draft revisions and invalidate prior draft assessment and preview after every edit.
- [x] 4.6 Add safety fixtures for prompt injection, executable extensions, shell content, unsafe paths, oversized input, exact-patch mismatch, and rejected-body logging.

## 5. Draft-Bound Quality Review

- [x] 5.1 Project each draft into a bounded lesson shape without generating additional mutation content.
- [x] 5.2 Reuse the nine deterministic quality checks with candidate, target, and exact draft hash witnesses.
- [x] 5.3 Apply optional constrained model judging only under the existing evaluation consent and sanitized payload rules.
- [x] 5.4 Block target-changing, hard-stop, executable, unsupported, or non-approvable draft results from `ready_for_review`.
- [x] 5.5 Add tests for safe refinement, materially changed guidance, target mismatch, deterministic fallback, and edited-draft reassessment.

## 6. Overlay Preview Orchestration

- [x] 6.1 Invoke the existing Overlay preview boundary for the exact current draft without reading Overlay files directly.
- [x] 6.2 Produce base-to-current, current-to-proposed, and base-to-proposed effective diff projections with explicit completeness metadata.
- [x] 6.3 Bind preview hashes to candidate, draft, assessment, target, base, effective content, Overlay, pin, trust, scanner, and policy witnesses.
- [x] 6.4 Enforce 15-minute preview expiry and invalidate previews on any relevant state change.
- [x] 6.5 Add preview tests for drift, conflicts, pinned refusal, trust changes, size limits, patch ambiguity, pagination, and expiry.

## 7. Human Decisions and Trusted Actor

- [x] 7.1 Derive `local_interactive_user`, `system`, and Web/mock actor classes at the runtime boundary and ignore client-supplied actor identities or timestamps.
- [x] 7.2 Implement rejection with required category, bounded sanitized note, terminal transition, and preview invalidation.
- [x] 7.3 Implement deferral with required reason, optional 1–180 day review time, and no automatic resume.
- [x] 7.4 Implement explicit resume with current-witness validation and correct return to awaiting-draft or ready-for-review.
- [x] 7.5 Require exact current preview confirmation for approval and reject expired, missing, stale, or mismatched witnesses.
- [x] 7.6 Add authorization, forged-actor, duplicate-action, stale-window, terminal-state, reject, defer, resume, and approval tests.

## 8. Overlay Application Saga

- [x] 8.1 Persist candidate `applying`, approval decision, application intent, and outbox record atomically before calling Overlay mutation.
- [x] 8.2 Extend Overlay mutation provenance and history lookup with an idempotent Curator application id.
- [x] 8.3 Revalidate every approval witness and require the committed effective diff hash to match the approved preview without rebasing.
- [x] 8.4 Finalize applied candidates with Overlay revision and history references or record safe `apply_failed` categories.
- [x] 8.5 Implement outbox recovery that reconciles committed Overlay history after crashes without silently replaying failed intents.
- [x] 8.6 Require a fresh preview and explicit approval before retrying any failed application.
- [x] 8.7 Add crash-point tests before intent commit, before Overlay commit, after Overlay commit, before finalization, and during recovery.
- [x] 8.8 Add idempotency, pinned, CAS, audit-failure, filesystem-recovery, usage-counter, and duplicate-history tests.

## 9. Policy, Retention, and Purge

- [x] 9.1 Implement versioned workspace policy for queue routes, reason requirements, defer bounds, retention, notification preferences, and display limits.
- [x] 9.2 Reject unknown automatic-apply, approve-all, or mutation-bypass policy fields and keep manual approval invariant.
- [x] 9.3 Invalidate affected previews when policy revisions change while preserving policy witnesses on historical decisions.
- [x] 9.4 Implement default 180-day open and 365-day terminal-detail retention with bounded configurable reductions.
- [x] 9.5 Integrate evidence purge by removing detailed lineage and removable draft content while retaining minimal applied decision tombstones and Overlay links.
- [x] 9.6 Add policy conflict, unsupported field, retention, open expiration, applied tombstone, and purge tests.

## 10. Service Boundary and Adapters

- [x] 10.1 Add typed Curator queue, detail, draft, preview, decision, audit, policy, and stable error contracts to `agent-service.ts`.
- [x] 10.2 Add Rust/Tauri commands for scoped queries and every conflict-safe Curator action using `Result<T, String>` or typed serialized errors.
- [x] 10.3 Implement all native invocation mappings in `tauri-agent-client.ts` without component-level `invoke()` calls.
- [x] 10.4 Implement Web/mock queue, revisions, conflicts, preview expiry, pinned refusal, supersession, application result, and recovery behavior.
- [x] 10.5 Add adapter contract tests for pagination, bounded diffs, state transitions, version conflicts, action idempotency, and desktop/Web error parity.

## 11. Curator Notifications

- [x] 11.1 Publish structured pending-review, deferral-date, supersession, rejection, apply-success, and apply-failure events through the notification service.
- [x] 11.2 Deduplicate notifications by candidate id, revision, and event type using delivery receipts.
- [x] 11.3 Limit notification payloads to safe Skill identity, state, risk, route, candidate id, and navigation target.
- [x] 11.4 Ensure notification actions only navigate and cannot decide, resume, retry, or apply candidates.
- [x] 11.5 Add notification deduplication, localization, sensitive-data exclusion, navigation, failure-isolation, and recovery tests.

## 12. Curator UI

- [x] 12.1 Add a service-backed Curator workspace with counts and filters for state, route, risk, Skill, age, readiness, staleness, and notification status.
- [x] 12.2 Add complete candidate review showing sanitized lineage, assessment, nine checks, target revision, Skill/Overlay state, draft history, and audit timeline.
- [x] 12.3 Add constrained learned-guidance and exact-patch editors with safe unsaved-state recovery and prohibited-operation explanations.
- [x] 12.4 Add the three-part diff preview, validation report, expiry state, and explicit per-candidate approval confirmation.
- [x] 12.5 Add reject, defer, and resume dialogs with required categories, bounded notes, review-after validation, and conflict handling.
- [x] 12.6 Add stale, superseded, pinned, apply-failed, recovery, and applied Overlay-history states.
- [x] 12.7 Add policy and retention controls while excluding auto-apply, approve-all, bulk approval, model approval, target override, and pin bypass.
- [x] 12.8 Keep new production modules below 300 lines and add localization, responsive, dark-theme, keyboard, focus, screen-reader, loading, empty, and error tests.

## 13. End-to-End and Project Verification

- [x] 13.1 Add E2E flows for awaiting draft, draft edit and reassessment, preview, approve and apply, reject, defer/resume, stale preview, pinned refusal, and apply retry.
- [x] 13.2 Add an end-to-end crash-recovery scenario proving durable approval intent and one Overlay history event.
- [x] 13.3 Run privacy, prompt-injection, audit-integrity, state-machine, Overlay recovery, and notification sanitization corpora.
- [x] 13.4 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 13.5 Run `npm run build` and `npx playwright test` for Curator UI and runtime-adapter behavior.
- [x] 13.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 13.7 Run `openspec validate add-skill-evolution-curator-governance --strict`, `openspec validate --specs --strict`, and repository documentation checks.
- [x] 13.8 Verify Curator-disabled, database-unavailable, notification-failed, audit-failed, and rollback scenarios leave every Agent, evidence, assessment, and existing Overlay consumer operational.
