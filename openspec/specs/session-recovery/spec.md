# session-recovery Specification

## Purpose
Defines the durable safety state, generation correlation, evidence reconciliation, user review, and runtime-neutral contracts required to recover managed sessions after an unclean shutdown without guessing or replaying uncertain work.
## Requirements
### Requirement: Recovery safety is independent from session lifecycle
Every durable session SHALL expose a recovery status independently from its existing lifecycle state. Recovery status SHALL be one of `clean`, `reconciling`, `action_required`, or `quarantined`.

#### Scenario: Failed clean session accepts later work
- **WHEN** a session has lifecycle `failed`, recovery status `clean`, no active execution run, and is not archived
- **THEN** the session SHALL remain eligible to accept a new message

#### Scenario: Idle ambiguous session rejects work
- **WHEN** a session has lifecycle `idle` and recovery status `action_required`
- **THEN** every managed submission path SHALL reject new generation work for that session until an allowed recovery action succeeds

#### Scenario: Quarantined session remains inspectable
- **WHEN** recovery detects a stable structural inconsistency that cannot be reconciled without risking evidence loss
- **THEN** the session SHALL become `quarantined`, remain readable and exportable, and reject generation or mutation that depends on the inconsistent state

### Requirement: Managed generations use durable execution identity and ownership
Every accepted managed generation SHALL have one stable execution run identifier correlated with its session and persisted messages, and the session SHALL durably claim at most one active execution run before provider or CLI execution begins.

#### Scenario: Claim an available session
- **WHEN** a managed caller submits work to a non-archived, recovery-clean session with no active execution run
- **THEN** the runtime SHALL atomically claim that session for the new execution run before starting provider, CLI, or tool work

#### Scenario: Reject a competing claim
- **WHEN** another caller tries to start work while the session already has an active execution run
- **THEN** the durable claim SHALL reject the competing generation without starting it

#### Scenario: Preserve uncorrelated legacy history
- **WHEN** an upgraded database contains historical messages whose originating execution run cannot be proven
- **THEN** those messages SHALL remain readable with no fabricated execution run association

### Requirement: Startup recovery reconciles business evidence conservatively
Startup recovery SHALL reconcile session, message, operation, and available tool evidence belonging to the same execution run. It SHALL NOT choose an outcome from timestamps, display order, diagnostic logs, or observability records alone.

#### Scenario: Reconcile a confirmed completed message
- **WHEN** an orphan active session has one completed assistant message conclusively correlated with its active execution run and no conflicting business evidence
- **THEN** recovery SHALL clear the active run, set lifecycle to `idle`, and set recovery status to `clean`

#### Scenario: Reconcile a confirmed failed message
- **WHEN** an orphan active session has one failed assistant message conclusively correlated with its active execution run and no conflicting business evidence
- **THEN** recovery SHALL clear the active run, set lifecycle to `failed`, and set recovery status to `clean`

#### Scenario: Preserve a tool-free partial response
- **WHEN** the active run has only an unfinished assistant response, partial content is persisted, and no tool activity or conflicting terminal evidence is present
- **THEN** recovery SHALL preserve the partial content, mark the response interrupted or failed, clear the active run, and return the session to a recovery-clean terminal lifecycle

#### Scenario: Escalate uncertain tool activity
- **WHEN** recovery observes an unfinished tool activity whose effect cannot be conclusively determined from durable business evidence
- **THEN** it SHALL preserve the evidence, clear no uncertainty by assumption, and place the session in `action_required`

#### Scenario: Escalate conflicting execution evidence
- **WHEN** terminal facts from different execution runs or incompatible terminal outcomes claim the same active session work
- **THEN** recovery SHALL place the session in `action_required` or `quarantined` according to whether the evidence is ambiguous or structurally invalid

### Requirement: Recovery preserves evidence and records its decisions
Recovery SHALL preserve existing message content and provider resume metadata, SHALL never replay interrupted work automatically, and SHALL persist one immutable recovery report for each applied recovery revision.

#### Scenario: Record a recovery report
- **WHEN** recovery changes or confirms a session recovery projection
- **THEN** it SHALL persist the observed execution correlation, decision, safe reason codes, and bounded evidence references without copying prompts, message bodies, tool payloads, commands, credentials, or raw provider errors

