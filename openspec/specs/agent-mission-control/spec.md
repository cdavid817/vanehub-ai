# agent-mission-control Specification

## Purpose
Provides a bounded, attention-first operational view of canonical Agent Runs while preserving the ownership, security, and detailed workflows of existing Sessions, Plans, Loops, Reviews, Approvals, Evaluations, and logs.
## Requirements
### Requirement: Bounded Mission Control overview
Mission Control SHALL prioritize a compact attention summary, a queryable canonical Run list, and the selected Run detail. Aggregate metrics SHALL remain bounded and SHALL not compete with the primary list and detail for permanent screen space.

#### Scenario: Render the default overview
- **WHEN** Mission Control opens
- **THEN** the page SHALL present actionable attention state and a bounded Run result set before optional aggregate metrics

#### Scenario: Render summary metrics
- **WHEN** reliable counts for attention, active, failed, completed, or other documented categories are available
- **THEN** the page SHALL show a compact summary that can drive the corresponding filter
- **AND** unknown counts SHALL not be shown as zero

#### Scenario: No Run is selected
- **WHEN** the result list has Runs but no explicit selection
- **THEN** the page SHALL select a deterministic first eligible Run or show a clear choose-a-Run state according to the route contract

#### Scenario: No Runs exist
- **WHEN** the canonical query returns no history and no filters are active
- **THEN** the page SHALL show a first-run explanation and routes to supported ways of starting work

#### Scenario: More than one hundred historical Runs exist
- **WHEN** Mission Control loads with more than one hundred historical Runs
- **THEN** the native runtime SHALL return only configured bounded pages and summary counts through indexed queries
- **AND** the number of native queries SHALL NOT grow with the number of returned Runs

#### Scenario: Multiple Agents run concurrently
- **WHEN** Runs belonging to different Agents transition independently
- **THEN** every overview row SHALL reflect its own latest canonical state, owner, title, elapsed state, workspace, phase, attention reason, and verification summary

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
Run detail SHALL provide Overview, Plan and Tasks, Timeline, Verification, Files and Artifacts, Tools, Context, Usage, and Logs as lazy bounded sections with explicit available, unavailable, restricted, loading, and error states.

#### Scenario: Open Run detail
- **WHEN** the user selects a Run
- **THEN** Overview SHALL load first from the bounded summary or detail service
- **AND** other heavy sections SHALL not load until selected or required for an attention decision

#### Scenario: Select an available section
- **WHEN** the owning service reports bounded evidence
- **THEN** the section SHALL render that evidence and next-page or owning-surface links when supported

#### Scenario: Evidence does not exist
- **WHEN** the owning service reports no evidence for a section
- **THEN** the section SHALL render localized unavailable state
- **AND** it SHALL not render generic placeholder text, mock artifacts, or a blank panel

#### Scenario: Evidence is restricted
- **WHEN** the caller lacks access to a section
- **THEN** the section SHALL render restricted state and permitted remediation without exposing content

#### Scenario: Section request fails
- **WHEN** a lazy request fails
- **THEN** only that section SHALL show a retryable safe error
- **AND** Overview, Run list, and other loaded sections SHALL remain usable

#### Scenario: Select Logs
- **WHEN** the Logs section opens
- **THEN** it SHALL request a bounded correlated page from the logging contract
- **AND** it SHALL not load unrelated Run logs or an unbounded body

#### Scenario: Logs tab is selected
- **WHEN** the user selects the Logs section
- **THEN** the page SHALL request a bounded log page correlated to the Run through the existing logging contract, per "Select Logs"
- **AND** it SHALL not load logs for unrelated Runs

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
Mission Control SHALL apply terminal and attention transitions promptly, coalesce high-frequency progress updates, and schedule reconciliation according to route visibility, document focus, connection state, and active Run selection.

#### Scenario: Receive terminal transition
- **WHEN** a visible or background Run reaches a terminal or newly actionable state
- **THEN** the global bounded summary and relevant visible row SHALL update promptly

