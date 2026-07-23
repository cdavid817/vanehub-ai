## ADDED Requirements

### Requirement: Coordination plan graph
The system SHALL represent a Multi-Agent coordination plan as task nodes with stable node ids, one primary stable Agent id, an ordered list of fallback stable Agent ids, an instruction, and prerequisite node ids.

#### Scenario: Accept a valid dependency graph
- **WHEN** a plan contains unique nodes whose prerequisite references form a directed acyclic graph
- **THEN** the system SHALL accept the plan and derive a deterministic topological execution order
- **AND** Agent references SHALL use stable Agent ids rather than display names

#### Scenario: Reject an invalid dependency graph
- **WHEN** a plan contains a duplicate node id, missing prerequisite, self-dependency, cycle, empty instruction, unknown Agent id, or duplicate primary/fallback Agent id
- **THEN** the system SHALL reject the complete plan before starting any Agent execution
- **AND** it SHALL return a command-safe validation error identifying the invalid node or relationship

### Requirement: Dependency-aware scheduling
The system SHALL start a coordination node only after every declared prerequisite node has succeeded.

#### Scenario: Execute prerequisites first
- **WHEN** node `review` depends on nodes `implement` and `test`
- **THEN** the scheduler SHALL start `review` only after both prerequisite nodes have succeeded
- **AND** simultaneously ready nodes SHALL be selected in deterministic node-id order in the first version

#### Scenario: Skip blocked dependents
- **WHEN** a node fails after exhausting its permitted Agents
- **THEN** every transitive dependent that can no longer satisfy its prerequisites SHALL become `skipped`
- **AND** independent ready branches SHALL remain eligible to run

### Requirement: Prerequisite output propagation
The system SHALL make every successful prerequisite node output available as bounded, provenance-preserving context for its dependent node.

#### Scenario: Inject prerequisite outputs
- **WHEN** a dependent node begins after two prerequisites succeed
- **THEN** its Agent input SHALL contain the prerequisite outputs in declared dependency order
- **AND** each context block SHALL identify the source node id, actual Agent id, and attempt number separately from the dependent instruction

#### Scenario: Enforce output and context bounds
- **WHEN** an Agent output or assembled prerequisite context exceeds its configured bound
- **THEN** the system SHALL record whether an individual output was truncated
- **AND** it SHALL reject an over-limit combined context as a non-retryable failure instead of silently omitting a prerequisite

### Requirement: Ordered Agent failover
The system SHALL attempt a node's primary Agent first and SHALL advance through declared fallback Agents in order only after retryable execution failures.

#### Scenario: Primary succeeds
- **WHEN** the primary Agent completes a node successfully
- **THEN** no fallback Agent SHALL start
- **AND** the node output SHALL identify the primary Agent as the actual executor

#### Scenario: Primary fails retryably
- **WHEN** the primary Agent is unavailable at execution time or returns a retryable process, provider, transport, or timeout failure
- **THEN** the system SHALL persist the failed attempt and start the first declared fallback Agent
- **AND** later fallbacks SHALL be attempted only if every earlier candidate fails retryably

#### Scenario: Failure is not retryable
- **WHEN** an attempt is cancelled or fails because of invalid input, policy rejection, context bounds, output validation, or persistence failure
- **THEN** the system SHALL NOT start a fallback Agent
- **AND** the node SHALL reach the corresponding cancelled or failed terminal state

#### Scenario: Fallback succeeds
- **WHEN** a fallback Agent succeeds after one or more retryable failures
- **THEN** the node SHALL succeed with the fallback output
- **AND** the node projection SHALL retain all attempt outcomes and identify the fallback as the actual Agent

### Requirement: Durable coordination lifecycle
The desktop runtime SHALL persist coordination plan snapshots, runs, node states, attempt history, bounded outputs, timestamps, and terminal errors through the Rust SQLite layer.

#### Scenario: Start coordination asynchronously
- **WHEN** the desktop runtime accepts a valid coordination request
- **THEN** it SHALL persist the run snapshot before Agent execution
- **AND** it SHALL return stable operation and run identities before variable-duration work completes

#### Scenario: Recover an interrupted run
- **WHEN** startup recovery finds a run or attempt left active without a live execution lease
- **THEN** the runtime SHALL terminate the orphan attempt with a runtime-interruption classification
- **AND** it SHALL deterministically resume with an unused fallback or fail and skip dependents according to the saved snapshot

### Requirement: Coordination query and cancellation boundary
The frontend SHALL start, list, read, and cancel coordination runs through a shared service interface implemented by Tauri and Web/mock adapters.

#### Scenario: Desktop queries a run
- **WHEN** React requests coordination state in the desktop runtime
- **THEN** it SHALL call the frontend service interface
- **AND** the Tauri adapter SHALL call declared Rust commands rather than React reading SQLite or invoking Agent processes directly

#### Scenario: Cancel an active run
- **WHEN** cancellation is requested for an active coordination run
- **THEN** the runtime SHALL stop the active node attempt, prevent new attempts from starting, and mark remaining non-terminal nodes cancelled or skipped consistently
- **AND** repeated cancellation SHALL be idempotent

#### Scenario: Web runtime parity
- **WHEN** the application uses the Web/mock adapter
- **THEN** it SHALL apply equivalent graph, propagation, failover, lifecycle, and cancellation contracts with deterministic simulated execution
- **AND** it SHALL NOT claim native process, SQLite, or unified-log side effects

### Requirement: Safe coordination diagnostics
The desktop runtime SHALL associate coordination runs, nodes, and failover attempts with unified operations, redacted logging, and execution observability without persisting raw coordination content in diagnostic channels.

#### Scenario: Record failover
- **WHEN** a primary attempt fails and a fallback starts
- **THEN** the runtime SHALL record redacted lifecycle diagnostics with run, node, attempt, stable Agent, and failure-classification correlation
- **AND** it SHALL keep raw instructions, propagated context, and Agent output out of logs and telemetry

#### Scenario: Preserve page-visible output
- **WHEN** a node succeeds
- **THEN** its bounded output SHALL remain queryable through the coordination run service projection
- **AND** diagnostic redaction SHALL NOT remove that output from the page-facing result store
