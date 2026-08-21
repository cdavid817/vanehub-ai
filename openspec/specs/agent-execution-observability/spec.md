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
The system SHALL record bounded metrics for task, Agent, process, tool, MCP, cancellation, failure, and first-output performance where the required boundaries are available.

#### Scenario: Metric dimensions are recorded
- **WHEN** observability metrics are emitted
- **THEN** their dimensions SHALL be limited to stable low-cardinality classifications such as Agent id, provider id, source, outcome, operation class, failure classification, and fidelity
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

### Requirement: Evidence-safe execution projection
The native observability boundary SHALL project registered execution outcomes into a bounded versioned evidence source envelope after applying existing metadata privacy rules. Evidence projection SHALL be local, asynchronous, non-blocking, and independent of optional OTLP export.

#### Scenario: Native run projects Skill revisions
- **WHEN** a native API run reaches a registered tool, verification, delegation, or terminal outcome
- **THEN** the projection SHALL include safe correlation, fidelity, status, counts, and exact effective Skill revision associations observed by that run

#### Scenario: CLI projection preserves fidelity
- **WHEN** a managed or interactive CLI run emits a registered observable outcome
- **THEN** the projection SHALL preserve native, proxied, inferred, or opaque fidelity plus only the binding and mount facts actually captured for that run

#### Scenario: OTLP disabled
- **WHEN** optional OTLP export is disabled
- **THEN** local evidence projection MAY continue according to local evidence policy

#### Scenario: Evidence projection fails
- **WHEN** projection or enqueue fails
- **THEN** the execution run and its observability timeline SHALL continue normally and a rate-limited redacted diagnostic MAY be emitted

### Requirement: Observed Skill revision set
Execution metadata SHALL record the bounded set of canonical Skill revisions actually injected, successfully loaded, delegated, or actively mounted for each eligible run stage, with association kind and observation time.

#### Scenario: Eager Skill recorded
- **WHEN** an eager Skill is included in the final native API prompt
- **THEN** observability SHALL record its canonical id, effective revision hash, and `injected` association for that generation

#### Scenario: On-demand Skill recorded
- **WHEN** `load_skill` returns effective instructions successfully
- **THEN** observability SHALL record the canonical id, effective revision hash, and `loaded` association for that generation

#### Scenario: Utility recorded
- **WHEN** a delegated Utility child begins
- **THEN** observability SHALL record its canonical id, effective revision hash, and `delegated` association on parent and child topology

#### Scenario: CLI configured but not mounted
- **WHEN** a CLI Skill binding exists but no active mount snapshot was captured
- **THEN** observability SHALL NOT label that Skill as used or mounted

### Requirement: Plan execution trace correlation
The observability system SHALL correlate PlanRun, SubTaskRun, and SubTaskAttempt identities with their Agent sessions, provider generations, tool operations, validation operations, and state transitions while preserving the existing execution-run and trace topology.

#### Scenario: Inspect a SubTask attempt timeline
- **WHEN** a user opens the evidence for a SubTask attempt
- **THEN** the service boundary SHALL return a bounded timeline whose safe correlation fields connect the attempt to its session, generation, operations, and verification result

#### Scenario: Trace a PlanRun summary
- **WHEN** a PlanRun contains multiple serial attempts
- **THEN** the runtime SHALL expose their parent-child correlation and durations without embedding full session transcripts in the PlanRun summary

### Requirement: Redacted Plan telemetry
Plan execution diagnostics SHALL allow stable IDs, state names, durations, counts, safe filenames, exit classifications, and non-reversible fingerprints, and SHALL exclude user goals, generated task descriptions, prompts, credentials, raw tool arguments, raw tool results, and unredacted command output by default.

#### Scenario: Record an orchestration failure
- **WHEN** planning, dispatch, execution, verification, or recovery fails
- **THEN** the unified observability path SHALL persist a redacted classified event that remains useful for correlation without persisting prohibited content

#### Scenario: Preserve user-facing output separately
- **WHEN** a user inspects allowed Agent or validation output in the Plan UI
- **THEN** the frontend SHALL obtain it through the bounded session or operation presentation service rather than from diagnostic telemetry

### Requirement: Autonomous Plan loop trace correlation
The observability system SHALL correlate Plan driver activation, scheduling cycles, discovery sessions, original and repair Attempts, SubTask verification, final verification, pause and cancellation boundaries, and user recovery actions while preserving metadata-only diagnostic defaults.

#### Scenario: Trace an automatic repair chain
- **WHEN** one SubTask has multiple original or repair Attempts
- **THEN** the execution topology SHALL retain their sequence, parent PlanRun and SubTask identities, safe failure classes, durations, and terminal states without storing prompts or raw validation output in diagnostics

#### Scenario: Trace background continuation
- **WHEN** the native driver advances a PlanRun while no Plan view is open
- **THEN** unified logging SHALL record bounded lifecycle and correlation events sufficient to distinguish activation, claim, execution, verification, repair, and stop boundaries

#### Scenario: Correlate originating session navigation safely
- **WHEN** a PlanRun is associated with its originating OnePiece session
- **THEN** diagnostics MAY correlate non-secret session and PlanRun ids while excluding session titles, prompts, goals, and message content

#### Scenario: Inspect final verification evidence
- **WHEN** a user requests final verification details through the Plan service
- **THEN** the user-facing bounded evidence path MAY return allowed command summaries while persistent diagnostics SHALL continue excluding unredacted command output

### Requirement: Canonical lifecycle correlation
Execution observability SHALL reuse the canonical Run id for lifecycle correlation while retaining independent trace/span identity and telemetry status. Canonical transition persistence SHALL NOT depend on OTLP exporter or timeline availability.

#### Scenario: Canonical Run transitions
- **WHEN** a correlated Run waits, retries, verifies, or terminates
- **THEN** observability records a bounded safe lifecycle event without replacing the canonical Run state or inventing unavailable child detail

### Requirement: Performance evidence correlates with execution without blocking it
Performance evidence for an Agent Run SHALL use existing Run, operation, span, and dataset correlations and SHALL remain metadata-only, bounded, and non-blocking to the owning execution.

#### Scenario: Evidence recording fails
- **WHEN** a performance measurement cannot be persisted or exported
- **THEN** the Run SHALL continue according to its canonical outcome
- **AND** unified logging SHALL receive a bounded redacted failure classification without recursive telemetry

#### Scenario: Run performance result is exported
- **WHEN** dedicated benchmark evidence is produced for a Run
- **THEN** it SHALL identify commit, platform, profile, dataset, metric, baseline, delta, and correlation ids without captured execution content

### Requirement: Observable telemetry persistence failure
Execution telemetry failures MUST remain non-blocking to the owning operation and SHALL produce bounded, redacted local diagnostics without recursively using the failing telemetry path.

#### Scenario: Span start or finish fails
- **WHEN** local persistence or export rejects a span or run transition
- **THEN** the Agent operation SHALL continue according to its own outcome and the unified log SHALL receive a safe failure classification
