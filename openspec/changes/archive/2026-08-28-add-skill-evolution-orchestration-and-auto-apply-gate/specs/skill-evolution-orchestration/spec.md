## Purpose

Defines how VaneHub schedules, bounds, recovers, and exposes Skill-evolution runs and how an explicitly enabled deterministic gate may apply narrowly eligible learned guidance without bypassing Curator or Overlay governance.

## ADDED Requirements

### Requirement: Exactly ten orchestration trigger families
The system SHALL recognize exactly these ten trigger families: startup recovery, periodic maintenance, application-idle transition, Agent run completion, conversation completion, explicit feedback commit, verification completion, delegated Utility completion, relevant Skill/Overlay/policy change, and manual run request. Every trigger SHALL retain its family, workspace, source identity, timestamp, and safe reason metadata.

#### Scenario: Runtime outcome produces multiple triggers
- **WHEN** one Agent completion also closes a conversation and completes verification
- **THEN** the system records the applicable trigger families and may coalesce them into one workspace run request

#### Scenario: Unknown trigger is received
- **WHEN** a producer submits an unregistered trigger family or version
- **THEN** the system rejects it without scheduling work

### Requirement: Idempotent trigger receipts and coalescing
The system SHALL deduplicate triggers by source identity, debounce compatible workspace triggers, preserve per-family counters, and create at most one pending run request for a coalescing window.

#### Scenario: Trigger is delivered twice
- **WHEN** the same source event is replayed
- **THEN** the trigger counter and pending run are not duplicated

#### Scenario: Burst of compatible triggers
- **WHEN** multiple automatic triggers arrive during the debounce window
- **THEN** the pending run accumulates their counters and safe source references without creating parallel runs

### Requirement: Orchestration policy modes
Each workspace SHALL have an orchestration policy mode of `off`, `observe`, or `enabled`, defaulting to `off`. `observe` SHALL execute scheduling, pipeline, and auto-eligibility decisions without committing an automatic mutation; `enabled` SHALL permit only mutations that pass the complete auto-apply gate.

#### Scenario: Policy is off
- **WHEN** automatic triggers arrive while orchestration is off
- **THEN** the system records bounded trigger counters but does not start an automatic evolution run

#### Scenario: Policy is observe
- **WHEN** a candidate would pass automatic eligibility
- **THEN** the system records an observed-would-apply decision and performs no Overlay mutation

### Requirement: Explicit auto-apply consent and Skill allowlist
Enabling automatic application SHALL require versioned workspace consent to the disclosed behavior and a non-empty allowlist of stable Skill ids. Consent revocation or Skill removal SHALL take effect before the next mutation preflight.

#### Scenario: Workspace enables orchestration without allowlist
- **WHEN** the user selects enabled mode but has not allowed any Skill
- **THEN** runs may process pipeline work but no candidate is eligible for automatic mutation

#### Scenario: Consent is revoked during a run
- **WHEN** the user revokes auto-apply consent before commit
- **THEN** the final preflight fails and the candidate routes to Curator without mutation

### Requirement: Automatic-run idle gate
An automatically triggered run SHALL wait until no Agent is starting or generating, no managed CLI command or delegated Utility is active, no tool approval or verification is pending, no Skill/Overlay/Curator mutation is in progress, the application is not shutting down, and the configured user-idle interval has elapsed.

#### Scenario: Agent generation is active
- **WHEN** an automatic run is pending while any managed Agent is generating
- **THEN** the run remains waiting for idle and does not compete with the Agent

#### Scenario: Idle wait exceeds policy
- **WHEN** a request cannot pass the idle gate before its maximum wait
- **THEN** it checkpoints as deferred and a later compatible trigger may resume it without busy polling

### Requirement: Manual-run safety gate
A manual run request MAY bypass the user-idle interval but MUST NOT overlap active Skill/Overlay/Curator writes, shutdown, another workspace run, or any automatic mutation safety prerequisite.

