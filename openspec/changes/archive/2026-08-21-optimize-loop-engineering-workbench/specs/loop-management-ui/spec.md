## ADDED Requirements

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

