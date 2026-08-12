## Context

See `proposal.md` for motivation and the delta specifications for behavior. The prerequisite changes deliberately expose independent idempotent boundaries: evidence ingestion, seed construction, assessment, Curator intake/application, and Overlay mutation. Orchestration must coordinate those boundaries without duplicating their data or converting their recoverable operations into one monolithic transaction.

Automatic application is materially different from normal Curator approval because no interactive user confirms a specific preview. It therefore needs its own policy authorization, a much narrower draft provenance, stricter thresholds, rate limits, final preflight, and circuit breakers. It must never fabricate an interactive actor.

## Goals / Non-Goals

**Goals:**

- Convert runtime and maintenance signals into durable, coalesced evolution runs.
- Run expensive maintenance only under bounded concurrency and observable idle conditions.
- Resume safely after interruption using subsystem idempotency and stage checkpoints.
- Provide a useful observe mode before any automatic mutation is enabled.
- Permit only deterministic, explicitly authorized learned guidance through auto-apply.
- Reuse Curator and Overlay application recovery rather than creating a second write path.
- Detect and suspend unsafe automatic behavior quickly without autonomous rollback.
- Keep the desktop background lifecycle and Web/mock limitations honest.

**Non-Goals:**

- Creating scheduled Agent conversations or a system-owned chat session.
- Generating drafts with an LLM, Role Skill, Utility Skill, or external CLI.
- Automatically applying exact patches, files, scripts, tools, or permissions.
- Automatically approving existing user-authored Curator drafts.
- Automatically reverting an Overlay.
- Running after the desktop process or browser page exits.
- Distributed scheduling across machines or remote accounts.

## Decisions

### 1. Build an internal orchestrator, not a scheduled Agent task

Add a Rust `skill_evolution_orchestration` context with ports for trigger receipt, runtime-idle projection, pipeline stages, policy, Curator/Overlay application, probation observations, notifications, clock, and unified diagnostics.

The existing scheduled-task capability creates user-visible Agent sessions and sends user task content. Evolution runs do neither, so reusing that entity would mix identity, recurrence, session, and availability semantics. The orchestrator uses the application's lifecycle supervisor and SQLite but owns separate records and does not register an Agent.

Alternative considered: model evolution as a recurring OnePiece task. Rejected because this would limit orchestration to one Agent, create unwanted sessions, and place background governance behind model availability.

### 2. Normalize exactly ten trigger envelopes

Each producer emits a closed `EvolutionTriggerEnvelope` with version, family, workspace, source kind/id/revision, occurred time, priority, and safe reason codes. The ten families in the spec are exhaustive. Unknown families fail closed so new triggers require a policy and migration review.

Receipts are idempotent on `(family, source_kind, source_id, source_revision)`. Compatible automatic triggers debounce for 30 seconds per workspace. Counters retain each contributing family even when one run request is created. Manual triggers bypass debounce but join an already-active workspace follow-up request.

Periodic maintenance checks every 15 minutes while the desktop process lives; it creates work only if a workspace has due maintenance or backlog. Startup produces one recovery request rather than replaying every missed interval.

Alternative considered: use only a periodic scan. Rejected because feedback and verification would be delayed and unnecessary full scans would increase background cost.

### 3. Aggregate idle from authoritative runtime leases

`EvolutionIdleSnapshot` is assembled from runtime-owned state rather than UI focus alone:

- active Agent generation and starting leases;
- managed CLI process and delegated Utility leases;
- pending approval and verification leases;
- Skill catalog, Overlay, Curator, and application-saga writer leases;
- lifecycle shutdown flag;
- last foreground user interaction time;
- CPU/memory pressure category when available.

Automatic runs require all activity leases clear and 60 seconds of user inactivity. They wait up to 15 minutes, then checkpoint as deferred without polling faster than state-change notifications or a bounded timer. Mutation preflight requires a fresh snapshot no older than five seconds.

Manual runs can start read-only stages without the user-idle interval, but writer stages still require all writer and runtime mutation leases clear. This gives users responsive inspection without allowing “Run now” to become a safety bypass.

