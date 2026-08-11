## 1. Orchestration Domain and Persistence

- [ ] 1.1 Add the `skill_evolution_orchestration` Rust module and versioned enums for ten triggers, policy mode, run/stage status, checkpoints, eligibility, rate reservation, breaker, probation, and actor provenance.
- [ ] 1.2 Define trigger envelope, idle snapshot, run request, run/stage/item, budget, policy/consent, deterministic draft, eligibility proof, application, and probation models.
- [ ] 1.3 Add SQLite migrations for trigger receipts, requests, runs, trigger links, stages, items, checkpoints, policy, correction authorization, drafts, eligibility, rate reservations, breakers, applications, probations, and observations.
- [ ] 1.4 Implement canonical hashing, optimistic versions, idempotency keys, leases, and normalized safe persistence.
- [ ] 1.5 Add migration, serialization, compatibility, corruption-boundary, and concurrent-version tests.

## 2. Ten Trigger Producers and Coalescing

- [ ] 2.1 Implement the closed versioned trigger registry for exactly the ten specified families and reject unknown families.
- [ ] 2.2 Project startup recovery, periodic maintenance, and application-idle transition triggers.
- [ ] 2.3 Project Agent run, conversation, verification, and delegated Utility completion triggers from authoritative lifecycle events.
- [ ] 2.4 Project explicit feedback and relevant Skill, Overlay, and policy mutation triggers.
- [ ] 2.5 Implement manual run triggers through the orchestration boundary.
- [ ] 2.6 Implement receipt idempotency, 30-second workspace debounce, per-family counters, and folded follow-up requests.
- [ ] 2.7 Add duplicate, burst, out-of-order, unknown-version, cross-workspace, and active-run coalescing tests.

## 3. Idle and Quiescence Gate

- [ ] 3.1 Aggregate Agent generation, CLI process, delegated Utility, approval, verification, Skill/Overlay/Curator writer, shutdown, user-interaction, and resource-pressure leases.
- [ ] 3.2 Implement the 60-second automatic user-idle interval, state-change wakeup, 15-minute maximum wait, and deferred checkpoint.
- [ ] 3.3 Allow manual read-only progress without user idle while preserving all writer, shutdown, policy, rate, and mutation gates.
- [ ] 3.4 Require a fresh idle snapshot within five seconds for mutation preflight.
- [ ] 3.5 Add race tests for generation start, approval wait, verification, Overlay commit, Curator review, shutdown, and stale idle snapshots.

## 4. Durable Scheduler and Run Engine

- [ ] 4.1 Implement workspace single-flight, one folded follow-up request, two-run global read concurrency, and one global automatic-mutation lane.
- [ ] 4.2 Implement run lifecycle, heartbeats, lease expiry, cooperative cancellation, and stable status transitions.
- [ ] 4.3 Implement the eight ordered stages and per-item subsystem idempotency receipts.
- [ ] 4.4 Implement stable record-version cursors and transactional checkpoints rather than offset pagination.
- [ ] 4.5 Implement the version-1 automatic and manual budgets for time, items, assessments, model calls, notifications, and mutations.
- [ ] 4.6 Implement partial-budget continuation with bounded exponential backoff and reserved recovery capacity.
- [ ] 4.7 Add state-machine, single-flight, concurrency, budget-boundary, cancellation, and partial-continuation tests.

## 5. Startup and Crash Recovery

- [ ] 5.1 Reconcile nonterminal evidence, assessment, Curator, Overlay, notification, run, and rate-reservation receipts on startup.
- [ ] 5.2 Resume from the last safe checkpoint without duplicating seeds, assessments, candidates, notifications, or applications.
- [ ] 5.3 Schedule at most one startup recovery run instead of replaying missed periodic intervals.
- [ ] 5.4 Implement graceful shutdown ordering that stops triggers, checkpoints work, and lets active application sagas reach known state.
- [ ] 5.5 Add crash-point tests around every stage dispatch/receipt boundary and after Overlay commit but before run finalization.

## 6. Policy, Consent, and Skill Allowlist

