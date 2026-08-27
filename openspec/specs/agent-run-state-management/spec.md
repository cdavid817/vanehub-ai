# agent-run-state-management Specification

## Purpose
TBD - created by archiving change unify-agent-run-state-machine. Update Purpose after archive.
## Requirements
### Requirement: Canonical Run identity and ownership
Every accepted long-running Agent, Plan, Loop, or delegated execution SHALL have one stable UUID Run id before execution begins, an owning type/id, optional typed business links, and an optional parent Run id. Existing business objects SHALL retain their own identities and semantics.

#### Scenario: Agent execution is accepted
- **WHEN** an Agent generation is accepted
- **THEN** the system creates a canonical Run linked to its Session, messages, operation, and execution-observability identity before provider work begins

#### Scenario: Child work is created
- **WHEN** a Plan attempt or delegated Agent task starts beneath an existing Run
- **THEN** it receives a distinct Run id with the existing Run as parent

### Requirement: Guarded canonical lifecycle
The canonical states SHALL be `created`, `preparing`, `running`, `waiting_approval`, `waiting_user`, `paused`, `retrying`, `blocked`, `stuck`, `verifying`, `completed`, `failed`, and `cancelled`. Every transition SHALL be accepted only by the domain transition table with an explicit trigger, timestamp, bounded reason code when required, and retry count when applicable. Completed, failed, and cancelled SHALL be terminal.

#### Scenario: Normal Agent path
- **WHEN** an API Agent prepares, runs, verifies, and succeeds
- **THEN** its Run transitions `created` to `preparing` to `running` to `verifying` to `completed`

#### Scenario: Illegal terminal reversal
- **WHEN** a caller attempts to move a completed Run to running
- **THEN** the domain rejects the transition without persisting an event

#### Scenario: Retry limit is reached
- **WHEN** another retry would exceed the Run retry policy
- **THEN** the transition is rejected or the Run enters failed with a bounded retry-exhausted reason according to the owner policy

### Requirement: Distinct waiting and verification semantics
Runs SHALL distinguish permission approval, user input, external dependency blocking, explicit pause, retry backoff, stuck detection, and verification. Waiting approval SHALL leave only through approval, rejection/failure, or cancellation; waiting user SHALL leave only through answer, cancellation, or restart interruption.

#### Scenario: Permission wait resumes
- **WHEN** a running tool requires approval and is approved
- **THEN** the Run transitions `running` to `waiting_approval` to `running`

#### Scenario: User question resumes
- **WHEN** a running Agent asks a valid interactive question and receives an answer
- **THEN** the Run transitions `running` to `waiting_user` to `running`

#### Scenario: Transient provider failure retries
- **WHEN** a provider failure is retryable within policy
- **THEN** the Run transitions `running` to `retrying` and then to `running` or `failed`

### Requirement: Safe canonical Run events
Every accepted state transition SHALL append exactly one ordered safe event chosen from `run_created`, `run_started`, `run_waiting`, `run_resumed`, `run_retrying`, `run_verifying`, `run_completed`, `run_failed`, `run_cancelled`, and `run_stuck`. Events SHALL contain only stable ids, bounded enum classifications, sequence/version, timestamp, and bounded reason codes.

#### Scenario: Event metadata is persisted
- **WHEN** a Run enters a waiting or terminal state
- **THEN** the event is durably ordered and contains no raw prompt, model output, tool payload, credential, unrestricted path, or unredacted error

#### Scenario: Duplicate terminal delivery
- **WHEN** the same witnessed terminal outcome is delivered more than once
- **THEN** the existing terminal result is returned without appending another event

### Requirement: Hierarchical cancellation and resume contract
Cancellation SHALL support user, parent, timeout, and shutdown sources, persist terminal intent atomically, propagate to every non-terminal child, signal owned cooperative cancellation, and reject late effects. Resume SHALL be allowed only when the state and owning runtime policy both permit it.

