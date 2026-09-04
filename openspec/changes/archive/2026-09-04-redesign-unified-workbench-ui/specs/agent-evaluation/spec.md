# agent-evaluation Specification Delta

## ADDED Requirements

### Requirement: Evaluation experiment workflow
The Quality destination SHALL present Agent evaluation as an experiment workflow with a bounded experiment list, guided creation surface, result detail, and comparison route.

#### Scenario: Open Evaluations
- **WHEN** the user opens Quality and selects Evaluations
- **THEN** the page SHALL show recent experiments or arenas with task/version, selected Agents, state, outcome summary, and updated time

#### Scenario: Create an experiment
- **WHEN** the user activates New evaluation
- **THEN** a wizard or sheet SHALL collect benchmark task/version, filtered Agent selection, supported configuration, and final review
- **AND** the page header SHALL not expand into a permanent creation form

#### Scenario: Select many Agents
- **WHEN** the Agent catalog is large
- **THEN** the selector SHALL support search, status or capability filters, select-visible, and a bounded selected summary
- **AND** disabled Agents SHALL explain incompatibility

#### Scenario: Submit evaluation
- **WHEN** the review is valid
- **THEN** the service SHALL return stable asynchronous operation and experiment identities before attempts finish
- **AND** duplicate submission SHALL be prevented

### Requirement: Evaluation results data table
Evaluation result collections SHALL use a shared data-table or virtualized list model with server or bounded client pagination, sorting, filters, column visibility, row selection, and stable detail navigation.

#### Scenario: Render result collection
- **WHEN** an experiment has attempt or case results
- **THEN** the table SHALL prioritize outcome, Agent/configuration snapshot, task/case, core metrics, regression state, and duration
- **AND** low-frequency identifiers SHALL remain available through detail or column settings

#### Scenario: Sort or filter
- **WHEN** the user changes a supported result query
- **THEN** the result page SHALL restart pagination and preserve the selected experiment
- **AND** a stale selected row SHALL be reconciled explicitly

#### Scenario: Configure columns
- **WHEN** the user changes visible columns
- **THEN** the preference MAY persist as non-sensitive local state
- **AND** required outcome and identity columns SHALL remain available

#### Scenario: Render large results
- **WHEN** the fixture contains ten thousand result rows
- **THEN** the UI SHALL keep requested pages and mounted rows bounded
- **AND** row selection and keyboard navigation SHALL remain stable

### Requirement: Evaluation baseline and regression presentation
An experiment or comparison SHALL let the user select an eligible baseline and SHALL present metric deltas, outcome-tier changes, regressions, improvements, and unavailable comparisons transparently.

#### Scenario: Choose baseline
- **WHEN** two or more compatible experiments or attempts share the required task/version scope
- **THEN** the user SHALL be able to select one as the comparison baseline
- **AND** incompatible candidates SHALL be disabled with an explanation

#### Scenario: Show regression
- **WHEN** a candidate moves to a worse deterministic outcome tier or violates a configured regression rule
- **THEN** the UI SHALL show a non-color-only regression marker, bounded reason, and affected checks or metrics

#### Scenario: Show metric delta
- **WHEN** both baseline and candidate have comparable metric provenance
- **THEN** the UI SHALL show absolute or relative delta with units and direction

#### Scenario: Metric is unavailable
- **WHEN** one side lacks comparable data or provenance
- **THEN** the delta SHALL be unavailable rather than zero or inferred

### Requirement: Multi-experiment comparison
Evaluation SHALL provide a comparison route for two to four compatible experiments or arena results with aligned task rows, independent metric columns, baseline emphasis, and regression drill-down.

#### Scenario: Compare experiments
- **WHEN** the user selects between two and four compatible experiments
- **THEN** the page SHALL align comparable task/version rows and display each experiment's immutable Agent/configuration snapshot

#### Scenario: Comparison is incompatible
- **WHEN** selected experiments use incompatible task or manifest versions for the requested view
- **THEN** the UI SHALL identify the mismatch and prevent misleading row alignment

#### Scenario: Open a differing result
- **WHEN** the user activates a regression, improvement, failure, or metric delta
- **THEN** the detail Inspector SHALL show the involved outcomes, checks, reasons, and EvidenceLinks

#### Scenario: Share comparison
- **WHEN** the selected experiment identities are URL-safe
- **THEN** the route SHALL encode only stable ids and supported view state
- **AND** it SHALL not encode prompts, outputs, artifacts, or secrets

### Requirement: Explained evaluation outcomes
Every displayed evaluation outcome SHALL include a semantic classification and make its deterministic checks, optional judge evidence, thresholds, measured values, provenance, and bounded reason inspectable.

#### Scenario: Render PASS or success
- **WHEN** an attempt is deterministically successful
- **THEN** the UI SHALL identify the authoritative checks and SHALL not imply that an optional judge overrode deterministic verification

#### Scenario: Render task failure
- **WHEN** one or more deterministic checks fail
- **THEN** the detail SHALL identify failed checks, expected condition, measured result, and available evidence

#### Scenario: Render benchmark error
- **WHEN** the harness, manifest, isolation, verifier, or persistence fails independently of Agent output
- **THEN** the UI SHALL distinguish benchmark error from Agent task failure

#### Scenario: Render missing metric
- **WHEN** token, cost, context, or reliability data lacks reliable provenance
- **THEN** the field SHALL be absent or explicitly unavailable and SHALL not appear as zero

### Requirement: Evaluation artifact evidence links
Evaluation artifacts, Runs, Sessions, files, diffs, logs, traces, and context evidence SHALL render as typed EvidenceLinks with safe labels and availability state rather than as unactionable raw identifiers.

#### Scenario: Open an artifact
- **WHEN** a result references an available bounded artifact
- **THEN** the UI SHALL navigate to or open the owning safe artifact surface
- **AND** the stable id MAY be available through copy details without being the only label

#### Scenario: Artifact is unavailable
- **WHEN** the owning service reports a missing, expired, cleaned, or unsupported artifact
- **THEN** the UI SHALL show unavailable reason and SHALL not render an active-looking link

#### Scenario: Artifact is restricted
- **WHEN** permission denies access
- **THEN** the UI SHALL show restricted status without exposing protected paths or content

### Requirement: Evaluation visibility-aware updates
Active evaluation state SHALL update through coalesced events when available and bounded reconciliation with visibility-aware polling or backoff as recovery.

#### Scenario: Receive active progress
- **WHEN** an experiment page is visible and attempt state changes
- **THEN** the relevant experiment and result summaries SHALL update in bounded batches

#### Scenario: Hide Evaluation
- **WHEN** the route or document becomes hidden
- **THEN** page-owned one-second polling SHALL stop or back off according to lifecycle policy
- **AND** native execution SHALL continue

#### Scenario: Return after missing events
- **WHEN** the page becomes visible or reconnects
- **THEN** a bounded query SHALL reconcile experiment, attempts, metrics, and selected result

### Requirement: Evaluation component boundaries
The evaluation frontend SHALL separate query and mutation models, experiment toolbar, Agent selector, result table, result detail, comparison, and shared presentation helpers so each production file remains within repository size limits.

#### Scenario: Check architecture
- **WHEN** evaluation source files are analyzed
- **THEN** no production TS or TSX file SHALL exceed the repository line limit
- **AND** React components SHALL not call Tauri APIs directly

#### Scenario: Use Web/mock
- **WHEN** Evaluation runs through the Web adapter
- **THEN** the same UI states and contract shapes SHALL be available with clearly identified deterministic simulation