- [ ] 6.1 Implement versioned workspace `off`, `observe`, and `enabled` policy with default `off`.
- [ ] 6.2 Require current behavior disclosure consent and at least one stable Skill id before enabled mode can auto-apply.
- [ ] 6.3 Implement explicit per-Skill allowlist without wildcard support and invalidate affected eligibility on removal.
- [ ] 6.4 Ensure imported policy never carries local consent and revocation takes effect before final preflight.
- [ ] 6.5 Add conflict-safe policy updates, unsupported-field rejection, disclosure-version, revocation, and allowlist tests.

## 7. Reusable Correction Authorization

- [ ] 7.1 Extend feedback models and persistence with default-off authorization bound to one exact correction revision and disclosure version.
- [ ] 7.2 Revoke authorization automatically when correction content is replaced and explicitly when the user requests revocation.
- [ ] 7.3 Emit safe policy triggers that stale derived drafts and eligibility without changing ordinary feedback evidence.
- [ ] 7.4 Add matching Tauri and Web/mock service methods and chat controls without component-level `invoke()` calls.
- [ ] 7.5 Add service, component, privacy, replacement, revocation, stale-race, and Web parity tests.

## 8. Deterministic Correction Draft Producer

- [ ] 8.1 Implement the sole registered automatic producer for canonical authorized-correction `OverlayLearnBlock` drafts.
- [ ] 8.2 Require structured trigger, guidance, and verification fields and refuse to invent missing prose.
- [ ] 8.3 Canonicalize Unicode, whitespace, line endings, heading, field order, and 2 KiB output limit under a producer version.
- [ ] 8.4 Run sanitization, injection scanning, Overlay validation, and exact draft-bound nine-check assessment.
- [ ] 8.5 Mark user-authored, edited, model-generated, imported, patch, file, script, and unknown draft provenance permanently ineligible.
- [ ] 8.6 Add byte-reproducibility, incomplete-shape, unsafe-content, authorization, edit, and provenance fixtures.

## 9. Observe Mode and Auto-Apply Eligibility

- [ ] 9.1 Implement an all-condition eligibility proof with one explicit result for every required policy, target, evidence, quality, draft, lifecycle, rate, idle, and breaker predicate.
- [ ] 9.2 Require deterministic clear verified target, low risk, confidence at least 0.95, nine passing checks, and the specified independent support threshold.
- [ ] 9.3 Implement all permanent exclusions, including model-resolved targets, correlated attribution, exact patches, executable content, untrusted provenance, pinned targets, and System-scope escalation.
- [ ] 9.4 Implement observe-mode `would_apply` results without application intent, rate reservation, Overlay mutation, or misleading success state.
- [ ] 9.5 Route failed enabled-mode eligibility to Curator or waiting state using stable safe reasons.
- [ ] 9.6 Add exhaustive table-driven eligibility tests proving no weighted score or model confidence can compensate for a failed predicate.

## 10. Rate Limits and Final Preflight

- [ ] 10.1 Implement transactional reservations for one automatic mutation per run, three per workspace per rolling 24 hours, and one per Skill per seven days.
- [ ] 10.2 Reconcile stuck reservations against Curator and Overlay application history before release.
- [ ] 10.3 Revalidate policy, consent, authorization, allowlist, assessment, draft, target, Skill, Overlay, trust, pin, quality, rate, idle, probation, and breaker witnesses immediately before intent.
- [ ] 10.4 Generate and consume a single-use five-second preflight witness bound to the current Overlay preview diff hash.
- [ ] 10.5 Add rolling-window, concurrent reservation, failed attempt, stale preflight, pin race, consent race, and Overlay drift tests.

## 11. System-Policy Application and Recovery

- [ ] 11.1 Extend Curator application authorization with a distinct `system_policy` actor and policy proof without synthesizing interactive approval.
- [ ] 11.2 Persist run, eligibility, preflight, policy, rate, Curator outbox, and Overlay application ids in one recoverable provenance chain.
- [ ] 11.3 Commit eligible learned guidance only through Curator and Overlay scanner, CAS, history, trust, pin, usage, and filesystem recovery boundaries.
- [ ] 11.4 Finalize rate counters, run results, automatic application, and probation against the same idempotent application id.
- [ ] 11.5 Add crash recovery and duplicate prevention around intent, Overlay commit, Curator finalization, rate finalization, and run finalization.
- [ ] 11.6 Verify exact patches, user drafts, model drafts, files, scripts, tools, permissions, and direct Overlay calls have no automatic application path.