Alternative considered: define idle as no visible chat streaming. Rejected because CLI, approvals, verification, background delegation, and Overlay writes can remain active independently.

### 4. Use durable run requests and workspace single-flight

One workspace has at most one active run lease and one folded follow-up request. Version 1 global concurrency is two read-only runs, but only one run may enter an automatic mutation stage globally. Leases have heartbeat and expiry; recovery never assumes expiry means a write failed—it queries subsystem receipts.

Run status and stages use optimistic versions. A manual request returns the existing active run plus follow-up acknowledgement when single-flight is occupied. No command creates a parallel run to reduce perceived wait time.

Alternative considered: one global run for all workspaces. Rejected because a large workspace would starve independent work and make policy/consent witnesses hard to isolate.

### 5. Execute eight fixed checkpointed stages

The run engine executes:

1. `recover`: reconcile pipeline receipts, Curator outbox, Overlay history, and nonterminal prior runs.
2. `maintain_evidence`: retention, quotas, receipts, and pending extractor work.
3. `build_seeds`: deterministic candidate grouping and dirty-group rebuild.
4. `assess`: ready-seed selection and quality evaluation within policy.
5. `route_governance`: enqueue/supersede Curator candidates.
6. `evaluate_auto_apply`: draft production, eligibility, observe/apply, and probation registration.
7. `project_results`: update safe summaries, counters, and health.
8. `notify`: publish attention-required outcomes.

Each item receives a run-stage idempotency key. Stage cursors reference stable ordered record ids and versions, not database offsets. A stage result is `completed`, `skipped_empty`, `partial_budget`, `deferred_idle`, `failed_retryable`, or `failed_terminal`. Retryable failure does not erase previous subsystem commits.

Alternative considered: use one database transaction for a run. Rejected because model calls, filesystem transactions, and bounded continuation cannot safely remain inside one SQLite transaction.

### 6. Enforce explicit default budgets

Version 1 run budgets are:

| Resource | Automatic | Manual |
|---|---:|---:|
| Wall time | 120 seconds | 300 seconds |
| Evidence items | 1,000 | 5,000 |
| Seed groups | 100 | 500 |
| Assessments | 25 | 100 |
| Optional model calls | 10 | 25 |
| Notifications | 20 | 50 |
| Automatic mutations | 1 | 1 |

Sub-budgets are consumed using committed receipts so retries cannot reset them. Maintenance needed for security or transaction recovery has a small reserved budget and precedes ordinary items. Remaining work becomes a continuation request with exponential backoff capped at 15 minutes.

Alternative considered: run until backlog is empty. Rejected because a burst could monopolize CPU, provider budget, SQLite, or the Overlay writer.

### 7. Recover by reconciling subsystem receipts

SQLite stores the run and stage checkpoint before dispatching an item. The target subsystem stores the same item idempotency key with its committed result. After interruption, recovery checks:

- evidence receipt and seed builder version;
- assessment witness/result;
- Curator candidate identity;
- Curator application outbox and Overlay application id;
- notification delivery receipt.

It marks the item committed if the authoritative receipt exists, otherwise reruns only idempotent read/write operations. It never reissues an automatic application solely because the run checkpoint is missing. Startup schedules one recovery run and then normal due work.

Alternative considered: mark every interrupted run failed and start over. Rejected because it can duplicate external model cost, queue records, notifications, and mutation attempts.

### 8. Keep cancellation cooperative and shutdown ordered

Cancel sets a durable flag checked between bounded items and before any application intent. It cannot abort a SQLite commit or Overlay transaction. Once an automatic application intent exists, Curator recovery determines the result before the run becomes cancelled or recovered.

Graceful quit ordering is:

1. stop accepting new triggers except recovery-critical receipts;
2. mark active runs cancel requested;
3. stop dispatching new items;
4. let current transactions and application saga reach a known state within timeout;
5. persist checkpoints and release leases;
6. exit with warnings through unified logging if bounded shutdown expires.

