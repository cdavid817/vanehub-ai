# runtime-performance-governance Specification

## Purpose
TBD - created by archiving change optimize-runtime-performance-foundation. Update Purpose after archive.
## Requirements
### Requirement: Optimized release configuration guard
The project MUST automatically verify that distributable native builds use the approved optimized Cargo release profile and do not enable debug assertions or full debug information.

#### Scenario: Validate native build configuration
- **WHEN** the native architecture contract tests run
- **THEN** they SHALL require optimization level 3, ThinLTO, one code generation unit, and debuginfo stripping
- **AND** they SHALL reject release-profile debug assertions or debug information

### Requirement: Frontend artifact performance budget
The frontend production build MUST enforce versioned JavaScript artifact budgets using the generated Vite manifest.

#### Scenario: Validate a production frontend build
- **WHEN** the frontend chunk validation runs after a production build
- **THEN** the main static JavaScript closure SHALL NOT exceed 350 KiB gzip
- **AND** no emitted JavaScript chunk SHALL exceed 700 KiB raw
- **AND** a failure SHALL identify the measured artifact and budget

### Requirement: Deterministic performance regression coverage
The project MUST verify performance-sensitive data structures and query paths with deterministic automated tests rather than relying only on shared-host timing assertions.

#### Scenario: Run project validation
- **WHEN** automated tests validate settings loading, historical search, or retained terminal buffering
- **THEN** they SHALL verify first-visit mounting, indexed bounded query behavior, and bounded incremental transcript storage respectively
- **AND** the tests SHALL NOT require a fixed wall-clock latency on shared CI hosts

### Requirement: Blocking workspace/git/log work moves off the Tauri main thread

A Tauri command whose call path runs blocking subprocess execution (git inspection), whole-log file reads, file-export dialogs, or directory walks SHALL be `async` and SHALL run that blocking work on the blocking thread pool via `spawn_blocking`, not on the Tauri main thread. The `spawn_blocking` call SHALL live in the runtime/api layer, not in the command adapter, so the command layer stays free of IO primitives.

#### Scenario: git inspection on a slow repository

- **WHEN** a workspace command runs `git status` or `git diff` against a repository that takes seconds to respond
- **THEN** the work SHALL run on the blocking pool and SHALL NOT freeze the UI

#### Scenario: A diff preflight for a single path

- **WHEN** a git diff command needs to decide whether one path is untracked
- **THEN** it SHALL use a single-path query rather than a full-directory `git status` walk followed by a second git spawn

### Requirement: Repository reads batch instead of per-row on hot paths

A repository method that loads detail for a list of parent entities (workspaces, runs, source ids, documents being reconciled) SHALL load them in one query (or one transaction for writes) rather than one round-trip per item. A per-entity fallback MAY remain for single-entity lookups. Batched results SHALL be asserted equal to per-entity results in tests.

#### Scenario: A list endpoint loads status for every workspace

- **WHEN** a command lists code-index workspaces with their status
- **THEN** the repository SHALL run one aggregated query with correlated counts and a window function for each workspace's latest failure, not one status query per workspace

#### Scenario: A reconcile diff applies many upserts and deletes

- **WHEN** a reconciliation pass produces a set of changed documents and orphaned source ids
- **THEN** the repository SHALL apply the whole diff inside one transaction with prepared statements, not one autocommit per row

### Requirement: Chat stream events apply in batched traversals

The frontend SHALL coalesce high-frequency stream events (token, thinking) and apply them as a batch in a single message-array traversal per animation frame, rather than rebuilding the array once per event. Terminal events (completed/failed/cancelled) SHALL flush immediately so the stop indicator is not delayed.

#### Scenario: A turn emits thousands of token events

- **WHEN** an agent turn emits a burst of token events
- **THEN** the message array SHALL be rebuilt at most once per animation frame, not once per token
- **AND** messages that no event touched SHALL keep their reference identity so memoized children skip re-rendering

### Requirement: Versioned runtime performance harness
The repository SHALL provide a repeatable performance command driven by versioned deterministic dataset manifests. Each result SHALL identify the source commit, operating system and architecture, build profile, dataset id and version, metric id, metric class, measured value, unit, baseline, budget, and outcome.

#### Scenario: Run the deterministic performance suite
- **WHEN** the repository performance command runs twice from the same source and dataset version
- **THEN** it SHALL select the same fixtures and deterministic structural budgets
- **AND** it SHALL emit parseable result records with the required provenance

#### Scenario: Reject malformed or unsafe fixture metadata
- **WHEN** a dataset or result omits required provenance, uses an unknown metric class or unit, duplicates an id, exceeds declared fixture bounds, or references a path outside its fixture root
- **THEN** the harness SHALL fail before executing the affected workload and identify the safe validation reason

### Requirement: Performance metrics use stable gate classes
Every runtime performance metric SHALL be classified as `deterministic-gate`, `dedicated-benchmark`, or `informational-telemetry`. Shared CI SHALL enforce deterministic structural budgets and SHALL NOT enforce dedicated or informational wall-clock, throughput, CPU, or memory measurements as fixed absolute timing gates.

#### Scenario: Shared CI evaluates mixed metrics
- **WHEN** deterministic, dedicated, and informational records are compared
- **THEN** only a deterministic over-budget result SHALL fail the shared-CI command
- **AND** all classes SHALL remain present in the evidence report