#### Scenario: User cancels active Run
- **WHEN** the user cancels a Run from an allowed non-terminal state
- **THEN** the Run and its owned work terminate as cancelled and late tool execution or completion is rejected

#### Scenario: Parent cancellation propagates
- **WHEN** a parent Run is cancelled
- **THEN** each non-terminal child is cancelled while already terminal children remain unchanged

#### Scenario: Cancellation races with completion
- **WHEN** cancellation and completion compete for the same Run version
- **THEN** exactly one terminal transition wins and the other is idempotently returned or rejected without duplicate effects

### Requirement: Conservative restart recovery
Startup recovery SHALL preserve terminal Runs, reconcile only non-terminal Runs through their owning runtime policy, mark non-resumable external work with an explicit interrupted outcome, invalidate ephemeral waits, and SHALL NOT automatically replay a provider request, tool call, approval, question, or destructive action.

#### Scenario: Non-resumable CLI Run is found after restart
- **WHEN** startup finds a running CLI Run without a verified durable resume handle
- **THEN** it no longer reports running and records a failed interrupted-restart outcome

#### Scenario: Recovery is repeated
- **WHEN** startup reconciliation runs more than once for the same persisted evidence
- **THEN** the Run remains in the same reconciled state without duplicate terminal events or actions

### Requirement: Additive persistence and compatibility
The desktop runtime SHALL persist Run snapshots and append-only events in additive transactional SQLite tables without rewriting existing Session, message, Plan, Loop, Goal, operation, or observability data. Existing commands and serialized fields SHALL remain compatible, and an older binary SHALL be able to ignore the new tables.

#### Scenario: Existing database upgrades
- **WHEN** a database containing legacy execution records is migrated
- **THEN** all legacy records remain readable and the new Run schema is available without destructive backfill

#### Scenario: Migration fails
- **WHEN** Run schema creation fails within the migration
- **THEN** the transaction rolls back without recording a partial migration version

### Requirement: Shared Run service and minimal status presentation
The frontend SHALL query and control Runs through the shared Agent service interface with contract-compatible Tauri and Web/mock adapters. A reusable localized status presentation SHALL show status, elapsed time, explicit waiting reason, retry count, and only permitted cancel/resume actions using semantic visual tokens. Elapsed time for an active Run SHALL advance from its canonical creation or start timestamp against the current clock and SHALL freeze against its terminal timestamp after completion.

#### Scenario: Desktop queries a Run
- **WHEN** React requests Run status in desktop mode
- **THEN** it uses the shared service and the Tauri adapter invokes declared native commands

#### Scenario: Web simulates a Run
- **WHEN** the same surface runs in Web/mock mode
- **THEN** it receives the same state, reason, timestamp, retry, and action contract without claims of native persistence or process recovery

#### Scenario: Status renders across supported layouts
- **WHEN** the status component renders in futuristic or minimal style at desktop or narrow width
- **THEN** status and actions remain readable, keyboard accessible, non-overlapping, and distinguishable without color alone

#### Scenario: Active elapsed time advances
- **WHEN** a Run remains in a non-terminal active state while its persisted update timestamp is unchanged
- **THEN** the visible elapsed duration SHALL continue increasing from the Run's canonical timestamp

#### Scenario: Terminal elapsed time freezes
- **WHEN** a Run reaches a terminal state
- **THEN** its visible elapsed duration SHALL be calculated against its terminal update timestamp and SHALL no longer increase

#### Scenario: Managed CLI completion survives restart
- **WHEN** a managed CLI generation persists a completed, failed, or cancelled terminal message and Operation
- **THEN** the correlated canonical Run SHALL persist the matching terminal outcome before the execution is treated as finished
- **AND** a later client restart SHALL preserve that terminal Run instead of replacing it with `interrupted_restart`

### Requirement: Bounded lifecycle performance
Run transitions SHALL use bounded validation and one atomic snapshot/event persistence boundary, and Run/event queries SHALL be indexed and paginated with bounded reason and event metadata.

