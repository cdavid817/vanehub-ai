## ADDED Requirements

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
The system SHALL compare attempts for the same task/version using a documented ranking algorithm version and independent metric columns, and SHALL NOT synthesize an opaque total score from mutually unavailable metrics.

#### Scenario: Agents have different metric coverage
- **WHEN** one result has reported token usage and another has unavailable token usage
- **THEN** the table SHALL show the coverage difference and SHALL NOT treat the unavailable value as zero or use it as a hidden ranking advantage

#### Scenario: Deterministic outcomes differ
- **WHEN** one Agent passes all deterministic acceptance and another fails
- **THEN** the passing result SHALL rank ahead regardless of optional judge score

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
