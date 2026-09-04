# agent-evaluation Specification

## Purpose
TBD - created by archiving change add-agent-evaluation-platform. Update Purpose after archive.
## Requirements
### Requirement: Versioned benchmark manifests are bounded and safe
The system SHALL load benchmark tasks from a versioned manifest containing a stable id, positive version, supported category, bounded fixture path and prompt, bounded timeout, declarative acceptance, and metric policy, and SHALL reject unsafe or unsupported input before creating an execution workspace.

#### Scenario: Valid built-in manifest loads
- **WHEN** the catalog loads a supported built-in manifest and its fixture
- **THEN** it SHALL expose the immutable task id/version, category, configuration schema, acceptance summary, and metric policy

#### Scenario: Manifest requests an arbitrary command
- **WHEN** a manifest contains a free-form command, shell metacharacter, absolute path, traversal, unsupported version/category, excessive timeout, or unknown verifier profile
- **THEN** the system SHALL reject it as a benchmark harness error without executing a host process or creating an Agent Run

### Requirement: Evaluation attempts use clean isolated fixtures
Each evaluation attempt SHALL run against a fresh bounded fixture copy or worktree fixed to the task revision and manifest version, SHALL NOT share generated files with another attempt, and SHALL clean incomplete isolation state after cancellation, timeout, or setup failure.

#### Scenario: Arena runs two Agents
- **WHEN** OnePiece and a managed CLI Agent are selected for the same task
- **THEN** each attempt SHALL receive a distinct clean workspace with the same initial revision and manifest version

#### Scenario: Isolation setup is unsafe
- **WHEN** a fixture contains a traversal, escaping symlink, unsupported special file, or exceeds configured bounds
- **THEN** the attempt SHALL terminate as `benchmark_error` before Agent invocation and SHALL record a redacted safe reason

### Requirement: Evaluation reuses canonical Runs and asynchronous operations
Every accepted evaluation attempt SHALL own a canonical Agent Run linked to its arena, operation, task version, Agent snapshot, and available execution/context evidence, and variable-duration execution SHALL proceed asynchronously outside the Tauri main thread and browser event loop.

#### Scenario: Evaluation starts
- **WHEN** a valid arena request is accepted
- **THEN** the service SHALL return a stable arena/operation identity before all attempts finish and SHALL expose queued, running, waiting, and terminal projections

#### Scenario: Attempt is cancelled or times out
- **WHEN** cancellation is accepted or the task timeout expires
- **THEN** the canonical Run SHALL reach the matching terminal state, process work SHALL stop, partial evidence SHALL remain bounded, and cleanup SHALL run idempotently

### Requirement: Agent and configuration snapshots are immutable
Each attempt SHALL record stable Agent, provider, model, interaction mode, and effective configuration identifiers available at dispatch time and SHALL preserve the snapshot even if current configuration later changes.

#### Scenario: Configuration changes after completion
- **WHEN** a user changes an Agent model or profile after an evaluation completes
- **THEN** historical results and exported JSON SHALL retain the original snapshot and SHALL NOT display the new configuration as if it ran the task

### Requirement: Deterministic verification is authoritative
The evaluator SHALL apply allowlisted acceptance commands, static assertions, and repository diff rules before any optional structured judge, SHALL record each check result, and SHALL NOT allow an LLM judge to convert a deterministic failure into success.

#### Scenario: Tests fail but judge approves
- **WHEN** an acceptance command fails and an optional judge produces a favorable score
- **THEN** the attempt SHALL remain failed and the judge result SHALL be displayed only as non-authoritative evidence

#### Scenario: Benchmark harness fails
- **WHEN** manifest parsing, isolation, verifier launch, or result persistence fails independently of Agent output
- **THEN** the result SHALL use a benchmark-error classification distinct from Agent task failure

### Requirement: Metrics preserve quality and missing values
Evaluation results SHALL expose outcome, efficiency, context, and reliability metrics with source/quality provenance, SHALL leave unavailable values absent, and SHALL calculate monetary cost only from an explicit versioned pricing snapshot.

#### Scenario: Provider reports no token usage
- **WHEN** an Agent attempt completes without reliable token accounting
- **THEN** input, output, and cache tokens SHALL be unavailable or explicitly estimated and SHALL NOT be presented as exact reported values

#### Scenario: Context evidence exists
- **WHEN** a run has a Context Engine evidence manifest
- **THEN** the result SHALL link bounded selected/relevant/irrelevant evidence counts and token efficiency to the evaluation attempt without copying evidence content

#### Scenario: Pricing is unavailable
- **WHEN** no reliable pricing snapshot matches the provider/model snapshot
- **THEN** cost SHALL remain unavailable rather than using current or invented pricing

### Requirement: Failures and recovery are classified
The evaluator SHALL distinguish success, deterministic task failure, Agent failure, timeout, stuck, cancelled, and benchmark error, and SHALL record bounded retry, replan, recovery, flaky-result, and human-intervention counts when observable.