### Requirement: Budgets derive from recorded baselines
A hard or relative budget SHALL cite a measured baseline and justified headroom. The comparator SHALL report metric id, baseline, measured value, budget, delta, dataset, platform, and profile for every regression.

#### Scenario: Metric exceeds its budget
- **WHEN** a deterministic measurement is greater than its declared upper bound or lower than its declared lower bound
- **THEN** comparison SHALL fail with the complete actionable metric context

#### Scenario: Negative regression fixture is evaluated
- **WHEN** the repository's known-over-budget fixture is compared
- **THEN** the comparator SHALL reject it deterministically without changing the accepted baseline

### Requirement: Runtime surfaces retain dedicated and informational evidence
The harness SHALL support dedicated evidence for latency, throughput, and memory and informational evidence for cold start, time to interactive, idle memory, idle CPU, and main-thread long tasks without exposing raw prompts, responses, credentials, terminal content, file content, or unrestricted paths.

#### Scenario: Device evidence is recorded
- **WHEN** a supported desktop measurement is captured
- **THEN** the record SHALL contain bounded numeric metrics and environment provenance only
- **AND** another operating system SHALL remain `NOT RUN` unless it was actually measured

### Requirement: Agent Runner resource budgets are deterministic and bounded
The runtime performance harness SHALL measure Local and fake SSH Runner admission, event buffering, concurrent Run registry growth, pooled transport reuse, cancellation, disconnect/reconnect attempts, and cleanup using versioned fixtures and structural budgets. Shared CI MUST enforce declared counts and capacities rather than fixed wall-clock latency.

#### Scenario: Concurrent Runner fixture executes
- **WHEN** the versioned fixture increases Local and SSH Runs to the supported concurrency limit
- **THEN** active handles, threads or tasks, channels, pooled transports, queued events, retained bytes, reconnect attempts, and cleanup records remain within declared budgets

#### Scenario: Resource regression fixture exceeds one bound
- **WHEN** the negative fixture leaks a handle, grows an unbounded event queue, or establishes one SSH transport per compatible Run
- **THEN** the deterministic comparator fails with metric, dataset, baseline, measured value, and budget

### Requirement: Hot-path relationship reads are batched
Repository list operations MUST batch child relationship reads instead of issuing a fixed set of additional queries for every parent item.

#### Scenario: Load Agent registry
- **WHEN** the registry lists Agents with modes and capability tags
- **THEN** the repository SHALL load Agent rows and their relationships with a bounded number of queries independent of Agent count

#### Scenario: Load feedback for a message page
- **WHEN** a message page requests feedback summaries
- **THEN** the evidence repository SHALL use a bounded number of queries independent of message count

### Requirement: Shared registry locks exclude nested waits
An asynchronous registry lock SHALL be released before awaiting independently locked child state.

#### Scenario: Read connector health during replacement
- **WHEN** health is collected while a connector is being replaced or stopped
- **THEN** health collection SHALL NOT retain the connector registry read lock while awaiting connector state

### Requirement: Shell lifecycle resource use and cleanup latency are explicitly bounded

The runtime SHALL govern Shell capacity reservations, startup event buffering, command-path close duration, Reaper queue depth, Reaper concurrency, automatic retry count, and per-attempt deadlines through explicit finite limits. Structural counters and deterministic fakes SHALL verify those limits without relying on absolute shared-CI timing.

#### Scenario: Reaper queue reaches capacity

- **WHEN** another unconfirmed Shell cleanup is offered after the bounded Reaper queue is full
- **THEN** the system SHALL retain the Shell and runtime ownership in a typed `CloseFailed` state
- **AND** it SHALL reject or defer the new Reaper handoff without dropping handles or starting an unbounded task/thread

#### Scenario: Close worker never reports completion

- **WHEN** a deterministic fake worker never signals completion
- **THEN** the close command SHALL return by its injected deadline
- **AND** the test SHALL observe a bounded number of termination/reap checks and no blocking join

#### Scenario: Startup event burst exceeds its bounded gate

- **WHEN** output generated during `Opening` exceeds the configured startup event buffer or gate capacity
- **THEN** the Shell SHALL enter a typed failure/cleanup path
- **AND** the system SHALL not silently discard output while reporting successful startup

### Requirement: Workspace inspection concurrency and retained work are structurally bounded

The runtime SHALL enforce finite global and per-workspace active inspection limits, a finite admission wait/queue policy, finite operation budgets, and bounded retained candidate/page state before starting blocking or remote work. Admission ownership SHALL continue until the actual worker exits, including after caller cancellation.

#### Scenario: Repeated searches supersede faster than workers exit

- **WHEN** a view repeatedly starts the same search id while earlier blocking workers are still stopping
- **THEN** each newer generation SHALL cancel the prior generation
- **AND** active workers SHALL never exceed the configured global/per-workspace admission limits
- **AND** requests beyond finite admission capacity SHALL receive a typed busy result rather than creating an unbounded queue

#### Scenario: Structural memory gate for directory pagination

- **WHEN** an instrumented directory provider exposes a number of entries far above the requested page size
- **THEN** the test SHALL observe retained page candidates bounded by page size plus fixed overhead
- **AND** the assertion SHALL not depend on absolute process memory or shared-CI latency

#### Scenario: Cancellation checkpoint gate

- **WHEN** cancellation is signalled to an instrumented traversal or file reader
- **THEN** the worker SHALL stop within the configured maximum number of additional entry/chunk checkpoints
- **AND** the test SHALL not require a platform-specific millisecond threshold to prove correctness