Alternative considered: kill worker tasks immediately. Rejected because it increases ambiguous cross-store state during the most sensitive stage.

### 9. Separate orchestration mode from auto-apply authorization

`EvolutionOrchestrationPolicy` contains mode, consent version, allowed stable Skill ids, budgets, idle timing, notification preferences, and policy revision. `off` prevents automatic runs but still permits manual inspection and necessary recovery. `observe` runs all decision logic without application intent. `enabled` allows the auto gate to proceed.

Enabling requires a current disclosure acknowledgement and at least one allowlisted Skill. Removing a Skill or revoking consent advances policy revision immediately. No wildcard allowlist exists in version 1. Imported policies never carry consent; users must confirm locally.

Alternative considered: one global “self evolution” switch. Rejected because users need a no-write observation phase and per-Skill control.

### 10. Add explicit authorization to correction feedback

Feedback state gains `reusable_guidance_authorization` with `authorized`, feedback revision, disclosure version, created/revoked time, and local actor class. It defaults absent/false. Replacing correction text revokes old authorization because consent covered exact content.

The chat UI disclosure says the correction can become a learned-guidance proposal and, only under separately enabled strict auto policy, may be applied automatically. Normal feedback remains fully usable without authorization. Revocation emits a trigger so pending drafts and eligibility decisions become stale.

Alternative considered: infer authorization from submitting a correction. Rejected because feedback about one answer is not automatically consent to modify future Agent instructions.

### 11. Generate only canonical correction learn blocks

The sole automatic producer accepts an authorized sanitized correction plus deterministic lesson shape. It requires nonempty trigger, guidance action, and verification fields already supported by evidence; it never asks a model to fill them.

Canonical bytes are:

```markdown
### Verified correction guidance

- Trigger: <bounded structured trigger>
- Guidance: <authorized sanitized correction guidance>
- Verify: <bounded structured verification>
```

Line endings, whitespace, Unicode normalization, heading, and field order are fixed by producer version. The result is capped at 2 KiB, rescanned, and draft-bound quality reviewed. It is tagged `deterministic_authorized_correction`; user-authored, model-authored, imported, edited, or unknown provenance never qualifies.

Alternative considered: reuse any Curator learn-block draft. Rejected because human authorship signals review intent, not permission for unattended application, and arbitrary editing destroys deterministic provenance.

### 12. Implement eligibility as an all-condition proof

`AutoApplyEligibility` stores a result for every required predicate from the spec. No weighted score can compensate for a failure. The assessment must be deterministic: no model consultation was needed for target choice, and model judging cannot be required to reach `advance`. Attribution must be verified, not merely correlated.

Support requires either:

- at least three independent compatible runs; or
- one authorized verified correction plus two independent compatible confirmations.

Target must be current, allowlisted, mutable, unpinned, and have a healthy trusted Overlay chain. Draft must pass the exact nine draft-bound checks, remain low risk, and have system confidence at least 0.95. Any exclusion goes to Curator with safe reason, except observe mode which records only the hypothetical route.

Alternative considered: auto-apply every low-risk `advance` assessment. Rejected because the assessment threshold was designed to advance into governance, not authorize unattended mutation.

### 13. Repeat all mutable checks at final preflight

Eligibility creates no mutation authority. Immediately before application intent, preflight reloads:

- orchestration policy, consent, allowlist, and mode;
- correction authorization revision;
- assessment, target, draft, quality, and confidence;
- base/effective Skill, Overlay revision, trust, conflict, scanner, and pin state;
- current idle snapshot, rate counters, cooldown, probation, and breakers.

It hashes the proposed current-to-effective diff using the normal Overlay preview boundary. Any mismatch routes to Curator and invalidates eligibility. Preflight witness expires after five seconds and is consumed once by the application saga.

Alternative considered: reuse the earlier Curator preview token. Rejected because unattended queue delay makes short-lived mutable state likely to drift.

### 14. Extend the Curator saga with system-policy authorization

