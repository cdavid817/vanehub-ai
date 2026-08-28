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

### Requirement: Structured execution span classification

Execution timeline DTOs SHALL expose a versioned structured span kind derived by the native observability layer from pinned semantic conventions and documented `vanehub.*` attributes. React MUST NOT classify span behavior by matching display-name substrings.

#### Scenario: Classify a tool span

- **WHEN** a span carries the pinned standard or VaneHub semantic attributes for a tool invocation
- **THEN** the timeline SHALL return `tool` as its structured span kind regardless of the span display name

#### Scenario: Classify a process span with an arbitrary name

- **WHEN** a managed process span has a user-visible name that does not contain `process`, `shell`, or `tool`
- **THEN** its structured kind SHALL still be derived from native attributes
- **AND** React SHALL render it according to that kind rather than guessing from the name

#### Scenario: No known semantic classification

- **WHEN** a span has no applicable pinned or documented kind attribute
- **THEN** the timeline SHALL return `other`
- **AND** it SHALL preserve the span's reported fidelity without inventing a more specific category

### Requirement: Live local execution timeline updates

The desktop runtime SHALL publish bounded identifier-only notices after committed run, span, and event transitions so a visible Traces panel can refresh an active timeline without polling the complete store continuously.

#### Scenario: Running span completes

- **WHEN** a visible selected run receives a committed span terminal transition
- **THEN** the frontend adapter SHALL notify the active timeline query using safe run/span ids and sequence metadata
- **AND** the Traces panel SHALL refresh the affected run with bounded debounce

#### Scenario: Traces panel is hidden

- **WHEN** the mounted Traces panel is not visible
- **THEN** it SHALL unsubscribe or suspend live timeline refresh
- **AND** reopening it SHALL query current service state before presenting the timeline as current

#### Scenario: Live-notice queue drops updates

- **WHEN** a bounded subscriber queue cannot deliver one or more notices
- **THEN** it SHALL emit one safe gap notice or cause query invalidation
- **AND** the UI SHALL not assume that its current timeline is complete until it refreshes

### Requirement: Waterfall-ready bounded timeline projection

The execution timeline service SHALL provide bounded derived layout metadata required for a virtualized waterfall without changing canonical span timestamps or inventing unavailable duration.

#### Scenario: Render completed nested spans

- **WHEN** a bounded run contains completed nested spans
- **THEN** the service SHALL expose depth, start offset, duration, status, fidelity, and structured kind for each span
- **AND** the UI SHALL be able to render the same span set in tree and time-waterfall form

#### Scenario: Render running or incomplete span

- **WHEN** a span has no verified terminal timestamp
- **THEN** its duration SHALL remain running or unavailable according to canonical state
- **AND** the service SHALL NOT manufacture an end time solely for waterfall layout

#### Scenario: Timeline exceeds a configured bound

- **WHEN** a run contains more spans or events than the bounded timeline response permits
- **THEN** the service SHALL return truncation and coverage metadata
- **AND** the waterfall SHALL identify partial data rather than implying the omitted topology does not exist

### Requirement: Critical-path, retry, and delegation metadata

The local timeline MAY derive bounded critical-path, attempt, retry, and delegation presentation metadata from verified topology, but it SHALL identify insufficient evidence rather than infer nonexistent dependencies.

#### Scenario: Derive a completed critical path

- **WHEN** a completed run has verified parent/child or link relationships and terminal timestamps sufficient to calculate the longest dependent path
- **THEN** the timeline MAY mark spans on that path as critical
- **AND** the derivation SHALL not alter canonical span relationships or durations

#### Scenario: Retry relationship is observed

- **WHEN** the runtime records an explicit attempt or retry link
- **THEN** the timeline SHALL expose the attempt and relationship for presentation
- **AND** it SHALL retain independent span identity and any independent run/trace identity required by the existing observability specification

#### Scenario: Delegation detail is opaque

- **WHEN** delegated work is known but child topology is unavailable
- **THEN** the timeline SHALL expose an opaque delegation boundary
- **AND** it SHALL not fabricate child-Agent spans or a critical path through unknown work

### Requirement: Cross-signal execution evidence links

Execution run and span summaries SHALL expose bounded counts and service-owned link keys for correlated logs, execution records, file mutations, review findings, verification outcomes, and usage observations when those correlations exist.

#### Scenario: Span has correlated logs and command

- **WHEN** a span has indexed logs and an execution command record sharing verified correlation
- **THEN** the span detail SHALL expose bounded counts and query targets for Logs and Terminal History
- **AND** the timeline DTO SHALL NOT embed raw log messages, command output, or terminal transcript

#### Scenario: Span has correlated file changes

- **WHEN** one or more safe file-mutation observations are correlated to a span
- **THEN** the span detail SHALL expose file/Changes targets using relative-path or fingerprint metadata
- **AND** it SHALL not persist source content or full diffs in observability attributes

#### Scenario: Correlated source is unavailable

- **WHEN** an owning log, evidence, workspace, review, or usage source is unavailable or outside retention
- **THEN** the span detail SHALL mark that linked section partial or unavailable
- **AND** it SHALL preserve the rest of the timeline

### Requirement: Accessible execution waterfall and span detail

The Traces panel SHALL provide a virtualized run list, time waterfall, structured legend and filters, keyboard-selectable spans, and a detail surface for safe Overview, Attributes, Events, Logs, Commands, Files, Findings, Usage, Error, and coverage information.

#### Scenario: Use desktop-width Traces layout

- **WHEN** Traces renders at desktop width
- **THEN** the run list, waterfall, and selected-span detail SHALL remain simultaneously usable without unbounded row mounting

#### Scenario: Use narrow-width Traces layout

- **WHEN** Traces renders at narrow width
- **THEN** the run list or span detail SHALL move into an accessible drawer or switchable region
- **AND** horizontal timeline navigation SHALL remain recoverable inside the waterfall region

#### Scenario: Select span by keyboard

- **WHEN** a keyboard user moves through visible waterfall rows and selects a span
- **THEN** focus SHALL remain visible and the same detail/cross-link actions available to pointer users SHALL be reachable

#### Scenario: Use either visual style

- **WHEN** `futuristic` or `minimal` is active
- **THEN** shared semantic tokens SHALL identify running, succeeded, failed, cancelled, incomplete, critical, selected, fidelity, and partial-coverage states without relying on color alone or shifting layout

### Requirement: Bounded execution run comparison

The Traces or Report experience MAY compare two bounded execution runs using safe status, duration, usage-quality, tool, failure, change, and verification summaries, and MUST NOT compare raw prompt, output, terminal, or source content.

#### Scenario: Compare two retained runs

- **WHEN** a user selects two retained runs from the same session
- **THEN** the service SHALL return bounded comparable dimensions with per-source coverage
- **AND** the UI SHALL link each difference back to its owning run evidence

#### Scenario: One run has partial evidence

- **WHEN** one compared run lacks retained logs, commands, usage, or change evidence
- **THEN** the comparison SHALL mark that dimension partial or unavailable
- **AND** it SHALL not present missing evidence as an improvement or zero-value result

