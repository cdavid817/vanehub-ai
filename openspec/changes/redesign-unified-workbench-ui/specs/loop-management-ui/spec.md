# loop-management-ui Specification Delta

## ADDED Requirements

### Requirement: Loop definitions and runs route separation
The Runs destination SHALL expose Loop Definitions and Loop Runs as distinct route states while retaining direct navigation between a definition and its runs.

#### Scenario: Open Loop Definitions
- **WHEN** the user opens the Loops section without a Run id
- **THEN** the page SHALL show a bounded definition list and selected definition overview rather than mixing every Run into the same undifferentiated list

#### Scenario: Open Loop Runs
- **WHEN** the user chooses the Runs view or follows a Run deep link
- **THEN** the list SHALL show canonical Loop Run summaries and preserve definition filtering when requested

#### Scenario: Open a legacy Loop route
- **WHEN** a supported legacy Loop route is used
- **THEN** the router SHALL map it to the equivalent definition or Run route and retain a safe return context

### Requirement: Loop iteration Inspector selection
Loop phases, iterations, verification outcomes, findings, changed-file summaries, operations, and evidence links SHALL be selectable and drive the shared Inspector.

#### Scenario: Select an iteration
- **WHEN** the user activates an iteration summary
- **THEN** the Inspector SHALL show bounded Worker, change, verification, Verifier, decision, feedback, and recovery detail for that stable iteration

#### Scenario: Select evidence
- **WHEN** the user chooses a Session, file, change, operation, trace, or log reference
- **THEN** the Inspector or EvidenceLink SHALL open the owning bounded surface with validated scope

#### Scenario: Pin iteration detail
- **WHEN** the user pins an iteration in the Inspector
- **THEN** later timeline selection SHALL not replace it until unpinned

### Requirement: Loop local mutation feedback
Loop definition and Run actions SHALL disable only conflicting controls and preserve loaded definition, timeline, iteration, and evidence state while the owning service reconciles.

#### Scenario: Pause or resume
- **WHEN** a permitted Run action is pending
- **THEN** the action region SHALL show pending state and prevent duplicate submission
- **AND** the timeline and evidence SHALL remain readable

#### Scenario: Edit a definition
- **WHEN** a definition update is pending
- **THEN** the selected definition and unrelated definitions SHALL remain visible
- **AND** a version conflict SHALL preserve the draft and show canonical changes

#### Scenario: Action fails
- **WHEN** the service rejects a start, pause, stop, resume, accept, continue, reject, duplicate, enable, or delete action
- **THEN** the failure SHALL appear near the affected action with a safe explanation and retry path when valid

## MODIFIED Requirements

### Requirement: Dedicated Loop Center
The workbench SHALL provide Loops as a dedicated section of the Runs destination for managing Loop Definitions and Loop Runs without presenting Loop execution as a normal chat tab or scheduled-task editor.

#### Scenario: Open Loops
- **WHEN** the user selects Loops under Runs
- **THEN** the workspace SHALL show the definition or Run route, bounded contextual navigation, and the selected overview or timeline

#### Scenario: No definitions exist
- **WHEN** the definition collection is empty
- **THEN** the page SHALL explain Loop purpose and expose the guided creation action

#### Scenario: Return to Runs attention
- **WHEN** a Loop Run has actionable or failed canonical state
- **THEN** Mission Control MAY surface its safe summary and navigate to the authoritative Loop Run page

### Requirement: Loop Center operational layout
The Loop Center SHALL use a flexible two-region primary layout with bounded contextual navigation and a readable definition overview or Run timeline, while detailed selection content uses the shared optional Inspector.

#### Scenario: Render desktop Run
- **WHEN** sufficient width is available
- **THEN** the Loop list or contextual navigation and the main timeline SHALL remain readable
- **AND** the Inspector SHALL be collapsible or inline only when the main surface remains above its minimum width

#### Scenario: Render compact Run
- **WHEN** the page cannot fit navigation and timeline together
- **THEN** navigation and Inspector SHALL become accessible sheets
- **AND** critical Run state and actions SHALL remain available without opening the Inspector

#### Scenario: Resize content
- **WHEN** pane width changes
- **THEN** timeline labels, phase state, iteration summaries, and decision actions SHALL not overlap or become unreachable