#### Scenario: Large history is queried
- **WHEN** a Run has reached the supported event bound or many Runs share an owner
- **THEN** status and timeline queries use bounded pages and indexed access without scanning unrelated payload content

### Requirement: Mission Control Run projection and retry control
The shared Run service SHALL expose a bounded Mission Control projection over canonical Run state and a retry control that delegates eligibility and execution to the owning runtime. The projection MUST preserve canonical Run identity and terminal semantics and MUST NOT become a second lifecycle authority.

#### Scenario: Projection is queried
- **WHEN** Mission Control requests a filtered page
- **THEN** the shared service returns bounded summaries derived from canonical state using contract-compatible Tauri and Web/mock adapters

#### Scenario: Retry is accepted
- **WHEN** an eligible failed or stuck Run is retried
- **THEN** the owning runtime creates or transitions work according to its existing retry policy and returns the resulting canonical Run identity and state

#### Scenario: Retry is rejected
- **WHEN** state, owner policy, permission, or version does not allow retry
- **THEN** the service returns a safe typed rejection and does not alter the canonical Run

### Requirement: Run lifecycle performance is bounded and measurable
The canonical Run benchmark SHALL cover event propagation, valid state transition overhead, terminal idempotency, cancellation latency, token/progress event batching, and concurrent resource growth using the existing Run identity and lifecycle rules.

#### Scenario: One thousand Run histories are exercised
- **WHEN** the versioned 1,000-Run dataset applies deterministic lifecycle and cancellation sequences
- **THEN** transition work, retained events, query pages, and update batches SHALL remain within declared structural budgets
- **AND** illegal or duplicate terminal transitions SHALL retain their existing safe outcomes

#### Scenario: Concurrent runs are measured
- **WHEN** the dedicated benchmark increases supported concurrent Runs
- **THEN** it SHALL record throughput, cancellation latency, and resource growth with platform and build-profile provenance
- **AND** shared CI SHALL enforce only declared concurrency, buffer, and item-count bounds

### Requirement: Canonical Runs retain bounded Runner ownership
Every accepted Agent generation Run SHALL persist one immutable runner kind and stable bounded runner reference plus versioned capability, authority, recovery, and progress witnesses needed by its owner. Runner and progress metadata SHALL contain no credential, raw environment, prompt, unrestricted output, unrestricted path, or transport secret. Member progress projection SHALL identify the stable seat and bounded lifecycle milestone without creating a second Run lifecycle authority.

#### Scenario: Create a Local or SSH Run
- **WHEN** Agent generation is accepted for an eligible Runner
- **THEN** runner identity and recovery classification are committed with the canonical Run before it enters running

#### Scenario: Read an existing Run without runner metadata
- **WHEN** a legacy Run snapshot is loaded after migration
- **THEN** it remains readable and is conservatively projected as legacy Local only where existing ownership proves that classification
- **AND** no remote capability or live state is fabricated

#### Scenario: Project bounded member progress
- **WHEN** a child member Run starts, produces its first activity or output, waits, or terminates
- **THEN** the service SHALL expose its stable seat identity and bounded current milestone through the existing Run or stream boundary
- **AND** SHALL NOT expose secrets or unbounded raw process data in progress metadata

### Requirement: Runner-aware canonical cancellation and recovery
Canonical cancellation SHALL delegate owned process or channel termination through the Run owner's Runner handle, and startup recovery SHALL use runner inspection evidence before choosing reconnect, interrupted failure, or attention-required state. A Runner MUST NOT create a second Run lifecycle authority.

#### Scenario: Cancel an SSH Run
- **WHEN** canonical cancellation wins the Run version race
- **THEN** the owning remote process/channel receives cancellation and late Runner or provider completion cannot reverse the terminal state

#### Scenario: Restart with an unverifiable Runner
- **WHEN** a non-terminal Run's Runner cannot prove live ownership
- **THEN** canonical recovery stops presenting it as running and records one idempotent safe recovery outcome
