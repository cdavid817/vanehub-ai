## ADDED Requirements

### Requirement: Monotonic execution status transitions

Execution run and span status SHALL advance monotonically on every write path. A start, a finish, and any replay of either SHALL NOT return a record that already reached a terminal status to a non-terminal one, and SHALL NOT leave a record whose status and terminal timestamp disagree. A repeated start SHALL be idempotent for metadata the producer may legitimately supply late, and inert for lifecycle state.

#### Scenario: Late start replays against a terminal run

- **WHEN** a start transition arrives for a run that already reached `succeeded`, `failed`, or `cancelled`
- **THEN** the stored run SHALL retain its terminal status, terminal timestamp, and error classification
- **AND** the write SHALL NOT be reported as a failure to the producer, because a retried start is an expected recovery behavior rather than a caller error

#### Scenario: Late start replays against a terminal span

- **WHEN** a start transition arrives for a span that already reached a terminal status
- **THEN** the stored span SHALL retain its terminal status, terminal timestamp, and error classification
- **AND** any correlation metadata the replayed start carries MAY be completed where it was previously absent

#### Scenario: Terminal status and terminal timestamp stay consistent

- **WHEN** any sequence of start and finish transitions is applied to one run or span in any order
- **THEN** the stored record SHALL NOT end in a state that reports a non-terminal status together with a terminal timestamp
- **AND** a reader SHALL NOT have to infer which of the two fields to believe

## MODIFIED Requirements

### Requirement: Structured execution span classification

Execution timeline DTOs SHALL expose a versioned structured span kind derived by the native observability layer from pinned semantic conventions and documented `vanehub.*` attributes. React MUST NOT classify span behavior by matching display-name substrings. VaneHub's own span producers SHALL declare their structured kind explicitly rather than relying on inference from attributes they do not emit, and the value reported when no classification applies SHALL be named identically in this specification, the service DTO, and every consuming surface.

#### Scenario: Classify a tool span

- **WHEN** a span carries the pinned standard or VaneHub semantic attributes for a tool invocation
- **THEN** the timeline SHALL return `tool` as its structured span kind regardless of the span display name

#### Scenario: Classify a process span with an arbitrary name

- **WHEN** a managed process span has a user-visible name that does not contain `process`, `shell`, or `tool`
- **THEN** its structured kind SHALL still be derived from native attributes
- **AND** React SHALL render it according to that kind rather than guessing from the name

#### Scenario: No known semantic classification

- **WHEN** a span has no applicable pinned or documented kind attribute
- **THEN** the timeline SHALL return `unknown`
- **AND** it SHALL preserve the span's reported fidelity without inventing a more specific category

#### Scenario: VaneHub producer emits a span it can classify itself

- **WHEN** a VaneHub-owned producer emits a span whose kind it knows at emission time
- **THEN** the emitted attributes SHALL carry that declared kind through redaction and storage to the timeline DTO
- **AND** the resulting structured kind SHALL NOT be `unknown` merely because the producer omitted a conventional attribute its name implies

#### Scenario: Declared kind and opaque fidelity coexist

- **WHEN** a producer declares the kind of a span whose internal steps it cannot observe
- **THEN** the timeline SHALL report both the declared kind and `opaque` fidelity
- **AND** neither value SHALL suppress the other, because knowing what a span is and seeing inside it are independent facts

### Requirement: Live local execution timeline updates

The desktop runtime SHALL publish bounded identifier-only notices after committed run, span, and event transitions so a visible Traces panel can refresh an active timeline without polling the complete store continuously. A live refresh SHALL replace timeline data in place: it SHALL NOT be modelled as a different query, and it SHALL NOT discard loaded pages, the reader's selected run or span, or open detail, filter, and zoom state.

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

#### Scenario: Refresh arrives while the reader is deep in the run list

- **WHEN** a transition triggers a refresh after the reader has loaded additional run-list pages and selected a run other than the newest
- **THEN** the loaded pages and the selected run SHALL survive the refresh
- **AND** the panel SHALL NOT silently reselect the newest run on the reader's behalf

#### Scenario: Refresh arrives while span detail is open

- **WHEN** a transition triggers a refresh while a span detail surface, filter, or zoom level is active
- **THEN** those states SHALL survive the refresh
- **AND** previously displayed data SHALL remain visible while the refreshed data is fetched, rather than being replaced by an empty or loading state

#### Scenario: A newer run appears while a historical run is selected

- **WHEN** a new run starts while the reader has deliberately selected an older run
- **THEN** the panel SHALL indicate that a newer run exists without navigating away from the reader's selection
- **AND** it SHALL follow the newest run only when the reader has asked it to

### Requirement: Waterfall-ready bounded timeline projection

The execution timeline service SHALL provide bounded derived layout metadata required for a virtualized waterfall without changing canonical span timestamps or inventing unavailable duration. Lifecycle state and duration-measurement quality are independent facts: an absent duration SHALL NOT by itself denote a running span, and every surface that renders duration (bar geometry, textual detail, and accessible description) SHALL distinguish "not finished yet" from "finished, duration not measurable".

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

#### Scenario: Terminated span has an unusable duration measurement

- **WHEN** a span reached `failed`, `cancelled`, `incomplete`, or `succeeded` but its duration cannot be derived because a timestamp is absent, unparseable, or ends before it starts
- **THEN** the span SHALL be presented as ended with an unknown duration
- **AND** its bar SHALL NOT be drawn as open-ended and its accessible description SHALL NOT state that it is still running

#### Scenario: Reader asks how complete the event record is

- **WHEN** a timeline response omits events because the run exceeds the bounded event limit
- **THEN** the response SHALL report the omission and SHALL offer a stable continuation for the remaining events
- **AND** a consumer SHALL be able to distinguish a run that recorded no events from a response that stopped at its limit

#### Scenario: Diagnosing a failure recorded after the event bound

- **WHEN** the event that explains a run's failure falls outside the first bounded page of events
- **THEN** the reader SHALL still be able to reach that event through the reported continuation
- **AND** the panel SHALL NOT present the bounded first page as the complete event record
