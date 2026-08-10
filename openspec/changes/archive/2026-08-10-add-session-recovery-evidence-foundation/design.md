## Context

See `proposal.md` for motivation. The current session runtime owns active provider/CLI handles in process memory, persists session lifecycle and assistant message status separately, and performs startup recovery from a background maintenance path by converting orphan active state to failure. Message order is timestamp-based, `append_message_tool_use` stores lifecycle snapshots inside the assistant message, pending approvals and generation handles are process-local, and Plan and Loop have their own recovery projections.

The database is an app-owned SQLite file accessed through a bounded connection pool with WAL, foreign keys, and a busy timeout. Execution observability already creates a stable execution run for accepted work, but observability data is best-effort and retention-bounded. The existing `session:event` transport is also best-effort and currently acts as an invalidation notification for active-session and configuration changes.

API Agents and managed CLI chat can share session-level recovery semantics, but their evidence fidelity differs. VaneHub controls the API request and native API tool loop; a CLI or interactive provider can perform internal work that VaneHub cannot fence or prove. This foundation must represent that difference instead of making an Agent-id-specific promise.

## Goals / Non-Goals

**Goals:**

- Establish durable, database-enforced ownership for at most one active execution run per session.
- Separate runtime lifecycle from recovery safety and use one sending gate for desktop, headless native, Plan, Loop, multi-seat, and Web/mock callers.
- Give every new message a deterministic session sequence and correlate new managed generation messages with the existing execution run identity.
- Reconcile only business evidence belonging to the same run, retain partial content and resume metadata, and surface ambiguity explicitly.
- Make recovery publication conditional, idempotent, restartable, privacy-safe, and observable through service-backed state.
- Preserve compatibility with historical sessions without inventing missing execution identities.

**Non-Goals:**

- Proving, replaying, or undoing tool effects; durable tool intent belongs to a later change.
- Repairing provider-specific tool JSON or synthesizing provider protocol results.
- Resuming an interrupted provider request or child process automatically.
- Context budgeting, compaction, request reduction, checkpoints, forks, or in-place rollback.
- Treating logs, traces, spans, or LLM output as recovery authority.
- Guaranteeing recovery after physical media loss or corruption outside SQLite's committed durability contract.

## Decisions

### 1. Recovery safety is a second state axis

`SessionLifecycle` remains `idle`, `starting`, `running`, `failed`, or `stopped`. A new `SessionRecoveryStatus` is `clean`, `reconciling`, `action_required`, or `quarantined`.

The normalized sending predicate is:

```text
not archived
AND recovery_status == clean
AND active_execution_run_id is null
```

Lifecycle is intentionally absent from the predicate: a clean failed or stopped session can accept a new turn, while an idle action-required session cannot. `quarantined` is reserved for stable structural inconsistency; provider failure, missing configuration, filesystem unavailability, logging failure, event loss, SQLite contention, and other operational failures do not qualify by themselves.

Alternative considered: add recovery-related lifecycle variants. Rejected because it would combine the outcome of the last generation with whether persisted evidence is safe to continue from, producing ambiguous combinations and extensive provider-specific branching.

### 2. Reuse `ExecutionRunId` as generation identity without depending on telemetry retention

The application allocates the existing execution run identifier before the generation-start transaction and persists the opaque value on the session claim and its user/assistant messages. No foreign key points from business tables to observability storage because the observability row may be absent or removed by retention.

The identifier is reused for correlation, but the authoritative facts remain session, message, operation, and later tool-journal records. Creating or exporting telemetry remains best-effort and cannot fail or alter a recovery decision.

Alternative considered: introduce `GenerationAttemptId`. Rejected because it would duplicate an existing one-per-accepted-task identity and require permanent mappings between two identifiers with the same scope.

### 3. Add deterministic sequence allocation and three recovery-facing counters

The sessions table gains:

```text
recovery_status              TEXT NOT NULL DEFAULT 'clean'
recovery_revision            INTEGER NOT NULL DEFAULT 0
state_revision               INTEGER NOT NULL DEFAULT 0
history_revision             INTEGER NOT NULL DEFAULT 0
active_execution_run_id      TEXT NULL
next_message_sequence        INTEGER NOT NULL DEFAULT 1
```

Messages gain:

```text
session_sequence             INTEGER NOT NULL
execution_run_id             TEXT NULL
```

The uniqueness invariant is `(session_id, session_sequence)`. A transaction reserves one or more consecutive sequence values by conditionally advancing `next_message_sequence`; it does not use an unlocked `MAX(sequence) + 1` query.

`state_revision` changes for lifecycle, recovery, active ownership, archive/pin, runtime resume metadata, and configuration facts. `history_revision` changes for message insertion, persisted streaming updates, terminal transitions, and recovery annotations. `recovery_revision` changes only when a recovery decision or acknowledgement is published and is the user-confirmation compare token.

Alternative considered: keep ordering by `created_at, id`. Rejected because callers would still need timestamp-aware composite cursors and later compaction/fork boundaries would lack a stable ordinal.

### 4. Generation start and terminal publication are explicit transactions

The generation-start transaction:

