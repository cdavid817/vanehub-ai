# loop-management-ui Specification

## Purpose
TBD - created by archiving change add-loop-engineering-runtime. Update Purpose after archive.
## Requirements
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

#### Scenario: Open Loop Center
- **WHEN** the user selects Loops under the Runs destination
- **THEN** the workspace SHALL show the Loop definition or Run list, selected overview or timeline, and optional Inspector, per "Open Loops"

#### Scenario: Render empty state
- **WHEN** no Loop definitions exist
- **THEN** the Loops page SHALL show a localized empty state with a create-Loop action, per "No definitions exist"

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

#### Scenario: Render desktop layout
- **WHEN** sufficient desktop width is available
- **THEN** the Loop Center SHALL render bounded contextual navigation, a flexible timeline, and an optional Inspector with aligned heights and internal scrolling, per "Render desktop Run" — the fixed approximately-240px/300px three-panel grid is replaced by the shared bounded/collapsible model

#### Scenario: Render narrow layout
- **WHEN** the viewport cannot fit navigation, timeline, and Inspector without clipping
- **THEN** navigation and Inspector SHALL become accessible sheets while the timeline remains the primary surface, per "Render compact Run"

### Requirement: Guided Loop creation
The Loop Center SHALL provide a four-step creation flow for goal and scope, role Agents, verification and limits, and final review.

#### Scenario: Configure goal and scope
- **WHEN** the first step renders
- **THEN** it SHALL let the user select a known local Git project and base branch and enter name, goal, acceptance criteria, allowed paths, and protected paths

#### Scenario: Configure role Agents
- **WHEN** the Agent step renders
- **THEN** it SHALL let the user choose available Worker and Verifier Agents by stable id and show their selected identity clearly

#### Scenario: Configure verification and limits
- **WHEN** the verification step renders
- **THEN** it SHALL provide repeatable structured command rows and controls for iteration, step-timeout, total-runtime, runtime-error, and no-progress limits

#### Scenario: Review before save or start
- **WHEN** the review step renders
- **THEN** it SHALL summarize the goal, scope, Agents, worktree behavior, commands, limits, and mandatory human acceptance before allowing save or save-and-run

### Requirement: Structured verification command editor
The UI SHALL edit verification commands as discrete program, arguments, relative working directory, timeout, and required controls.

#### Scenario: Add verification command
- **WHEN** a user adds a command row
- **THEN** the UI SHALL provide structured fields without requiring shell-script concatenation

#### Scenario: Validate command row
- **WHEN** a command lacks a program, has an invalid timeout, or uses an absolute or escaping working directory
- **THEN** the UI SHALL show a localized validation error and SHALL NOT submit the definition

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

#### Scenario: Monitor active run
- **WHEN** a run is queued, running, paused, or recovery-required
- **THEN** the center SHALL show its current phase, iteration position, elapsed time, configured limits, and latest decision or operation status, per "Monitor active Run"

#### Scenario: Inspect iteration
- **WHEN** a user opens an iteration
- **THEN** the UI SHALL show Worker summary, changed-file and diff summary, verification outcomes, Verifier recommendation and findings, decision reason, and links to owned session inspection surfaces, per "Inspect iteration detail"

#### Scenario: Preserve loaded history during refresh
- **WHEN** updated run state is loading
- **THEN** the UI SHALL retain existing iteration history and indicate refreshing rather than replacing the center with a blank state, per "Refresh Run"

### Requirement: Loop run controls
The Loop Center SHALL expose controls appropriate to the selected run state with stable dimensions and explicit consequences.

#### Scenario: Pause active run
- **WHEN** the user activates pause on a running Loop
- **THEN** the UI SHALL explain that pause occurs after the current step and request pause through the service boundary

#### Scenario: Stop active run
- **WHEN** the user activates stop on an active Loop and confirms
- **THEN** the UI SHALL request immediate cancellation through the service boundary and keep visible evidence while cancellation reconciles

#### Scenario: Resume recoverable run
- **WHEN** a run is paused or recovery-required and can resume
- **THEN** the UI SHALL provide a resume action and show the phase boundary from which execution will continue

