## Purpose

Provides a bounded, attention-first operational view of canonical Agent Runs while preserving the ownership, security, and detailed workflows of existing Sessions, Plans, Loops, Reviews, Approvals, Evaluations, and logs.

## ADDED Requirements

### Requirement: Bounded Mission Control overview
The system SHALL expose summary counts for running, waiting approval, waiting user, retrying, blocked or stuck, failed, and recently completed Runs, plus bounded attention, active, and recent-completion pages derived from canonical Run snapshots. It MUST NOT load full aggregates, logs, diffs, prompts, tool payloads, or artifact bodies as part of the overview query.

#### Scenario: More than one hundred historical Runs exist
- **WHEN** Mission Control loads with more than one hundred historical Runs
- **THEN** the native runtime returns only configured bounded pages and summary counts through indexed queries
- **AND** the number of native queries does not grow with the number of returned Runs

#### Scenario: Multiple Agents run concurrently
- **WHEN** Runs belonging to different Agents transition independently
- **THEN** every overview row reflects its own latest canonical state, owner, title, elapsed state, workspace, phase, attention reason, and verification summary

### Requirement: Honest summary fields and terminal timing
Each summary SHALL expose stable Run and owner identities, Agent identity, safe title, canonical state, timestamps, optional workspace or worktree, optional phase, attention classification and bounded reason, verification status, and supported actions. Token or cost values SHALL appear only with reliable provenance, and terminal Runs SHALL use their terminal timestamp rather than a continuing timer.

#### Scenario: Usage provenance is absent
- **WHEN** a Run has no reported or explicitly classified estimated usage or no matching versioned price
- **THEN** Mission Control marks token or cost data unavailable and does not invent or silently treat it as zero

#### Scenario: Completed Run remains visible
- **WHEN** a completed Run appears in the recent section
- **THEN** its elapsed duration remains fixed across subsequent refreshes

#### Scenario: Retry or stuck needs explanation
- **WHEN** a Run is retrying, blocked, stuck, or failed
- **THEN** the summary presents a bounded safe reason classification without exposing raw provider errors or sensitive content

### Requirement: Filtered and ordered Run discovery
Mission Control SHALL support canonical status, stable Agent id, project, and runner filters plus newest, oldest, and needs-attention-first ordering. Local or remote runner filtering MAY remain hidden until reliable runner data is available, but hidden fields MUST NOT be fabricated.

#### Scenario: Attention ordering is selected
- **WHEN** the user orders Runs by needs attention first
- **THEN** waiting approval, waiting user, stuck or blocked, failed, and review-requested Runs precede non-attention Runs with deterministic tie-breaking

#### Scenario: Filters produce another page
- **WHEN** the user changes a filter or ordering option
- **THEN** pagination restarts from the first page and the result contains only matching Runs

### Requirement: Attention inbox and owning-surface navigation
The attention inbox SHALL prioritize waiting approval, waiting user, stuck or blocked, failed, and review-requested Runs. Its controls SHALL navigate to the authoritative Approval, Session, Code Review Center, or owning workflow and MUST NOT duplicate chat, editor, diff, or approval decision experiences inside the dashboard.

#### Scenario: Waiting approval is opened
- **WHEN** the user opens a waiting-approval attention item
- **THEN** the application navigates to the existing authoritative approval surface for the matching request or Run

#### Scenario: Review is opened
- **WHEN** the user chooses Review Changes for a Run with a review target
- **THEN** the application opens the existing Code Review Center with that target rather than rendering another review implementation

### Requirement: Lazy and truthful Run detail
Run detail SHALL provide Overview, Plan/Tasks, Timeline, Tools, Files/Artifacts, Review, Tests/Verification, Context, Usage, and Logs sections. Each section SHALL declare available, unavailable, or restricted status and SHALL load bounded evidence through its owning service only after selection.

#### Scenario: Evidence does not exist
- **WHEN** a Run has no Context, Review, Plan, Usage, or artifact evidence
- **THEN** the corresponding section displays a localized unavailable state and no synthetic fixture or placeholder evidence

#### Scenario: Logs tab is selected
- **WHEN** the user selects Logs
- **THEN** the page requests a bounded log page correlated to the Run through the existing logging contract and does not load logs for unrelated Runs

### Requirement: State-aware control actions
Mission Control SHALL expose only actions permitted by canonical Run state, owning runtime policy, and permission contracts: Open, Cancel, Resume, Retry, approval navigation, Review Changes, and Run Verification. Mutating actions MUST use the shared service contract, require stable current identity or version witnesses where supported, and return reconciled canonical state.