#### Scenario: Agent stops making progress
- **WHEN** the bounded stuck policy expires without progress evidence
- **THEN** the attempt SHALL terminate as stuck rather than success or generic failure

#### Scenario: Repeated deterministic verification differs
- **WHEN** repeated verifier results for the same frozen output disagree
- **THEN** the attempt SHALL be marked flaky and SHALL preserve both bounded verification outcomes

### Requirement: Arena comparison is transparent and versioned
The system SHALL compare attempts for the same task/version using a documented ranking algorithm version and independent metric columns, SHALL NOT synthesize an opaque total score from mutually unavailable metrics, and SHALL NOT let an attempt that recorded no evidence outrank an attempt that recorded evidence of the same or better quality. Attempts SHALL be ordered by outcome tier first — deterministic success, then deterministic task failure, then non-completion outcomes (Agent failure, timeout, stuck, cancelled, benchmark error) — before any evidence-count or metric comparison is applied, and the ranking SHALL be applied wherever an arena is read rather than only where it is computed. A change to this ordering SHALL be published as a new ranking algorithm version, and every arena, exported payload, and stored snapshot SHALL name the version its ordering was produced under.

#### Scenario: Agents have different metric coverage
- **WHEN** one result has reported token usage and another has unavailable token usage
- **THEN** the table SHALL show the coverage difference and SHALL NOT treat the unavailable value as zero or use it as a hidden ranking advantage

#### Scenario: Deterministic outcomes differ
- **WHEN** one Agent passes all deterministic acceptance and another fails
- **THEN** the passing result SHALL rank ahead regardless of optional judge score

#### Scenario: One attempt failed a check and another produced no checks at all
- **WHEN** one attempt is a deterministic task failure with one or more recorded failing checks and another attempt ended in a non-completion outcome with no recorded checks
- **THEN** the task failure SHALL rank ahead of the non-completion outcome, and the empty check list SHALL NOT be read as "zero failures"

#### Scenario: Arena is read back after it finished
- **WHEN** a stored arena is listed, fetched, or exported, in any order the underlying store returns its attempts
- **THEN** the attempts SHALL be presented in ranked order, and two attempts that the ranking cannot distinguish SHALL be returned in a stable order that does not vary between reads

#### Scenario: Ranking semantics change
- **WHEN** the comparison rules are changed such that two attempts could order differently than before
- **THEN** the ranking algorithm version SHALL be incremented, arenas recorded under an earlier version SHALL keep naming that earlier version, and exports SHALL NOT present results produced under different versions as directly comparable

### Requirement: Evaluation results are bounded, persistent, and exportable
The desktop runtime SHALL persist safe evaluation task/run/snapshot/metric/verification metadata and artifact references in SQLite, SHALL keep large logs and diffs in bounded existing stores, and SHALL export a versioned content-safe JSON result.

#### Scenario: Result is exported
- **WHEN** a user exports an arena or attempt
- **THEN** the JSON SHALL include schema/ranking versions, task and Agent snapshots, metric provenance, verification, classifications, correlations, and safe artifact references

#### Scenario: Sensitive execution data exists
- **WHEN** prompts, credentials, environment values, raw tool payloads, private absolute paths, logs, or diffs are produced
- **THEN** evaluation rows, export, and unified diagnostics SHALL omit or redact those values before persistence

#### Scenario: Retention runs
- **WHEN** evaluation metadata or artifacts exceed configured age/count/size bounds
- **THEN** maintenance SHALL prune eligible oldest data without deleting canonical Runs, chat messages, or unrelated artifacts

### Requirement: Frontend evaluation service has runtime parity
React SHALL access catalog, configuration, start/cancel/status, result listing/detail/comparison, timeline, and export through the shared Agent service interface implemented by both Tauri and Web/mock adapters.

#### Scenario: Desktop starts an evaluation
- **WHEN** the Eval page runs in Tauri
- **THEN** the Tauri adapter SHALL call declared Rust commands and React SHALL NOT invoke Tauri or access SQLite/process/filesystem APIs directly

#### Scenario: Web runs a mock evaluation
- **WHEN** the Eval page runs in browser mode
- **THEN** the Web adapter SHALL simulate deterministic isolated attempts and lifecycle transitions without claiming native process or SQLite side effects

### Requirement: Local Eval workspace supports the complete workflow
The application SHALL provide a local Eval/Benchmark page for catalog filtering, task/Agent configuration, live run status, results, comparison, task detail, bounded diff/verification, context/tool timeline, and JSON export.

#### Scenario: Complete mock benchmark
- **WHEN** a user selects a built-in task and two mock Agents, starts the arena, waits for completion, compares results, opens verification/timeline details, and exports JSON
- **THEN** every step SHALL remain available through accessible translated controls and stable service-backed state

