## MODIFIED Requirements

### Requirement: Correlated execution topology
The system SHALL represent task execution, prompt assembly, Agent invocation, managed or interactive child-process execution, stream milestones, observable tool activity, observable MCP activity, and terminal outcome as one correlated trace when those stages occur.

#### Scenario: Agent CLI run completes
- **WHEN** a submitted task invokes a managed Agent CLI process and completes
- **THEN** the trace SHALL contain correlated Agent and process lifecycle spans or events with start, terminal status, and duration
- **AND** the run SHALL retain safe links to the existing operation and provider runtime-session identifiers when available

#### Scenario: Interactive Agent terminal process completes
- **WHEN** the desktop runtime starts an embedded interactive Agent terminal and its PTY process later exits or is stopped
- **THEN** the trace SHALL contain correlated Session, Agent, opaque Tool/MCP boundary, and Process Exec lifecycle spans with terminal status and duration
- **AND** the run SHALL retain safe session, stable Agent, and provider runtime-session identifiers when available
- **AND** it SHALL omit terminal content, full paths, raw command arguments, and environment values

#### Scenario: Interactive child details are unavailable
- **WHEN** an interactive terminal's TUI does not expose structured tool or MCP lifecycle data
- **THEN** the runtime SHALL emit only the known terminal Tool/MCP boundary with `opaque` fidelity and SHALL NOT invent concrete tool-call or MCP-operation child spans
- **AND** observation capability SHALL remain `opaque` or `inferred` according to the available evidence

#### Scenario: Parallel or delegated work is observed
- **WHEN** the runtime observes delegated, retried, parallel, or child-Agent work
- **THEN** it SHALL preserve explicit parent-Agent, delegation, and attempt metadata when available
- **AND** it SHALL use parent spans or span links without reusing the original run or trace identity for an independent retry

### Requirement: Bounded observability metrics
The system SHALL record bounded metrics for task, Agent, process, tool, MCP, cancellation, failure, and first-output performance where the required boundaries are available.

#### Scenario: Metric dimensions are recorded
- **WHEN** observability metrics are emitted
- **THEN** their dimensions SHALL be limited to stable low-cardinality classifications such as Agent id, provider id, source, outcome, operation class, failure classification, and fidelity
- **AND** they SHALL NOT include run, trace, span, session, message, operation, process, node, or tool-call identifiers

## REMOVED Requirements

### Requirement: Coordination fallback observation
**Reason**: This behavior lived as the "Coordination fallback is observed" scenario under correlated execution topology. With the `multi-agent-coordination` capability retired, no execution path advances a node from a primary to a fallback Agent, so there is nothing to correlate.

**Migration**: None. Delegated, retried, and parallel work remains observable through the retained "Parallel or delegated work is observed" scenario, which preserves parent-Agent, delegation, and attempt metadata.
