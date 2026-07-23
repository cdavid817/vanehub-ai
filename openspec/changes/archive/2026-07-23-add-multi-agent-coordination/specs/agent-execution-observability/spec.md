## MODIFIED Requirements

### Requirement: Correlated execution topology
The system SHALL represent task execution, prompt assembly, Agent invocation, managed child-process execution, stream milestones, tool activity, MCP activity, coordination nodes, failover attempts, and terminal outcome as one correlated trace when those stages occur.

#### Scenario: Agent CLI run completes
- **WHEN** a submitted task invokes a managed Agent CLI process and completes
- **THEN** the trace SHALL contain correlated Agent and process lifecycle spans or events with start, terminal status, and duration
- **AND** the run SHALL retain safe links to the existing operation and provider runtime-session identifiers when available

#### Scenario: Parallel or delegated work is observed
- **WHEN** the runtime observes delegated, retried, parallel, or child-Agent work
- **THEN** it SHALL preserve explicit parent-Agent, delegation, and attempt metadata when available
- **AND** it SHALL use parent spans or span links without reusing the original run or trace identity for an independent retry

#### Scenario: Coordination fallback is observed
- **WHEN** a coordination node advances from its primary Agent to a fallback Agent
- **THEN** both attempts SHALL remain correlated to the coordination run and node with distinct attempt spans or events
- **AND** telemetry SHALL identify bounded candidate role, stable Agent id, failure classification, and attempt number without capturing raw instructions, context, or output

### Requirement: Bounded observability metrics
The system SHALL record bounded metrics for task, Agent, process, tool, MCP, coordination node, failover, cancellation, failure, and first-output performance where the required boundaries are available.

#### Scenario: Metric dimensions are recorded
- **WHEN** observability metrics are emitted
- **THEN** their dimensions SHALL be limited to stable low-cardinality classifications such as Agent id, provider id, source, outcome, operation class, candidate role, failure classification, and fidelity
- **AND** they SHALL NOT include run, trace, span, session, message, operation, process, node, or tool-call identifiers