#### Scenario: Cancel races with completion
- **WHEN** a cancellation loses a race to terminal completion
- **THEN** the action reports or returns the existing terminal state without reversing it or duplicating terminal effects

#### Scenario: Unsupported retry is inspected
- **WHEN** a failed Run's owner does not support retry
- **THEN** Retry is absent or disabled with an accessible explanation and no mutation request is sent

#### Scenario: Verification is requested
- **WHEN** the user requests verification for an eligible Run
- **THEN** the owning verification workflow starts asynchronously with a stable operation identity and Mission Control reconciles its canonical Run state

### Requirement: Coalesced events and deterministic reconciliation
Mission Control SHALL promptly apply canonical state transitions, immediately flush terminal and attention transitions, coalesce high-frequency progress or usage events, and reconcile by bounded query on mount, reconnect, and application focus.

#### Scenario: Token events arrive rapidly
- **WHEN** many usage or token progress events arrive without a state transition
- **THEN** the dashboard batches their presentation and does not rerender once per token event

#### Scenario: Event is missed while unfocused
- **WHEN** the application regains focus after missing a Run transition
- **THEN** a bounded reconciliation query replaces stale summary state with persisted canonical state

### Requirement: Runtime parity and deterministic Web fixtures
React SHALL access Mission Control only through the shared Agent service interface. Tauri SHALL use declared native commands, while Web/mock SHALL expose deterministic multi-Run, waiting, failed, completed, unavailable-evidence, filtering, pagination, action, and reconciliation behavior without claiming native persistence or side effects.

#### Scenario: Web fixture is reopened
- **WHEN** the deterministic Web/mock Mission Control is initialized again from the same fixture seed
- **THEN** stable Run identities, initial states, filters, attention ordering, and unavailable fields are reproducible

#### Scenario: Desktop loads Mission Control
- **WHEN** the desktop surface requests the overview
- **THEN** the Tauri adapter calls a declared Rust command and the React component does not import or call Tauri APIs

### Requirement: Safe persistence and compatibility
Any Mission Control persistence or indexes SHALL be additive, transactionally migrated, derived from authoritative existing records, and compatible with existing databases and serialized service consumers. Safe summaries MUST exclude prompts, responses, tool inputs, credentials, environment values, unrestricted paths, raw errors, log bodies, and diff contents.

#### Scenario: Existing database is upgraded
- **WHEN** a database created before Mission Control is opened
- **THEN** additive migration succeeds without rewriting or deleting canonical Runs, Sessions, Plans, Loops, Goals, operations, evaluations, reviews, or logs

#### Scenario: Unsafe source data exists
- **WHEN** an owning record contains sensitive or oversized text
- **THEN** projection creation stores only allowlisted bounded classifications or redacted display fields

### Requirement: Compact accessible responsive presentation
Mission Control SHALL use semantic visual tokens and localized text with compact operational density. Futuristic and minimal themes at desktop and narrow widths MUST preserve readable summary, attention, Run lists, detail navigation, focus, loading, disabled, error, unavailable, and terminal states without overlap, clipping, blank panels, or color-only meaning.

#### Scenario: Narrow detail is opened
- **WHEN** Run detail renders at narrow width
- **THEN** detail navigation becomes a bounded tab scroller or selector and the desktop multi-column table does not compress into unreadable columns

#### Scenario: Keyboard user operates actions
- **WHEN** a keyboard or assistive-technology user filters, opens, or acts on a Run
- **THEN** controls expose localized accessible names, stable focus states, and no layout shift

### Requirement: Structural performance evidence
The repository SHALL provide deterministic aggregation, query-plan, event-coalescing, and large-fixture measurements demonstrating bounded page sizes, indexed selection, constant query count per overview request, lazy detail loading, and bounded frontend update batches without fragile shared-runner wall-clock assertions.

#### Scenario: Maximum supported fixture is measured
- **WHEN** the performance suite processes the documented maximum history and event burst fixtures
- **THEN** it satisfies structural query, allocation or item-count, and render-batch budgets and reports the measured evidence

### Requirement: Native Mission Control smoke coverage
The desktop test harness SHALL exercise a minimal real local operation entering canonical Run state and appearing in Mission Control, and results SHALL be reported only for operating systems actually executed.

#### Scenario: Native operation is observed
- **WHEN** the desktop smoke starts a supported local test operation
- **THEN** Mission Control shows its non-terminal state and later reconciles its terminal state through the real desktop service boundary