#### Scenario: Receive high-frequency progress
- **WHEN** many token, usage, phase-progress, or heartbeat events arrive
- **THEN** the frontend SHALL batch presentation updates according to the structural budget

#### Scenario: Hide Mission Control
- **WHEN** the user navigates away or the document becomes hidden
- **THEN** page-owned high-frequency polling and rendering SHALL stop or back off according to lifecycle policy
- **AND** service-owned Run execution SHALL continue

#### Scenario: Return after missed events
- **WHEN** Mission Control becomes visible or reconnects
- **THEN** a bounded canonical reconciliation query SHALL replace stale summary and selected-detail state

#### Scenario: Token events arrive rapidly
- **WHEN** many token, usage, phase-progress, or heartbeat events arrive without a state transition
- **THEN** the dashboard SHALL batch their presentation and SHALL NOT rerender once per token event, per "Receive high-frequency progress"

#### Scenario: Event is missed while unfocused
- **WHEN** the application regains focus after missing a Run transition
- **THEN** a bounded reconciliation query SHALL replace stale summary state with persisted canonical state, per "Return after missed events"

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
Mission Control SHALL use the shared responsive collection and detail layout so Run discovery, attention, detail sections, state-aware actions, unavailable states, and return navigation remain complete without compressing nine detail controls into an unreadable row.

#### Scenario: Render wide layout
- **WHEN** sufficient width is available
- **THEN** the page MAY show Run list and selected detail together
- **AND** detail section navigation SHALL use a readable bounded vertical or horizontal form

#### Scenario: Render compact detail
- **WHEN** the page cannot fit readable list and detail
- **THEN** opening a Run SHALL present detail as the primary surface with a clear Back action
- **AND** section navigation SHALL become a selector, menu, or bounded scroller without clipping

#### Scenario: Operate with keyboard
- **WHEN** a keyboard or assistive-technology user filters, selects, changes detail section, or acts on a Run
- **THEN** focus order, selected state, accessible names, and state explanations SHALL remain stable

#### Scenario: Render both themes
- **WHEN** Mission Control renders in futuristic or minimal
- **THEN** state and hierarchy SHALL be equivalent and SHALL not depend on color alone

#### Scenario: Narrow detail is opened
- **WHEN** Run detail renders at narrow width
- **THEN** detail section navigation SHALL become a bounded selector or scroller, per "Render compact detail", rather than compressing a multi-column table into unreadable columns

#### Scenario: Keyboard user operates actions
- **WHEN** a keyboard or assistive-technology user filters, opens, or acts on a Run
- **THEN** controls SHALL expose localized accessible names, stable focus states, and no layout shift, per "Operate with keyboard"

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

### Requirement: Mission Control performance fixtures cover 100 and 1,000 Runs
Mission Control performance coverage SHALL use deterministic 100-Run and 1,000-Run histories to verify indexed selection, bounded pages, lazy detail loading, a query count independent of result count, safe summaries, and coalesced frontend updates.

#### Scenario: One thousand Runs are listed
- **WHEN** the maximum versioned history is queried and rendered through the existing service boundary
- **THEN** overview query count SHALL remain constant, returned rows SHALL remain page-bounded, detail SHALL remain lazy, and the frontend SHALL NOT create one update per token event

#### Scenario: N plus one regression is introduced
- **WHEN** the performance negative fixture reports query count growing with Run count
- **THEN** the deterministic gate SHALL fail with the query-count baseline, measured value, and budget

### Requirement: Reliable Runner discovery and presentation
Mission Control SHALL expose a safe runner kind, runner capability state, and bounded Local host or SSH profile/host label derived from canonical Run metadata. It SHALL support reliable Local and SSH filtering and MUST NOT infer remote state from workspace text or owner identity.

#### Scenario: Filter Runs by Runner
- **WHEN** the user selects Local or SSH filtering
- **THEN** every returned row has matching persisted Runner metadata and pagination restarts from the first page

#### Scenario: Present Runner identity responsively
- **WHEN** a Run card renders in futuristic or minimal style at desktop or narrow width
- **THEN** its localized runner badge, safe host label, canonical state, attention, and actions remain readable without clipping, layout shift, or color-only meaning

