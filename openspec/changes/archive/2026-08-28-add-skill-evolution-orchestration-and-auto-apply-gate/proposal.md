## Why

The evidence, assessment, Overlay, and Curator capabilities define safe individual operations but do not yet coordinate them into durable evolution runs. VaneHub needs a bounded scheduler and run engine that advances pipeline backlogs during safe idle periods, plus an explicitly opted-in auto-apply gate narrow enough to preserve user control.

## What Changes

- Add an internal Skill-evolution scheduler with exactly ten trigger families: startup recovery, periodic maintenance, application-idle transition, Agent run completion, conversation completion, explicit feedback commit, verification completion, delegated Utility completion, relevant Skill/Overlay/policy change, and manual run request.
- Coalesce and debounce triggers into durable workspace-scoped run requests with single-flight execution, bounded retries, counters, cancellation, crash recovery, and no unbounded missed-run replay.
- Add an idle and quiescence gate that prevents automatic runs from competing with active Agent generation, CLI execution, tool approval, verification, Skill/Overlay mutation, Curator review, or application shutdown.
- Add a staged evolution run engine for recovery, evidence maintenance, seed readiness, assessment, Curator routing, auto-apply evaluation, result projection, and notification; it does not create a user-visible Agent session.
- Add execution budgets for elapsed time, evidence items, seeds, assessments, model calls, notifications, and mutations, with partial-success checkpoints and continuation runs.
- Add workspace policy modes `off`, `observe`, and `enabled`. Automatic application is disabled by default and requires explicit versioned consent plus a per-Skill allowlist.
- Extend corrected-feedback capture with a default-off “reusable Skill guidance” authorization. Only authorized, sanitized, structured corrections may produce deterministic learned-guidance drafts.
- Add a strict auto-apply gate requiring a current deterministic `advance` assessment, clear verified target, low risk, system confidence of at least 0.95, all quality checks passing, complete current witnesses, eligible trusted draft provenance, idle state, rate-limit capacity, and no open circuit breaker.
- Restrict automatic mutation to one bounded `OverlayLearnBlock`; exact patches, files, scripts, tool definitions, permissions, model-generated drafts, user-authored Curator drafts, ambiguous targets, correlated-only attribution, pinned or untrusted targets, and any medium/high risk always route to Curator.
- Apply eligible automatic drafts through the Curator/Overlay application saga with system-policy authorization, immutable audit, CAS, scanner, trust, pin, recovery, usage, and history invariants.
- Add limits of one automatic mutation per run, three per workspace per 24 hours, and one per Skill per seven days. Integrity/security failures open a workspace circuit breaker; repeated application failures or verified probation regressions suspend automatic application and route review to Curator.
- Mark automatically applied guidance with a seven-day probation period, monitor structured verified outcomes, and create a Curator rollback-review candidate on regression; automatic rollback is not implemented in this change.
- Expose scheduler state, runs, stages, checkpoints, trigger counters, policy, observed eligibility decisions, automatic applications, probation, circuit breakers, and manual run/cancel controls through the Skill service boundary and both adapters.
- Add orchestration and auto-apply governance panels to the Skill Evolution UI, with clear desktop/Web capability differences and sanitized notifications.

## Capabilities

### New Capabilities

- `skill-evolution-orchestration`: Trigger scheduling, idle gating, durable staged runs, bounded resource use, deterministic draft production, governed auto-apply eligibility, rate limits, circuit breakers, probation, recovery, and result projection.

### Modified Capabilities

- `chat-experience`: Adds explicit default-off authorization for corrected feedback to become reusable Skill guidance and supports later revocation before application.
- `skill-management`: Adds orchestration status, run history, policy, eligibility, probation, circuit-breaker, manual-run, cancel, and consent operations through desktop and Web adapters.
- `settings-skill-management-ui`: Adds evolution-run monitoring, trigger and idle explanations, observe/enable policy controls, Skill allowlists, eligibility decisions, probation, and circuit-breaker recovery.
- `notification-system`: Adds sanitized, deduplicated evolution run, automatic application, probation regression, and circuit-breaker notifications.
- `desktop-background-lifecycle`: Allows internal evolution maintenance while the desktop app remains in the tray, while preserving graceful shutdown and preventing new mutation work during quit.

## Impact

- Desktop/runtime: adds a Rust internal scheduler, idle-state aggregator, durable run engine, policy and budget enforcement, deterministic correction-draft producer, auto-apply gate, probation monitor, SQLite persistence, and Tauri commands.
- Web runtime: provides behaviorally equivalent status and policy mocks while running only page-active simulated orchestration and never claiming native background scheduling or filesystem Overlay commits.
- Frontend: extends service contracts and both adapters; React components remain isolated from Tauri invocation.
- Data: adds durable trigger receipts, run/checkpoint records, stage attempts, policy/consent, eligibility decisions, rate counters, circuit-breaker state, automatic application links, and probation observations.
- Dependencies: requires effective Skill runtime, Overlay governance, evidence pipeline, assessment, and Curator governance. It does not add an autonomous Skill-generation Agent, system-owned conversation, model-generated mutation, or automatic rollback.
- Logging and notifications: all diagnostics use unified redacted logging and the existing notification boundary; no feature-local log files are introduced.
