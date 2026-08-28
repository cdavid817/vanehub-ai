## ADDED Requirements

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