1. Checks archived, recovery status, and absence of an active run.
2. Claims `active_execution_run_id` with compare-and-set semantics.
3. Allocates message sequences.
4. Inserts or correlates the accepted user message and assistant placeholder with the same run.
5. Advances session revisions and moves lifecycle to `starting`.

No provider request, process spawn, approval wait, or tool action occurs before commit.

The generation-terminal transaction:

1. Conditionally transitions the correlated assistant message from an unfinished status to exactly one terminal status.
2. Persists normalized usage when available under the existing message uniqueness rule.
3. Updates lifecycle to the matching terminal projection.
4. Clears `active_execution_run_id` only when it still equals the terminating run.
5. Advances state/history revisions.

Process-local generation coordination remains useful for cancellation handles, but its reservation is no longer the ownership authority. Competing application instances therefore race on the database claim rather than starting duplicate work.

Alternative considered: retain separate repository saves and repair discrepancies later. Rejected because ordinary successful execution would continue creating avoidable crash windows that recovery then has to guess through.

### 5. Terminal evidence is a typed read model, not a generic precedence list

A `SessionTerminalEvidencePort` returns a consistent evidence snapshot for one candidate session and, when known, one execution run. It composes:

- session lifecycle, recovery, active-run, and revisions;
- ordered messages and their run associations;
- durable operation terminal facts;
- the presence and terminal completeness of available tool activity;
- provider resume metadata as continuity metadata, not an outcome;
- whether a live runtime handle still exists after adapters are attached.

Logs and observability may be linked for inspection but do not participate in the decision. Evidence for different runs is never merged because it happens to be newer. Matching terminal facts can corroborate one another; incompatible terminal facts produce ambiguity rather than a winner.

Initial decision classes are:

```text
Completed
Failed
Cancelled
InterruptedWithoutToolAmbiguity
ActionRequired(reason codes)
Quarantined(reason codes)
RetryLater(storage/runtime readiness)
```

An unfinished historical tool-use snapshot without a durable outcome is action-required. This deliberately errs toward review until the separate tool-journal capability exists.

Alternative considered: prefer assistant status, operation status, or latest timestamp in a fixed order. Rejected because each source can be the one write that happened immediately before a crash; precedence would convert a write-order accident into a false fact.

### 6. Recovery uses claim, read, and conditional publish phases

For each candidate, startup recovery:

1. Selects sessions whose lifecycle/active claim/recovery state requires inspection in bounded batches.
2. Conditionally claims the candidate by moving it to `reconciling` and capturing revisions.
3. Reads one consistent evidence snapshot without holding a write transaction across provider, filesystem, or logging work.
4. Computes a deterministic decision locally.
5. Publishes the projection, active-run clearing, unfinished-message terminal update where allowed, and immutable recovery report in one short transaction conditioned on the captured revisions and run identity.
6. Emits a typed invalidation event after commit and writes safe unified diagnostics independently.

If revisions changed, publication is abandoned and the candidate is re-read or deferred. Temporary `SQLITE_BUSY`, connection-pool timeout, or storage unavailability results in `RetryLater`, not quarantine. Re-running the coordinator over an already-published revision produces no duplicate report or annotation.

Alternative considered: hold one transaction through all recovery work. Rejected because it would block the single SQLite writer, grow contention during startup, and make adapter or logging latency part of database availability.

### 7. Recovery reports are immutable safe metadata

`session_recovery_reports` stores:

```text
report_id
session_id
recovery_revision
trigger
observed_lifecycle
observed_execution_run_id
decision
reason_codes_json
evidence_refs_json
created_at
```

The unique key `(session_id, recovery_revision)` makes publication idempotent. Evidence references are bounded identifiers and statuses, not copied content. Reports remain owned by the session and are deleted with it under existing privacy semantics.

Alternative considered: store only the current decision on `sessions`. Rejected because acknowledgement and later recovery passes would erase the reason the session had been blocked and make failure analysis depend on retention-bounded logs.

### 8. Acknowledgement releases the gate but does not resolve an external fact

`acknowledgeSessionRecovery(sessionId, expectedRecoveryRevision)` is allowed only for `action_required`. A conditional transaction records a new report/revision, clears recovery to `clean`, clears any stale active claim, and leaves the interrupted lifecycle and all messages unchanged. It does not retry, delete, rewrite tool snapshots, or state that an uncertain effect did not occur.

`quarantined` has no generic acknowledgement because its invariant violation must be handled by a future targeted repair/export workflow.

Alternative considered: convert action-required directly to idle and continue automatically. Rejected because it would hide that the prior run was interrupted and could make provider resume or user interpretation treat partial output as a completed answer.

### 9. Startup recovery is separated from recurring maintenance

Bootstrap ordering becomes:

```text
database migrations and registry seed
→ unified logging
→ sessions, operations, runtime, and evidence adapters
→ attach real runtime/process adapters
→ ordinary session recovery
→ Plan recovery projection
→ Loop recovery projection
→ session invalidation events
→ recurring archival and retention jobs
```

Recovery is one-shot per bootstrap plus explicit retry for retryable candidates. Automatic archival remains hourly and cannot archive sessions being reconciled, action-required, quarantined, or actively claimed.