### Requirement: Background and recovery visibility
Mission Control SHALL continue to show a Run after its Session page is no longer visible and SHALL distinguish running, disconnected, reconnecting, interrupted, and attention-required runner outcomes through canonical state plus bounded reason classifications. Open and cancel actions SHALL route to existing owning services.

#### Scenario: Reopen a background Run
- **WHEN** the user navigates away from an active Session and later opens its Mission Control row
- **THEN** Mission Control reconciles persisted canonical state and navigates to the authoritative Session without creating another execution

#### Scenario: Remote connection drops
- **WHEN** an SSH Runner reports disconnect or reconnect exhaustion
- **THEN** the row displays a localized safe runner reason and only actions allowed by canonical state, Runner policy, version, and permissions

### Requirement: Runs destination hierarchy
Mission Control SHALL serve as the attention-first overview within a Runs destination that also exposes Active, History, Loops, and Schedules as localized secondary routes without merging their authoritative services.

#### Scenario: Open Runs
- **WHEN** the user activates the Runs primary destination
- **THEN** the Attention view SHALL open by default when actionable Runs exist
- **AND** the selected secondary route SHALL be represented in the URL

#### Scenario: Open Loops or Schedules
- **WHEN** the user chooses the Loops or Schedules secondary route
- **THEN** the workbench SHALL load the existing owning feature in the Runs destination
- **AND** Mission Control SHALL not duplicate its editor or execution logic

#### Scenario: Return from owning surface
- **WHEN** the user opens a Session, Review, Approval, Loop, Schedule, Log, or Evaluation from a Run
- **THEN** a safe return context SHALL restore the Run selection and supported query state

### Requirement: Mission Control saved views
Mission Control SHALL support versioned local saved views composed from supported status, attention, Agent, project, runner, ordering, and bounded text filters.

#### Scenario: Save current Run query
- **WHEN** the user names the current supported filters and ordering
- **THEN** the view SHALL be stored without sensitive Run evidence and receive a stable local id

#### Scenario: Open saved view
- **WHEN** the user activates a saved view
- **THEN** pagination SHALL restart and only matching canonical summaries SHALL be requested
- **AND** unavailable filter values SHALL be identified rather than silently substituted

#### Scenario: Share a view route
- **WHEN** the query contains only URL-safe supported values
- **THEN** the page MAY expose a copyable route
- **AND** the route SHALL not contain prompts, raw errors, host credentials, log text, or unrestricted paths

### Requirement: Mission Control evidence navigation
Every Mission Control summary and detail evidence reference SHALL identify its owning surface and provide bounded navigation rather than displaying an unactionable raw id.

#### Scenario: Open a Session-owned reference
- **WHEN** a Run summary or detail references a Session, message, change, file, operation, trace, or log
- **THEN** the page SHALL create a validated EvidenceLink to the authoritative Session or evidence surface

#### Scenario: Evidence is unavailable
- **WHEN** the owning service reports no evidence or the identity cannot be resolved
- **THEN** the page SHALL show unavailable state and SHALL not render an active-looking raw link

#### Scenario: Evidence is restricted
- **WHEN** the caller lacks permission
- **THEN** the page SHALL identify restricted status without exposing the protected identifier or content

### Requirement: Mission Control action locality
Run actions SHALL present pending, conflict, success, and failure state in the selected row or detail action region without blocking unrelated Run discovery and inspection.

#### Scenario: Cancel one Run
- **WHEN** a permitted cancellation is submitted
- **THEN** only the target Run's conflicting actions SHALL be disabled while canonical state reconciles

#### Scenario: Action loses a race
- **WHEN** the Run reaches a different canonical terminal or attention state before the mutation commits
- **THEN** the page SHALL retain and explain the returned canonical state
- **AND** it SHALL not restore a stale optimistic state

#### Scenario: Action fails
- **WHEN** an owning service rejects an operation
- **THEN** the target detail SHALL show a safe retryable error while loaded summaries remain visible