### Requirement: Human acceptance interactions
An awaiting-acceptance run SHALL present acceptance, feedback-and-continue, and rejection controls alongside the evidence needed to decide.

#### Scenario: Accept reviewed result
- **WHEN** the user accepts an awaiting run
- **THEN** the UI SHALL mark the service mutation as pending, prevent duplicate submission, and render the resulting succeeded state

#### Scenario: Continue with feedback
- **WHEN** the user enters non-empty feedback and requests another iteration
- **THEN** the UI SHALL submit that feedback and render the next queued or running iteration without discarding prior evidence

#### Scenario: Reject reviewed result
- **WHEN** the user chooses rejection and confirms
- **THEN** the UI SHALL request rejection without deleting the worktree or run history

### Requirement: Loop result preserves project inspection access
The Loop Center SHALL link Loop iterations and results to existing session and project inspection surfaces.

#### Scenario: Open Loop changes
- **WHEN** a user chooses to inspect changed files or diffs for a run
- **THEN** the workspace SHALL open the existing bounded Changes or Files experience for the Loop worktree or owned role session

#### Scenario: Open execution evidence
- **WHEN** a user chooses a Worker, Verifier, operation, terminal, or log reference
- **THEN** the workspace SHALL open the corresponding existing inspection surface without adding the role session to normal navigation by default

### Requirement: Localized and theme-compatible Loop UI
All Loop Center visible text and states SHALL support synchronized Simplified Chinese and English resources and both registered visual styles.

#### Scenario: Render localized Loop UI
- **WHEN** Loop definition, creation, monitoring, evidence, confirmation, validation, empty, loading, or error UI renders
- **THEN** all frontend-owned visible text, accessible names, and tooltips SHALL use the active locale

#### Scenario: Render both visual styles
- **WHEN** the Loop Center renders in `futuristic` or `minimal` style
- **THEN** it SHALL use semantic tokens, compact operational density, stable controls, 8px-or-less radii, and internal scrolling without overlap or clipping

### Requirement: Web/mock Loop clarity
The Web/mock Loop Center SHALL preserve the complete interaction contract while clearly identifying simulated execution where runtime truth matters.

#### Scenario: Monitor simulated run
- **WHEN** a user starts a Loop in Web/mock mode
- **THEN** the UI SHALL progress through representative asynchronous phases and evidence through the same service calls
- **AND** it SHALL not imply that local Git files or Agent CLIs were actually executed

### Requirement: Loop Center first-run state
The Loop Center SHALL present an explanatory empty state with a primary creation action when no Loop definition exists.

#### Scenario: No Loop definitions exist
- **WHEN** the Loop Center opens and the definition list is empty
- **THEN** it SHALL present an icon, a title, and an explanation of what a Loop definition is
- **AND** it SHALL present a primary action that starts Loop creation

#### Scenario: Creation remains reachable once definitions exist
- **WHEN** at least one Loop definition exists
- **THEN** the definition list SHALL continue to expose its creation control
- **AND** the empty state SHALL NOT be rendered

#### Scenario: Inspector reflects the empty state
- **WHEN** the Loop Center has no definitions and therefore no selectable run
- **THEN** the inspector SHALL state that no run is available rather than presenting an empty panel with no explanation

#### Scenario: Localized empty state
- **WHEN** the Loop Center empty state renders
- **THEN** its title, explanation, and primary action label SHALL use the active application language

### Requirement: Loop definition operational overview
The Loop Center SHALL present a selected definition as an actionable overview when no run is selected, including its goal, acceptance criteria, path scope, role Agents, verification policy, limits, recent run outcomes, enabled state, and primary start action.

#### Scenario: Select definition without a run
- **WHEN** the user selects a Loop definition and no run is selected
- **THEN** the center surface SHALL render the definition overview instead of a generic empty-run message
- **AND** an enabled definition with no active run SHALL expose a primary start action

#### Scenario: Manage an existing definition
- **WHEN** a Loop definition is selected
- **THEN** the UI SHALL expose edit, duplicate, enable or disable, and guarded delete actions
- **AND** it SHALL explain and prevent any action that conflicts with an active run

#### Scenario: Duplicate a definition
- **WHEN** the user duplicates a Loop definition
- **THEN** the system SHALL create a disabled copy with a new stable id and version while preserving the source configuration
- **AND** it SHALL require a distinct user-visible name before the copy can be enabled or started