The Curator application coordinator accepts either interactive approval or `SystemPolicyAuthorization`. The latter includes run id, eligibility proof hash, preflight hash, policy/consent revisions, rate reservation, and actor `system_policy`. It never creates a fake approval decision.

The same outbox-first sequence and Overlay application id protect cross-store consistency. Overlay history records automatic provenance, policy version, prior effective hash, and probation id. Rate counters finalize with the application result. Recovery queries application id and cannot create a second commit.

Alternative considered: call Overlay apply directly from the run worker. Rejected because it would bypass Curator audit, outbox recovery, and consistent governance history.

### 15. Reserve rate capacity transactionally

Before application intent, Curator atomically checks and reserves:

- one automatic mutation per run;
- fewer than three committed or in-flight mutations for the workspace rolling 24-hour window;
- no committed or in-flight mutation for the Skill in the prior seven days.

Reservations expire only after recovery proves no application committed. Failures remain counted for breaker analysis; an integrity-related failure always counts toward the conservative window. Observe decisions do not reserve or consume rate capacity but report whether it was available.

Alternative considered: count only successful commits. Rejected because repeated failing attempts could hammer a corrupted target indefinitely.

### 16. Use hierarchical circuit breakers

There is one workspace breaker plus per-Skill suspension. Immediate workspace-open categories are scanner/integrity/audit/idempotency violations. Two application failures in 24 hours also open the workspace breaker. A probation regression suspends the affected Skill; a security-related regression also opens the workspace breaker.

Breaker state includes cause category, source application/run, opened time, health-check version, acknowledgement state, and last probe. Background runs may continue read-only pipeline work. Closing requires an interactive acknowledgement and a passing deterministic health probe; acknowledgement alone cannot force close.

Alternative considered: automatically retry after a timer. Rejected because integrity and policy failures need explicit investigation, not time-based optimism.

### 17. Monitor probation conservatively

An automatic application creates a seven-day probation baseline using the same task fingerprint, target revision, evidence categories, and pre-application outcome rates. Monitoring consumes only structured sanitized signals.

A regression condition is:

- two independent verified compatible negative outcomes after application that exceed the versioned baseline threshold; or
- one verified explicit harmful-result correction directly linked to the new effective revision.

Because participation is not causality, the response is suspension and Curator rollback review, not automatic revert. The rollback candidate includes the application, prior and current effective hashes, probation observations, and safe Overlay revert preview link. Healthy expiry closes probation but does not erase history.

Alternative considered: auto-revert on first failure. Rejected because unrelated failures could remove useful guidance and rollback is itself a consequential mutation.

### 18. Persist orchestration as normalized durable state

Add SQLite tables:

- `evolution_trigger_receipts`
- `evolution_run_requests`
- `evolution_runs`
- `evolution_run_trigger_links`
- `evolution_run_stages`
- `evolution_run_items`
- `evolution_run_checkpoints`
- `evolution_orchestration_policy`
- `evolution_correction_authorizations`
- `evolution_deterministic_drafts`
- `evolution_auto_eligibility`
- `evolution_auto_rate_reservations`
- `evolution_auto_breakers`
- `evolution_auto_applications`
- `evolution_auto_probations`
- `evolution_probation_observations`

Large or sensitive content is not stored in orchestration tables. They retain ids, hashes, enums, counts, versions, safe reasons, and subsystem references. Retention keeps runs 90 days, detailed eligibility 180 days, and automatic-application/probation audit aligned with Curator/Overlay history tombstones.

Alternative considered: reconstruct runs only from unified logs. Rejected because logs are diagnostic, not transactional scheduler state, and must not be parsed as a control plane.

### 19. Integrate background and Web lifecycles honestly

The desktop lifecycle supervisor starts trigger consumers and recovery after database and Overlay recovery are ready. Hiding to tray does not pause the process; idle policy remains authoritative. Graceful quit stops orchestration before connector/process exit completes.

Web/mock uses an in-memory scheduler driven while the page is active. It exposes the same states, gates, and simulated applications with `mock` provenance, but no OS timer after page close, startup crash recovery claim, filesystem mutation, or tray behavior.