### Requirement: Run phase and iteration monitoring
The selected Loop Run view SHALL expose current status, phase, progress, limits, compact iteration history, evidence availability, and decision reasons while progressively disclosing low-level evidence through selection and the Inspector.

#### Scenario: Monitor active Run
- **WHEN** a Run is queued, running, paused, or recovery-required
- **THEN** the header and phase stepper SHALL show current activity, iteration, elapsed or remaining budget, and state-appropriate actions

#### Scenario: Review iteration summary
- **WHEN** an iteration exists
- **THEN** the timeline SHALL show outcome, material change, required verification, Verifier recommendation, decision reason, and recovery indicator in a bounded summary

#### Scenario: Inspect iteration detail
- **WHEN** the user opens an iteration
- **THEN** complete bounded detail and owned evidence links SHALL appear in the Inspector or an explicit detail surface rather than expanding every iteration inline

#### Scenario: Refresh Run
- **WHEN** updated state is loading
- **THEN** existing timeline and selected detail SHALL remain visible with a refresh indicator

### Requirement: Persistent Loop run action header
The selected Loop Run surface SHALL keep a bounded canonical status, phase, current activity, budget summary, one state-appropriate primary action, and a secondary action menu visible independently of the Inspector.

#### Scenario: Run is active
- **WHEN** pause or stop is permitted
- **THEN** the header SHALL show the most likely state-appropriate primary action and place other permitted actions in a grouped menu with consequences

#### Scenario: Run awaits acceptance
- **WHEN** the canonical state is awaiting acceptance
- **THEN** the header SHALL lead to the focused decision panel rather than duplicating all decision controls

#### Scenario: Run is terminal
- **WHEN** the Run completed, failed, cancelled, or was rejected
- **THEN** the header SHALL show terminal time and available inspection, retry, duplicate, or navigation actions supported by the owning service

#### Scenario: Use compact width
- **WHEN** the header cannot fit all summaries
- **THEN** secondary metrics SHALL collapse into a details trigger while state and the primary action remain readable

### Requirement: Decision-oriented iteration history
The Loop Center SHALL summarize iterations as a compact decision timeline and SHALL place full chronological evidence behind explicit selection rather than rendering a large accordion for every iteration by default.

#### Scenario: Compare consecutive iterations
- **WHEN** a Run has at least two iterations
- **THEN** later iteration summaries SHALL identify material verification and change-count deltas when available
- **AND** objective no-progress state SHALL be visible

#### Scenario: Expand evidence
- **WHEN** the user requests complete iteration evidence
- **THEN** the owning bounded evidence page SHALL load on demand
- **AND** the default timeline SHALL not duplicate that full content

#### Scenario: Recover interrupted Run
- **WHEN** an iteration stopped at a durable recovery boundary
- **THEN** the summary SHALL explain preserved evidence and available resume or stop actions

### Requirement: Decision-ready human acceptance panel
An awaiting-acceptance Loop Run SHALL present one focused sticky or primary decision panel that relates acceptance criteria, required verification outcomes, Verifier advice, change summary, risks, remaining budget, and consequences of Accept, Continue, and Reject.

#### Scenario: Review decision
- **WHEN** a Run enters awaiting acceptance
- **THEN** the panel SHALL show each criterion as evidence-backed, failed, or not evaluated and summarize the latest relevant iteration

#### Scenario: Accept
- **WHEN** the user accepts and the action is permitted
- **THEN** only decision controls SHALL lock while the mutation reconciles
- **AND** prior timeline and evidence SHALL remain visible

#### Scenario: Continue with feedback
- **WHEN** non-empty feedback is provided and another iteration is allowed
- **THEN** the panel SHALL show the remaining budget and submit through the Loop service
- **AND** prior evidence SHALL be retained

#### Scenario: Continuation is unavailable
- **WHEN** iteration, runtime, no-progress, policy, or permission limits prevent another iteration
- **THEN** Continue SHALL be absent or disabled with a specific accessible explanation
- **AND** Accept and Reject SHALL remain according to domain rules

#### Scenario: Reject
- **WHEN** the user confirms rejection
- **THEN** the action SHALL not delete the worktree, Sessions, artifacts, or Run history unless a separate explicit cleanup action says so