#### Scenario: Provider resume metadata survives interruption
- **WHEN** a managed CLI or API session has persisted provider runtime resume metadata before an unclean shutdown
- **THEN** recovery SHALL preserve that metadata but SHALL NOT automatically resume or resend the interrupted generation

#### Scenario: Diagnostic telemetry is unavailable
- **WHEN** unified logs or execution observability records are missing, expired, disabled, or failed to persist
- **THEN** recovery SHALL derive its decision from durable business records and SHALL NOT invent the missing evidence

### Requirement: Ambiguous recovery requires explicit acknowledgement
The system SHALL offer an explicit acknowledgement action for an `action_required` session that retains all evidence and does not imply that an uncertain external effect did or did not occur.

#### Scenario: Acknowledge current evidence
- **WHEN** the user acknowledges an action-required recovery at the current recovery revision
- **THEN** the system SHALL record the acknowledgement, clear the recovery gate, retain the interrupted lifecycle and all prior evidence, and SHALL NOT retry the original generation

#### Scenario: Reject stale acknowledgement
- **WHEN** the submitted acknowledgement revision no longer matches the session recovery revision
- **THEN** the system SHALL reject the mutation and return the current recovery state for renewed review

### Requirement: Recovery is idempotent and concurrency-safe
Recovery SHALL use durable revisions and conditional transitions so repeated startup passes or concurrent state changes cannot duplicate reports, regress terminal states, or overwrite newer user activity.

#### Scenario: Run recovery twice
- **WHEN** startup reconciliation processes the same unchanged interrupted session more than once
- **THEN** the durable session projection and recovery reports SHALL remain equivalent to one successful pass

#### Scenario: Session changes during reconciliation
- **WHEN** the session revision changes after recovery reads evidence but before it publishes a decision
- **THEN** recovery SHALL abandon the stale decision and re-read or defer the session rather than overwrite the newer state

#### Scenario: Storage is temporarily busy
- **WHEN** recovery encounters temporary database contention or another retryable storage condition
- **THEN** it SHALL retain the prior durable state for retry and SHALL NOT classify the session as quarantined solely because of that condition

### Requirement: Recovery contracts are runtime-neutral and fidelity-aware
Recovery summaries, acknowledgement, sending gates, and typed invalidation events SHALL be exposed through shared frontend service contracts implemented by the desktop and Web/mock adapters. The native runtime SHALL report only the recovery fidelity available for the execution mode.

#### Scenario: Native API execution has managed evidence
- **WHEN** an API Agent generation runs through VaneHub-managed provider and tool boundaries
- **THEN** session recovery SHALL use the durable evidence available at those managed boundaries without making the capability specific to one Agent id

#### Scenario: CLI internal activity is opaque
- **WHEN** a managed CLI or interactive provider performs internal tool activity for which VaneHub has no conclusive durable outcome
- **THEN** recovery SHALL classify the incomplete activity conservatively and SHALL NOT fabricate a tool result

#### Scenario: Web adapter presents recovery behavior
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL expose deterministic compatible recovery states, gates, reports, acknowledgement, and events without claiming to recover native processes or SQLite state

### Requirement: Session recovery reconciles canonical Runs
Startup Session recovery SHALL reconcile its active execution claim with the canonical Run and owner recovery policy, SHALL clear false running state for non-resumable processes, and SHALL never replay destructive work.

#### Scenario: Orphan generation has no live handle
- **WHEN** startup finds an active Session claim and canonical Run without resumable runtime evidence
- **THEN** the Session and Run record an explicit interrupted outcome and no provider or tool action is replayed

### Requirement: User initiated recoverable session reconnect
The system SHALL expose an explicit recovery action for a failed or disconnected session when the runtime can safely attempt to resume it, including from the session context menu.

#### Scenario: Recover a failed session
- **WHEN** a user selects recovery for a recoverable failed session
- **THEN** the system attempts reconnection through the session service boundary
- **AND** reports the resulting running, failed, or unavailable state without discarding session history

#### Scenario: Recovery is unavailable
- **WHEN** a failed session cannot safely be resumed
- **THEN** the recovery action explains why it is unavailable
- **AND** the user retains access to session history and diagnostics