#### Scenario: User requests run during active chat
- **WHEN** no conflicting mutation is active but an Agent is generating
- **THEN** the run may perform safe read-only maintenance and queues mutation-capable stages until quiescent

#### Scenario: User requests run during Overlay commit
- **WHEN** an Overlay transaction is active
- **THEN** the request waits or returns a stable busy state and does not bypass the writer lock

### Requirement: Durable run lifecycle
The system SHALL persist evolution runs with `requested`, `waiting_idle`, `running`, `partial`, `completed`, `failed`, `cancel_requested`, `cancelled`, or `recovered` status and SHALL preserve trigger summary, policy witness, budgets, current stage, checkpoints, counters, and timestamps.

#### Scenario: Run completes all eligible work
- **WHEN** every stage finishes within budget
- **THEN** the run becomes completed with per-stage outcomes and result counts

#### Scenario: Some work remains at budget exhaustion
- **WHEN** a run reaches a safe budget boundary with eligible backlog remaining
- **THEN** it becomes partial and stores a continuation checkpoint rather than marking the remaining work failed

### Requirement: Fixed staged run engine
Each run SHALL execute the ordered stages `recover`, `maintain_evidence`, `build_seeds`, `assess`, `route_governance`, `evaluate_auto_apply`, `project_results`, and `notify`. A stage MUST consume authoritative records through existing service boundaries and MUST NOT create a user-visible Agent session.

#### Scenario: Earlier stage has no work
- **WHEN** an ordered stage finds no eligible records
- **THEN** it records a skipped-empty outcome and the run continues

#### Scenario: Noncritical notification stage fails
- **WHEN** result notification fails after durable pipeline results commit
- **THEN** the run records partial notification failure without rolling back those results

### Requirement: Run budgets and continuation
The system SHALL enforce versioned limits for elapsed time, evidence items, seed builds, assessments, model calls, notifications, and mutations. Budget exhaustion SHALL stop at a transactional checkpoint and SHALL NOT discard completed stage results.

#### Scenario: Assessment budget is exhausted
- **WHEN** a run reaches its assessment limit
- **THEN** it stores the stable continuation cursor and leaves remaining assessments for a later run

#### Scenario: Mutation limit is reached
- **WHEN** one automatic mutation has committed in the run
- **THEN** every other eligible candidate routes or waits according to policy without another automatic commit

### Requirement: Workspace single-flight and global concurrency
The system SHALL allow at most one active evolution run per workspace and SHALL apply a bounded global concurrency policy across workspaces.

#### Scenario: Another trigger arrives during a run
- **WHEN** the same workspace already has an active run
- **THEN** the trigger is folded into a follow-up request and does not start a concurrent run

### Requirement: Crash recovery and idempotent stages
On startup, the system SHALL inspect nonterminal runs and stage receipts, reconcile authoritative subsystem results, and resume from the last safe checkpoint without duplicating signals, assessments, Curator candidates, notifications, or Overlay mutations.

#### Scenario: Process exits after automatic Overlay commit
- **WHEN** the application restarts before the run records the stage result
- **THEN** recovery discovers the Curator/Overlay application id and finalizes the run without applying again

#### Scenario: Process exits mid-read-only stage
- **WHEN** a stage has no committed receipt
- **THEN** recovery may safely rerun it using subsystem idempotency keys

### Requirement: Cooperative cancellation and graceful shutdown
Cancellation and application quit SHALL stop scheduling new stage work, allow current transactions to reach a bounded safe point, persist a checkpoint, and never interrupt an Overlay transaction into an unknown state.

#### Scenario: User cancels a run
- **WHEN** cancellation is requested during assessment
- **THEN** the run stops after the current bounded operation, persists its cursor, and performs no new automatic mutation

#### Scenario: Application quits during application saga
- **WHEN** graceful shutdown begins while an Overlay application is in progress
- **THEN** the existing saga recovery contract completes or records recoverable intent before process exit