## 12. Circuit Breakers and Probation

- [ ] 12.1 Implement workspace breakers for security, integrity, audit, idempotency, and two application failures in 24 hours.
- [ ] 12.2 Implement per-Skill suspension for verified probation regression and workspace escalation for security-related regression.
- [ ] 12.3 Require deterministic health probes plus interactive acknowledgement before a breaker can close.
- [ ] 12.4 Create seven-day probation baselines linked to application, prior/current effective revisions, target, fingerprint, and evidence categories.
- [ ] 12.5 Evaluate structured verified outcomes using the two-independent-negative or one verified harmful-correction thresholds without claiming causality.
- [ ] 12.6 Create Curator rollback-review candidates and notifications on regression without automatically reverting Overlay content.
- [ ] 12.7 Add breaker opening, repeated failure, unresolved acknowledgement, healthy close, false-correlation, probation expiry, regression, and no-auto-revert tests.

## 13. Service Boundary, Background Lifecycle, and Notifications

- [ ] 13.1 Add typed scheduler, trigger, idle, run, stage, checkpoint, policy, eligibility, application, probation, and breaker contracts to `agent-service.ts`.
- [ ] 13.2 Add Rust/Tauri query and mutation commands with typed boundary errors and all native invocations isolated in `tauri-agent-client.ts`.
- [ ] 13.3 Implement Web/mock page-active scheduling, policy, observe, simulated application, probation, and breaker behavior with explicit mock provenance.
- [ ] 13.4 Integrate desktop startup, tray-idle processing, and graceful quit without claiming work continues after process exit or tray fallback close.
- [ ] 13.5 Publish deduplicated safe run-attention, automatic-application, regression, and breaker notifications with navigation-only actions.
- [ ] 13.6 Add desktop/Web adapter contracts, tray lifecycle, page-close, notification privacy, deduplication, and failure-isolation tests.

## 14. Skill Evolution Orchestration UI

- [ ] 14.1 Add scheduler overview with mode, idle gate, trigger counters, pending work, active/recent runs, stages, checkpoints, budgets, and safe failures.
- [ ] 14.2 Add policy disclosure, off/observe/enabled controls, per-Skill allowlist, fixed exclusions, rates, cooldowns, and desktop/Web capability labels.
- [ ] 14.3 Add eligibility proof and observe-mode inspection with every condition, draft provenance, final-preflight state, and Curator routing link.
- [ ] 14.4 Add automatic application history and seven-day probation outcome views without exposing correction or diff content in list surfaces.
- [ ] 14.5 Add breaker and Skill suspension views with cause, health probe, acknowledgement, and Curator rollback-review navigation but no auto rollback.
- [ ] 14.6 Add cooperative manual-run and cancellation controls that explain non-bypassable gates.
- [ ] 14.7 Keep production modules below 300 lines and add localization, responsive, dark-theme, keyboard, focus, screen-reader, loading, empty, error, and Web/mock tests.

## 15. End-to-End and Project Verification

- [ ] 15.1 Add E2E flows for trigger coalescing, idle wait, manual read-only run, partial continuation, cancellation, startup recovery, and tray-background run.
- [ ] 15.2 Add E2E policy flows for default off, observe would-apply, consent, allowlist, enabled application, revocation race, limits, and Curator fallback.
- [ ] 15.3 Add E2E safety flows for model-assisted exclusion, exact-patch exclusion, pinned target, stale preflight, breaker, probation regression, and rollback review.
- [ ] 15.4 Run privacy, prompt-injection, idempotency, crash-recovery, state-machine, rate, breaker, probation, unified-log, and notification corpora.
- [ ] 15.5 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [ ] 15.6 Run `npm run build` and `npx playwright test` for orchestration, policy, background, and automatic-application behavior.
- [ ] 15.7 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 15.8 Run `openspec validate add-skill-evolution-orchestration-and-auto-apply-gate --strict`, `openspec validate --specs --strict`, and repository documentation checks.
- [ ] 15.9 Verify policy-off, database-unavailable, queue-saturated, shutdown, Curator-unavailable, Overlay-failed, breaker-open, and rollback scenarios leave all Agent families and existing pipeline data operational.
