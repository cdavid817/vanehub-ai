# loop-management-ui Specification

## Purpose
TBD - created by archiving change add-loop-engineering-runtime. Update Purpose after archive.
## Requirements
### Requirement: Dedicated Loop Center
The workspace SHALL provide a dedicated Loop Center for managing definitions and runs without presenting Loop execution as a normal chat tab or scheduled-task dialog.

#### Scenario: Open Loop Center
- **WHEN** the user activates the Loops activity entry
- **THEN** the workspace SHALL show the Loop definition and run list, selected run timeline, and configuration or control inspector

#### Scenario: Render empty state
- **WHEN** no Loop definitions exist
- **THEN** the Loop Center SHALL show a localized empty state with a create-Loop action

### Requirement: Loop Center operational layout
The Loop Center SHALL use a compact three-panel desktop layout with a bounded list, flexible timeline, and bounded inspector.

#### Scenario: Render desktop layout
- **WHEN** sufficient desktop width is available
- **THEN** the Loop Center SHALL render an approximately 240px list, flexible center content, and approximately 300px inspector with aligned heights and internal scrolling

#### Scenario: Render narrow layout
- **WHEN** the viewport cannot fit all three panels without clipping
- **THEN** the definition list and inspector SHALL become accessible drawers while the timeline remains the primary surface

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
The selected run view SHALL expose current status, phase, progress, limits, iterations, evidence, and decision reasons without requiring the user to inspect raw logs.

#### Scenario: Monitor active run
- **WHEN** a run is queued, running, or paused
- **THEN** the center SHALL show its current phase, iteration position, elapsed time, configured limits, and latest decision or operation status

#### Scenario: Inspect iteration
- **WHEN** a user expands an iteration
- **THEN** the UI SHALL show Worker summary, changed-file and diff summary, verification outcomes, Verifier recommendation and findings, decision reason, and links to owned session inspection surfaces

#### Scenario: Preserve loaded history during refresh
- **WHEN** updated run state is loading
- **THEN** the UI SHALL retain existing iteration history and indicate refreshing rather than replacing the center with a blank state

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
The selected run surface SHALL keep its status, current activity, budget summary, and state-appropriate primary controls visible independently of the secondary inspector.

#### Scenario: Monitor and control a desktop run
- **WHEN** a run is selected at desktop width
- **THEN** the center header SHALL show status, phase, current iteration, elapsed or remaining budget, and the current activity summary
- **AND** pause, resume, stop, or acceptance actions appropriate to the state SHALL remain reachable from that header or an adjacent persistent action region

#### Scenario: Control a run at narrow width
- **WHEN** the Loop Center uses its narrow layout
- **THEN** critical run controls SHALL remain reachable without first opening the inspector drawer
- **AND** controls SHALL not overlap the timeline or hide state and consequence text

### Requirement: Decision-oriented iteration history
The Loop Center SHALL summarize each iteration by outcome, change from the previous iteration, required verification results, Verifier recommendation, decision reason, user feedback, and relevant recovery guidance before exposing low-level evidence details.

#### Scenario: Compare consecutive iterations
- **WHEN** a run has two or more iterations
- **THEN** each later iteration SHALL identify material change from its predecessor, including resolved or newly failing required checks and available change-count deltas
- **AND** it SHALL identify an objective no-progress determination when present

#### Scenario: Inspect low-level evidence
- **WHEN** the user requests full evidence for an iteration
- **THEN** the UI SHALL progressively disclose its chronological evidence and inspection links
- **AND** it SHALL not repeat the same full evidence list in the default summary

#### Scenario: Recover from an interrupted run
- **WHEN** a run is paused with recovery-required detail
- **THEN** the selected run surface SHALL explain the durable boundary, preserved evidence, and available resume or stop actions

### Requirement: Decision-ready human acceptance panel
An awaiting-acceptance run SHALL present one focused decision surface that relates acceptance criteria, required verification outcomes, Verifier advice and findings, change summary, known risks, and the consequences of each human action.

#### Scenario: Review an acceptance-ready result
- **WHEN** a run enters awaiting-acceptance
- **THEN** the primary run surface SHALL show the evidence needed to accept, continue, or reject without requiring the user to assemble the decision from separate panels
- **AND** each acceptance criterion SHALL be shown with an evidence-backed or not-evaluated state rather than inferred as passed without evidence

#### Scenario: Continue with feedback from the decision panel
- **WHEN** the user supplies non-empty feedback and another iteration is permitted
- **THEN** the decision panel SHALL submit continuation through the Loop service boundary and retain all prior evidence while the next iteration is queued

#### Scenario: Iteration budget prevents continuation
- **WHEN** the run has exhausted its maximum iteration budget
- **THEN** the decision panel SHALL disable continuation, explain the exhausted limit, and keep accept and reject actions available