### Requirement: Authorized correction guidance
The system SHALL treat corrected feedback as reusable Skill guidance only when the user explicitly grants a default-off authorization bound to the feedback revision. Revocation before application SHALL make any derived automatic draft ineligible.

#### Scenario: Correction has no reusable authorization
- **WHEN** corrected feedback produces a candidate without reusable-guidance authorization
- **THEN** it may enter normal assessment and Curator review but cannot produce an automatic draft

#### Scenario: User authorizes corrected guidance
- **WHEN** the user explicitly authorizes a bounded correction revision for reuse
- **THEN** the authorization witness may be considered by the deterministic draft producer

### Requirement: Deterministic correction draft producer
The system SHALL register exactly one automatic draft producer in this change: a sanitized `OverlayLearnBlock` derived from an authorized corrected-feedback revision whose structured trigger, behavior, and verification fields are complete. Identical input and producer versions MUST produce identical draft bytes and hash.

#### Scenario: Structured correction is complete
- **WHEN** an authorized correction has a bounded reusable guidance body and complete lesson shape
- **THEN** the producer emits one deterministic learned-guidance draft with source and authorization witnesses

#### Scenario: Producer would need invented prose
- **WHEN** evidence lacks the content needed for a complete learned-guidance block
- **THEN** no automatic draft is produced and the candidate remains available to Curator

### Requirement: Strict auto-apply eligibility
Automatic eligibility SHALL require all of the following: enabled policy and current consent; allowlisted stable Skill id; current deterministic `advance` assessment; clear verified target; system confidence at least 0.95; low risk; all nine quality checks passing; at least three independent supporting runs or an authorized verified correction plus two independent confirmations; current trusted deterministic draft; mutable unpinned target; healthy trusted Overlay chain; idle and rate capacity; and closed circuit breakers.

#### Scenario: Every eligibility condition passes
- **WHEN** a candidate and deterministic correction draft satisfy every current eligibility condition
- **THEN** the gate produces an eligible decision with complete witnesses for final preflight

#### Scenario: One condition fails
- **WHEN** any required condition is absent, stale, or false
- **THEN** the gate records the stable reason and performs no automatic mutation

### Requirement: Permanently excluded automatic mutations
The auto-apply gate MUST reject exact patches, files, scripts, tool definitions, commands, permission or side-effect expansion, model-generated drafts, user-authored Curator drafts, imported or untrusted drafts, ambiguous or model-resolved targets, correlated-only attribution, System-scope escalation, pinned or archived targets, and medium or high risk.

#### Scenario: Low-risk exact patch is proposed
- **WHEN** an exact patch otherwise appears to satisfy assessment thresholds
- **THEN** it remains manual Curator work and cannot auto-apply

#### Scenario: Model-assisted target is selected
- **WHEN** target selection depended on model consultation
- **THEN** the candidate is excluded from auto-apply regardless of reported confidence

### Requirement: Observe-mode eligibility audit
Observe mode SHALL run the same eligibility and final-preflight logic against immutable snapshots but SHALL stop before creating application intent or calling Overlay mutation.

#### Scenario: Observe decision would apply
- **WHEN** every eligibility and preflight condition passes in observe mode
- **THEN** the system records `would_apply` with the witnesses and no Overlay application id

### Requirement: Automatic application rate limits and cooldowns
The system SHALL permit at most one automatic mutation per run, three per workspace in any 24-hour window, and one per Skill in any seven-day window. Failed or rolled-back attempts SHALL remain represented in safety counters according to versioned policy.

#### Scenario: Workspace daily limit is reached
- **WHEN** another candidate is otherwise eligible within the same rolling window
- **THEN** it routes to Curator or waits and no automatic mutation occurs

### Requirement: Final mutation preflight
Immediately before automatic application, the system SHALL revalidate run policy, consent, authorization, allowlist, assessment, draft, target, effective revision, Overlay revision, trust, pin, quality, rate, idle, and circuit-breaker witnesses.