#### Scenario: Narrow layout and both themes
- **WHEN** the page is rendered at desktop and narrow widths in futuristic and minimal styles
- **THEN** catalog, run controls, status, comparison, detail, and export SHALL remain readable and operable without overlap, clipping, blank panels, or layout-shifting states

### Requirement: Deterministic fixtures and performance evidence protect the framework
The repository SHALL include between three and five stable fixture tasks spanning coding, tool-use, context, or planning behavior, a deterministic fake Agent requiring no paid external model, and repeatable benchmark-framework performance checks based on bounded structural budgets.

#### Scenario: CI evaluates the fake Agent
- **WHEN** CI runs the evaluation framework without network credentials
- **THEN** manifest parsing, isolation reset, execution, verification, metrics, persistence, comparison, and export SHALL complete deterministically

#### Scenario: Framework processes bounded results
- **WHEN** the performance benchmark evaluates the maximum supported MVP arena and result page sizes
- **THEN** it SHALL meet documented allocation/query/count budgets without relying on fragile shared-runner wall-clock thresholds

### Requirement: Native smoke can exercise a minimal real benchmark
The desktop test harness SHALL support one minimal real local benchmark using an installed supported Agent and SHALL report the current operating system result without extrapolating to other platforms.

#### Scenario: Supported Agent is installed
- **WHEN** native desktop smoke selects an installed OnePiece or managed CLI Agent and a minimal fixture
- **THEN** it SHALL create isolated state, dispatch through the existing Agent runtime, verify the result, expose it through the UI/service boundary, and clean up according to policy

#### Scenario: No supported Agent is available
- **WHEN** the native host has no eligible configured Agent
- **THEN** the smoke SHALL report BLOCKED with evidence rather than substituting a fake result and claiming a real-Agent pass

### Requirement: Agent dispatch failures record bounded diagnostic evidence
When an evaluation attempt fails before or during Agent dispatch, the attempt SHALL record at least one failed, redacted, bounded diagnostic check naming why dispatch failed, and that check SHALL be presented as non-authoritative evidence that never converts a failure into a success.

#### Scenario: Agent cannot be dispatched
- **WHEN** an attempt's Agent cannot be dispatched — no configured model, unusable interaction mode, session creation refused, or the terminal channel disconnects
- **THEN** the attempt SHALL terminate as `agent_failed` and SHALL carry a failed diagnostic check whose summary states the reason within the bounded, redacted form applied to every persisted evaluation field

#### Scenario: Diagnostic evidence would leak execution data
- **WHEN** a dispatch failure reason contains a prompt, credential, environment value, raw tool payload, or private absolute path
- **THEN** the recorded summary SHALL omit or redact those values before persistence, export, and display, in the same way every other evaluation row is redacted

#### Scenario: Diagnostic evidence is not a verdict
- **WHEN** an attempt carries a dispatch diagnostic check
- **THEN** the check SHALL NOT be counted as a deterministic acceptance result and SHALL NOT rank the attempt below an attempt whose failure recorded no evidence at all

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

### Requirement: Provider-specific desktop evaluation qualification is truthful
The desktop evaluation harness SHALL support focused qualification of the stable `opencode` and `onepiece` Agent ids, SHALL distinguish deterministic fixture execution from live-provider execution, and SHALL NOT report a fixture result as proof that a real provider completed the benchmark.

#### Scenario: OpenCode fixture evaluation
- **WHEN** the required desktop gate evaluates `opencode` through the repository fixture
- **THEN** it completes the arena lifecycle without network credentials and records fixture provenance in the bounded test evidence

#### Scenario: Live OpenCode is unavailable
- **WHEN** live OpenCode qualification is requested but the executable or provider authentication is unavailable
- **THEN** the qualification reports `BLOCKED` with a safe prerequisite reason and does not substitute the fixture result

#### Scenario: Live OnePiece evaluation
- **WHEN** OnePiece qualification is requested with a process-scoped provider credential
- **THEN** the harness evaluates stable Agent id `onepiece`, waits for a terminal arena result, verifies persisted and rendered evidence, and removes the credential before evidence is written

#### Scenario: OnePiece credential is absent
- **WHEN** OnePiece qualification is requested without an accessible provider credential
- **THEN** the qualification reports `BLOCKED` and no arena is presented as a live-provider pass

### Requirement: Focused evaluation evidence is actionable and secret-safe
Each provider-specific evaluation qualification SHALL record its task identity, Agent id, fixture-or-live provenance, terminal outcome, arena and attempt correlation, and result status while omitting credentials, raw prompts, environment values, private absolute paths, and unbounded provider output.

#### Scenario: Evaluation dispatch fails
- **WHEN** OpenCode or OnePiece fails before deterministic verification
- **THEN** the evidence includes the bounded dispatch diagnostic exposed by the evaluation result and the qualification reports `FAILED` rather than timing out without explanation

#### Scenario: Evidence is audited
- **WHEN** the desktop layer finishes or aborts
- **THEN** its evidence can be checked for forbidden credential values and unsafe provider payloads without reading the provider credential itself

