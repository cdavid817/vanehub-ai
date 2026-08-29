# agent-mission-control Specification Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
