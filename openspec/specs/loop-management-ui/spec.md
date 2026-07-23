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