Alternative considered: keep recovery inside the current hourly maintenance function. Rejected because archival has different dependencies and retry semantics, and because recovery must complete before Plan/Loop infer their parent state.

### 10. Plan, Loop, and serial seat handoffs consume shared session evidence

Plan and Loop retain their own durable run/attempt state machines but stop scanning recent messages to derive a child session outcome. They receive a bounded session recovery projection keyed by session and execution run. A conclusive result is projected once; an ambiguous or quarantined result becomes their existing recovery-required/paused behavior. Retry always creates a new attempt/session/run.

Multi-seat handoffs are already serial. Each seat generation receives its own execution run, optionally linked by a seat-round and parent-run correlation, so recovery interrupts only the active seat and retains earlier terminal replies.

Alternative considered: give each orchestrator its own recovery engine. Rejected because the same session/message facts could receive different outcomes and bootstrap ordering would decide which engine overwrote the others.

### 11. Events invalidate cached state; they do not carry authority

The existing typed `session:event` union gains recovery-started, recovery-completed, recovery-action-required, recovery-quarantined, and recovery-acknowledged variants with session id and revision. The Tauri adapter validates the payload, and the Web/mock adapter emits deterministic equivalents.

React loads authoritative session/recovery state on initialization and refreshes it after an event. Missing an event, subscribing after startup recovery, or observing a revision gap triggers a service refresh; it never leaves the event payload as the only copy of state.

Alternative considered: introduce a new recovery event channel or replayable frontend event log. Rejected because the existing session subscription already supplies invalidations and durable state is available through the service boundary.

### 12. Recovery durability and failure injection use the real SQLite boundary

The database initialization explicitly verifies the synchronous durability level selected for committed recovery-critical transactions while retaining WAL and bounded connection waits. This change targets process crashes and unclean shutdowns once SQLite reports a commit; physical media loss and database-file destruction remain outside the capability.

Tests use file-backed temporary databases, deterministic failpoints after recovery-critical transitions, disposal of all process-local services, database reopen, and construction of a new startup coordinator. In-memory repository tests remain useful for domain decisions but do not count as crash-recovery proof.

Alternative considered: test only injected repository errors without reopening. Rejected because such tests cannot expose reliance on in-memory handles, transaction boundaries, WAL visibility, or migration state.

## Risks / Trade-offs

- **[Legacy active sessions lack run or tool evidence]** → Backfill ordering only; never fabricate identity. Recover tool-free partial sessions conservatively and send tool-bearing ambiguity to explicit review.
- **[Action-required may be more common before the tool journal exists]** → Use specific, localized reason codes and keep acknowledgement simple while retaining evidence.
- **[Durable claims add SQLite writer contention]** → Keep transactions short, process recovery in bounded batches, batch streaming persistence where safe, and treat contention as retryable.
- **[Explicit synchronous durability can increase write latency]** → Benchmark streaming and terminal workloads; batch token deltas while keeping claims and terminal publications immediate.
- **[CLI tools remain opaque]** → Report actual fidelity and never synthesize a tool result. Provider resume metadata is retained only for a later explicit invocation.
- **[Best-effort events can be missed]** → Make every frontend surface load and refresh durable state through the shared service contract using revisions.
- **[Observability run records may expire]** → Persist the opaque run identifier on business records without a foreign key and never require the observability row for recovery.
- **[Plan/Loop integration can create startup cycles]** → Enforce the single bootstrap order: ordinary sessions first, then dependent projections, then recurring maintenance.
- **[Terminal-message immutability conflicts with recovery of unfinished messages]** → Recovery may transition only pending/streaming messages to a terminal status; once terminal, content and status never regress.

## Migration Plan

1. Add recovery/session revision columns, the session sequence allocator, nullable message execution correlation, message sequence, recovery reports, constraints, and indexes using the next migration numbers available on the then-current main branch.
2. Deterministically backfill message sequence per session by `created_at ASC, id ASC`, set each session's next sequence, verify uniqueness, and leave historical execution-run correlation null.
3. Introduce typed domain values and repository methods while preserving current read behavior; add migration fixtures for clean, same-timestamp, active, CLI-resume, Plan-owned, and Loop-owned sessions.
4. Change new generation start and terminal paths to use the durable transactions and dual-present existing lifecycle/message fields so older UI behavior remains available.
5. Add evidence collection, pure decision tests, immutable reports, and the new coordinator behind startup composition; run it only after evidence adapters are attached.
6. Replace the old orphan mutation and split recurring archival from startup reconciliation.
7. Integrate Plan, Loop, multi-seat correlation, typed events, frontend service methods, Tauri adapter, Web/mock adapter, recovery UI, and acknowledgement.
8. Run file-backed crash/failpoint tests and the full project verification suite before enabling the new sending gate as the sole admission path.

The migrations are additive and preserve existing rows. Rolling application code back after migration can still read the pre-existing columns, but continuing to write with an older application is not a supported safety guarantee because it will not allocate sequence, run correlation, or revisions. If deployment must be rolled back during development, restore the pre-migration database copy rather than mixing writers from both schemas.
