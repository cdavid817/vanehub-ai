# agent-execution-observability Specification

## Purpose
Defines execution-run correlation, trace topology and fidelity, local timeline inspection, optional OTLP export, bounded metrics, and privacy requirements for Agent execution observability.
## Requirements
### Requirement: Execution run identity
The system SHALL create one execution run with independent run, trace, and root span identifiers for every accepted user-task submission before Agent execution begins.

#### Scenario: Desktop message creates a run
- **WHEN** the desktop runtime accepts a message for execution
- **THEN** it SHALL create an execution run linked to the source, session, user message, assistant message, operation, and stable Agent identifiers
- **AND** it SHALL NOT reuse the session id, message id, provider session id, or operation id as the trace id

#### Scenario: Non-desktop source creates a run
- **WHEN** an IM connector or scheduled task submits work through the shared native execution service
- **THEN** the system SHALL create the same execution-run contract with the corresponding source classification

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

### Requirement: Observation fidelity
Every observed Agent child operation SHALL report whether its telemetry is `native`, `proxied`, `inferred`, or `opaque`.

#### Scenario: Provider reports only tool start
- **WHEN** an Agent CLI reports a tool start without a matching terminal event
- **THEN** the tool observation SHALL be marked `inferred`
- **AND** it SHALL end as incomplete when the owning Agent process terminates rather than being reported as successful

#### Scenario: Details are unavailable
- **WHEN** VaneHub knows an execution stage exists but cannot observe its details
- **THEN** the timeline SHALL identify that stage as `opaque`
- **AND** it SHALL NOT invent child duration, result, error, or success data

### Requirement: Local execution timeline
The desktop runtime SHALL persist a bounded metadata timeline for execution runs in SQLite independently of external telemetry export.

#### Scenario: OTLP is disabled
- **WHEN** a task runs while OTLP export is disabled or unavailable
- **THEN** its redacted metadata timeline SHALL remain queryable locally through the native service boundary

#### Scenario: Trace retention expires
- **WHEN** execution timeline records exceed the configured retention period
- **THEN** scheduled maintenance SHALL delete the expired trace metadata without scanning or deleting records on every emitted event

### Requirement: Execution timeline service boundary
The frontend SHALL access execution run summaries and timelines through a shared service interface implemented by both Tauri and Web/mock adapters.

#### Scenario: Desktop requests run details
- **WHEN** React requests an execution timeline in the desktop runtime
- **THEN** it SHALL use the frontend service interface
- **AND** the Tauri-specific adapter SHALL obtain the data from a declared Rust command rather than React reading SQLite or log files

#### Scenario: Web requests run details
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL return a deterministic contract-compatible timeline
- **AND** it SHALL NOT claim native process, SQLite, or OTLP side effects

### Requirement: Optional non-blocking OTLP export
The desktop runtime SHALL export traces, metrics, and correlated logs over OTLP only when explicitly enabled, and exporter health SHALL NOT determine task success.

#### Scenario: Export succeeds
- **WHEN** OTLP export is enabled and the configured endpoint accepts telemetry
- **THEN** the runtime SHALL asynchronously export telemetry with the configured sampling and capture policy

#### Scenario: Export fails
- **WHEN** the exporter times out, rejects data, or becomes unavailable
- **THEN** the user task SHALL continue according to its Agent execution outcome
- **AND** the runtime SHALL emit a rate-limited redacted local diagnostic without recursively exporting through the failing path

### Requirement: Bounded observability metrics
The system SHALL record bounded metrics for task, Agent, process, tool, MCP, coordination node, failover, cancellation, failure, and first-output performance where the required boundaries are available.

#### Scenario: Metric dimensions are recorded
- **WHEN** observability metrics are emitted
- **THEN** their dimensions SHALL be limited to stable low-cardinality classifications such as Agent id, provider id, source, outcome, operation class, candidate role, failure classification, and fidelity
- **AND** they SHALL NOT include run, trace, span, session, message, operation, process, node, or tool-call identifiers

### Requirement: Metadata-only privacy default
Execution observability SHALL default to metadata-only capture and SHALL redact sensitive values before local persistence, unified logging, or OTLP export.

#### Scenario: Default task capture
- **WHEN** a task runs under the default capture policy
- **THEN** the timeline and exported telemetry SHALL omit raw prompts, model output, tool arguments and results, command-line prompt values, full user paths, headers, environment values, credentials, and MCP payload bodies

#### Scenario: Redacted content capture is enabled
- **WHEN** a user explicitly enables redacted content capture
- **THEN** only bounded redacted summaries allowed by the capture policy SHALL be persisted or exported
- **AND** raw content capture SHALL remain unavailable under this change

### Requirement: Semantic convention versioning
The native telemetry infrastructure SHALL emit a pinned OpenTelemetry semantic-convention schema version and SHALL isolate VaneHub-specific attributes under the `vanehub.*` namespace.

#### Scenario: Standard attribute exists
- **WHEN** the pinned OpenTelemetry GenAI or MCP convention defines a required concept
- **THEN** the exporter SHALL use the standard attribute and SHALL NOT duplicate it under a VaneHub-specific name

#### Scenario: Convention is not standardized
- **WHEN** a required product concept has no applicable pinned convention
- **THEN** the exporter SHALL use a documented `vanehub.*` attribute with bounded values