#### Scenario: Skill is pinned after eligibility
- **WHEN** pin state changes before application intent commits
- **THEN** preflight fails closed and the candidate routes to Curator

### Requirement: Curator and Overlay application path
An eligible automatic draft SHALL apply through the Curator application saga and Overlay mutation service with `system_policy` actor, policy authorization witness, idempotent application id, immutable audit, scanners, CAS, history, usage, and recovery. It SHALL NOT synthesize an interactive-user approval.

#### Scenario: Automatic learned guidance commits
- **WHEN** final preflight remains valid and the saga commits
- **THEN** the candidate, Curator audit, Overlay history, run result, counters, and probation record reference the same application id

#### Scenario: Application fails
- **WHEN** the saga cannot commit safely
- **THEN** the prior Overlay remains authoritative and the failure contributes to circuit-breaker policy

### Requirement: Automatic-application circuit breakers
A security, integrity, audit, or idempotency failure SHALL immediately open the workspace auto-apply circuit breaker. Two application failures within 24 hours SHALL also open it. An open breaker SHALL disable automatic mutation until explicit user acknowledgement after the underlying state is healthy.

#### Scenario: Integrity check fails
- **WHEN** Overlay history or Curator audit integrity cannot be verified
- **THEN** the breaker opens immediately and later runs remain read-only or Curator-routed

#### Scenario: User acknowledges unresolved breaker
- **WHEN** the underlying health check still fails
- **THEN** the system refuses to close the breaker

### Requirement: Seven-day probation monitoring
Every automatic application SHALL enter a seven-day probation linked to the target Skill, prior effective revision, new Overlay revision, evidence fingerprint, and application id. Structured verified outcomes SHALL be compared with the baseline without claiming causality from mere participation.

#### Scenario: Probation completes without regression
- **WHEN** the period ends with no verified compatible regression
- **THEN** probation becomes healthy and remains auditable

#### Scenario: Verified regression appears
- **WHEN** compatible verified negative outcomes exceed the versioned regression threshold
- **THEN** Skill auto-apply is suspended and a Curator rollback-review candidate is created

### Requirement: No automatic rollback
This change MUST NOT automatically revert an Overlay. Regression, anomaly, or probation failure SHALL route a witnessed rollback recommendation to Curator and notify the user.

#### Scenario: Severe regression is detected
- **WHEN** probation records a severe verified regression
- **THEN** the system opens the applicable breaker and creates urgent human review without executing revert

### Requirement: Sanitized run projection and notifications
The system SHALL expose and notify safe run, stage, eligibility, automatic application, probation, and breaker outcomes without raw prompts, correction bodies, terminal output, tool arguments, secrets, diffs, or provider payloads.

#### Scenario: Run contains sensitive evidence
- **WHEN** a run processes redacted candidate data
- **THEN** UI, unified logs, and notifications use identifiers, counts, categories, and sanitized reason codes only

### Requirement: Desktop and Web lifecycle separation
The desktop runtime MAY schedule idle and tray-background runs while the process is alive. The Web/mock runtime SHALL run only page-active simulated orchestration and SHALL NOT claim native background scheduling, durable process recovery, or real filesystem Overlay application.

#### Scenario: Desktop window is hidden to tray
- **WHEN** policy permits and the process becomes idle
- **THEN** maintenance may run subject to the same idle and safety gates

#### Scenario: Browser page is closed
- **WHEN** Web/mock orchestration was pending
- **THEN** it stops with the page and does not claim background continuation

### Requirement: Read-only orchestration queries
The system SHALL expose workspace- and Skill-scoped paginated queries for scheduler state, pending triggers, runs, stages, checkpoints, counters, policy, consent, allowlist, eligibility decisions, automatic applications, probation, and circuit breakers.

#### Scenario: Inspect partial run
- **WHEN** the user opens a budget-limited run
- **THEN** the system shows completed stages, remaining checkpoint, budget reason, and whether continuation is pending