Alternative considered: hide unavailable Web controls. Rejected because policy and state UX need adapter testing, but capability labels must prevent users interpreting mocks as native effects.

### 20. Expose monitoring and control through the Skill service

Extend `agent-service.ts` with typed models for trigger summaries, idle reasons, run/stage/checkpoint history, policy/consent, allowlist, eligibility proof, deterministic draft provenance, application, probation, and breaker health. Tauri invocation remains only in `tauri-agent-client.ts`; Web simulation remains in `web-agent-client.ts`.

The Skill Evolution UI includes orchestration overview, run detail, policy setup, observe decisions, automatic history, probation, and breakers. It never shows raw authorized correction bodies or diff content in list/notification surfaces. Manual run and cancel are cooperative controls, not gate bypasses. Breaker acknowledgement is separate from health verification and never performs rollback.

Alternative considered: expose only a background on/off toggle. Rejected because unattended mutation requires explainable eligibility, history, limits, and recovery state.

## Risks / Trade-offs

- [Ten triggers generate excessive runs] → Deduplicate receipts, debounce bursts, single-flight workspaces, and fold triggers into checkpoints.
- [Idle detection is inaccurate] → Aggregate authoritative leases and require a fresh quiescence snapshot before mutation.
- [Manual run becomes a bypass] → Permit read-only progress while preserving writer, policy, rate, breaker, and preflight gates.
- [Authorized correction is still too vague] → Require structured trigger/action/verification, deterministic bytes, draft-bound gates, and independent confirmations.
- [Auto threshold creates very few applications] → Observe mode measures eligibility safely; thresholds can change only through a versioned future policy review.
- [Model influence leaks into auto-apply] → Require deterministic target and route, deterministic draft provenance, and no model dependency for `advance`.
- [Crash duplicates mutation] → Use run receipts plus Curator outbox and Overlay application ids; recovery never retries from run state alone.
- [Rate reservations become stuck] → Reconcile against authoritative application history before expiration or release.
- [False regression suspends a Skill] → Require verified independent compatible outcomes and route rollback to human review rather than reverting.
- [Background work consumes resources] → Enforce idle, pressure, wall-time, item, model, and concurrency budgets.
- [Users forget automation is enabled] → Show persistent mode/allowlist state, notify each automatic commit, and include probation history.
- [Web mocks imply real background behavior] → Mark mock provenance and explicitly describe page-active and no-filesystem limits.

## Migration Plan

1. Complete and verify effective runtime, Overlay, evidence, assessment, and Curator prerequisites, including application-id recovery.
2. Add trigger, run, stage, policy, consent, eligibility, rate, breaker, and probation domain models and migrations with orchestration disabled.
3. Add trigger receipts, debounce, workspace single-flight, idle aggregation, budgets, checkpoints, cancellation, startup recovery, and pure scheduler tests.
4. Wire read-only stages incrementally and verify subsystem idempotency under repeated and interrupted runs.
5. Add service contracts, Tauri and Web/mock adapters, run monitoring UI, and manual read-only execution.
6. Add correction authorization and deterministic learn-block production with privacy, injection, quality, and reproducibility tests.
7. Add observe mode, full eligibility proof, rate simulation, preflight simulation, policy UI, and collect local fixture results without mutations.
8. Extend Curator saga for `system_policy`, add rate reservations, application audit, probation, breaker logic, and crash-point recovery behind a disabled feature flag.
9. Enable automatic application only for test fixtures, then opt-in development workspaces with one allowlisted Skill and inspect every result.
10. Add tray-background lifecycle, notifications, regression-to-Curator routing, E2E coverage, and full project validation before exposing enabled mode generally.

Rollback first changes all policies to `off`, opens the workspace breaker, stops new triggers, and lets active application sagas reconcile. It then disables scheduler dispatch and UI mutation controls while preserving completed Overlays, Curator/Overlay audit, runs, and probation records. Rollback does not automatically revert applied guidance. Re-enabling performs startup recovery, health probes, and explicit user consent renewal before any automatic mutation.