### Requirement: Loop configuration uses discovered project context
The Loop editor SHALL obtain project and branch choices through the frontend service boundary and SHALL distinguish desktop discovery from Web/mock simulation.

#### Scenario: Select a desktop project and branch
- **WHEN** the desktop Loop editor configures goal and scope
- **THEN** it SHALL offer known local Git projects and branches discovered without starting an interactive Agent session
- **AND** it SHALL preserve the canonical project path and branch reference in the saved definition

#### Scenario: Configure a simulated Web Loop
- **WHEN** the Web/mock Loop editor configures goal and scope
- **THEN** it SHALL offer clearly identified simulated project and branch choices through the same service contract
- **AND** it SHALL NOT imply that a local repository was inspected

#### Scenario: Preserve an unavailable saved selection
- **WHEN** an existing definition references a project or branch that discovery no longer returns
- **THEN** the editor SHALL retain and identify the saved value as unavailable
- **AND** it SHALL require remediation before the definition can be started

### Requirement: Loop start readiness preflight
The Loop Center SHALL run a non-launching readiness preflight before starting a definition and SHALL report readiness for the project, base branch, Worker and Verifier eligibility, structured verification commands, path scope, and active-run constraint.

#### Scenario: Preflight passes
- **WHEN** every required readiness check passes
- **THEN** the UI SHALL show a ready result and SHALL allow the user to confirm start
- **AND** the preflight SHALL NOT create a run, worktree, or Agent session

#### Scenario: Preflight finds a blocking issue
- **WHEN** a required readiness check fails
- **THEN** the UI SHALL identify the failed check, explain the cause, and offer an actionable remediation when one exists
- **AND** it SHALL prevent start without discarding the loaded definition or recent run history

#### Scenario: Readiness changes before start commits
- **WHEN** readiness passed but the authoritative start operation detects a conflicting active run or newly unavailable dependency
- **THEN** start SHALL remain rejected by the native runtime
- **AND** the UI SHALL refresh readiness and present the authoritative failure without creating a partial run

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

#### Scenario: Monitor and control a desktop run
- **WHEN** a run is selected at desktop width
- **THEN** the header SHALL show status, phase, current iteration, elapsed or remaining budget, and the current activity summary, per "Run is active"
- **AND** pause, resume, stop, or acceptance actions appropriate to the state SHALL remain reachable from that header or an adjacent persistent action region

#### Scenario: Control a run at narrow width
- **WHEN** the Loop Center uses its compact composition
- **THEN** critical run controls SHALL remain reachable without first opening the Inspector, per "Use compact width"
- **AND** controls SHALL not overlap the timeline or hide state and consequence text

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

#### Scenario: Inspect low-level evidence
- **WHEN** the user requests full evidence for an iteration
- **THEN** the UI SHALL progressively disclose its chronological evidence and inspection links, per "Expand evidence"
- **AND** it SHALL not repeat the same full evidence list in the default summary

#### Scenario: Recover from an interrupted run
- **WHEN** a run is paused with recovery-required detail
- **THEN** the selected run surface SHALL explain the durable boundary, preserved evidence, and available resume or stop actions, per "Recover interrupted Run"

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

#### Scenario: Review an acceptance-ready result
- **WHEN** a run enters awaiting-acceptance
- **THEN** the primary run surface SHALL show the evidence needed to accept, continue, or reject without requiring the user to assemble the decision from separate panels, per "Review decision"
- **AND** each acceptance criterion SHALL be shown with an evidence-backed or not-evaluated state rather than inferred as passed without evidence

#### Scenario: Continue with feedback from the decision panel
- **WHEN** the user supplies non-empty feedback and another iteration is permitted
- **THEN** the decision panel SHALL submit continuation through the Loop service boundary and retain all prior evidence while the next iteration is queued, per "Continue with feedback"

#### Scenario: Iteration budget prevents continuation
- **WHEN** the run has exhausted its maximum iteration budget
- **THEN** the decision panel SHALL disable continuation, explain the exhausted limit, and keep accept and reject actions available, per "Continuation is unavailable"

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

