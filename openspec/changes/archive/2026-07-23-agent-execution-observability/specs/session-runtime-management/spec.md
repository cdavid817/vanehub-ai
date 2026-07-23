## ADDED Requirements

### Requirement: Shared message execution creates trace context
The shared native message execution service SHALL create and carry one execution context across preparation, Agent invocation, background monitoring, event handling, cancellation, and terminal persistence.

#### Scenario: Generation crosses a monitor thread
- **WHEN** an Agent generation starts a managed process and transfers monitoring to a background thread
- **THEN** the monitor and its emitted events SHALL retain the owning run, trace, Agent, session, message, and operation correlation

#### Scenario: Preparation fails before process start
- **WHEN** prompt assembly, CLI profile loading, or process construction fails before a child process starts
- **THEN** the run SHALL terminate with the corresponding failed stage and safe error classification

### Requirement: Managed Agent process telemetry
Managed Agent CLI execution SHALL record process spawn, start, output milestones, cancellation, exit, and monitoring failure within the owning Agent trace.

#### Scenario: Process starts successfully
- **WHEN** the runtime starts a managed Agent CLI child process
- **THEN** it SHALL record a safe executable classification, process identity, start timestamp, Agent id, and observation fidelity
- **AND** it SHALL NOT record the raw prompt argument or sensitive environment values

#### Scenario: Process exits unsuccessfully
- **WHEN** the child process exits with a non-success status or cannot be monitored
- **THEN** the process and Agent spans SHALL carry an error status and bounded error classification
- **AND** detailed redacted diagnostics SHALL remain available through unified logging

#### Scenario: Generation is cancelled
- **WHEN** a user, archive flow, delete flow, or owning runtime cancels generation
- **THEN** the trace SHALL record the cancellation initiator and terminal cancelled state
- **AND** cancellation of another session SHALL NOT mutate the run

### Requirement: Stream performance milestones
The runtime SHALL record first output and terminal stream milestones without persisting raw generated content under metadata-only capture.

#### Scenario: First visible output arrives
- **WHEN** the provider emits the first token, thinking block, tool event, rich block, or other visible output
- **THEN** the Agent execution SHALL record time to first output exactly once

#### Scenario: Stream completes without visible output
- **WHEN** the process completes successfully without a visible stream event
- **THEN** the run SHALL preserve that outcome without fabricating a first-output timestamp

### Requirement: Provider tool lifecycle normalization
Provider output adapters SHALL normalize tool lifecycle events by stable call id and SHALL preserve incomplete, duplicated, out-of-order, failed, and completed observations without inventing missing facts.

#### Scenario: Matching tool terminal event arrives
- **WHEN** a provider emits start and terminal events for the same stable tool-call id
- **THEN** the runtime SHALL update one correlated tool span with the terminal status and duration

#### Scenario: Duplicate tool event arrives
- **WHEN** a provider emits a duplicate lifecycle event for an already applied tool-call phase
- **THEN** normalization SHALL remain idempotent and SHALL NOT create a duplicate tool span

#### Scenario: Tool identity is unavailable
- **WHEN** a provider reports tool activity without a stable call id or name
- **THEN** the runtime SHALL preserve a bounded inferred observation when useful
- **AND** it SHALL NOT merge unrelated tool calls based only on display text

