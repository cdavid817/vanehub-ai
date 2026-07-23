## ADDED Requirements

### Requirement: Durable Loop definition contract
The system SHALL persist Loop definitions with a stable id, name, enabled state, local Git project path, base branch, goal, acceptance criteria, allowed and protected paths, stable Worker and Verifier Agent ids, structured verification commands, stop limits, version, and timestamps.

#### Scenario: Create valid Loop definition
- **WHEN** a user submits a valid first-phase Loop configuration
- **THEN** the system SHALL return a durable definition with a stable id and version
- **AND** it SHALL preserve stable Agent ids rather than matching display names

#### Scenario: Reject unsupported first-phase scope
- **WHEN** a definition targets a non-Git project, remote workspace, missing Agent, unsafe path scope, or invalid limit
- **THEN** the system SHALL reject the definition without starting an Agent or creating a worktree

### Requirement: Manual bounded Loop start
The first-phase system SHALL start a Loop only through an explicit user action and SHALL snapshot the selected definition before asynchronous work begins.

#### Scenario: Start enabled Loop
- **WHEN** a user manually starts an enabled definition with available role Agents
- **THEN** the system SHALL persist a queued run with an immutable definition snapshot
- **AND** it SHALL return a stable run or operation identifier before variable-duration preparation completes

#### Scenario: Reject concurrent run for definition
- **WHEN** a definition already has a queued, running, paused, or awaiting-acceptance run
- **THEN** the system SHALL reject another start for that definition without creating a second worktree

### Requirement: Fixed Loop execution lifecycle
The native runtime SHALL orchestrate each run through preparing, acting, verifying, deciding, and finalizing phases using explicit durable transitions.

#### Scenario: Execute one iteration
- **WHEN** run preparation succeeds
- **THEN** the runtime SHALL create a Worker iteration session, wait for its terminal result, execute configured checks, obtain Verifier advice, and apply native decision policy

#### Scenario: Continue revision
- **WHEN** decision policy selects revision and all stop limits permit another iteration
- **THEN** the runtime SHALL persist the completed iteration and create the next iteration with bounded prior evidence and remaining limits

### Requirement: Independent Worker and Verifier roles
The runtime SHALL execute Worker and Verifier activity in separate sessions and SHALL enforce read-only Verifier behavior.

#### Scenario: Verify Worker result
- **WHEN** deterministic checks finish for an iteration
- **THEN** the Verifier SHALL receive the immutable goal, acceptance criteria, bounded Git diff, and check evidence in a new session
- **AND** it SHALL return a structured pass, revise, or blocked recommendation with findings

#### Scenario: Prevent Verifier mutation
- **WHEN** the Verifier attempts to write files, execute a mutating project command, or change run state
- **THEN** the runtime SHALL deny that action and record a redacted diagnostic associated with the run

### Requirement: Guarded deterministic verification
The native runtime SHALL execute verification as structured program, argument, working-directory, timeout, and required-policy records without shell command concatenation.

#### Scenario: Run required verification
- **WHEN** an iteration reaches verification
- **THEN** each configured command SHALL execute under the canonical run worktree root with bounded output and timeout
- **AND** the system SHALL persist its exit status, duration, summary, and associated operation id as evidence

#### Scenario: Reject unsafe verification command
- **WHEN** a command resolves outside the run root, uses a disallowed executable, or contains invalid structured arguments
- **THEN** the runtime SHALL reject execution and fail the verification phase with concise user-visible context and detailed unified diagnostics

#### Scenario: Required check fails
- **WHEN** any required verification command fails or times out
- **THEN** the run SHALL NOT enter awaiting acceptance for that iteration

### Requirement: Native decision and stop policy
The native runtime SHALL determine continuation and terminal outcomes from deterministic evidence, Verifier advice, user feedback, and configured hard limits rather than Worker self-assessment alone.

#### Scenario: Automated evidence is acceptable
- **WHEN** all required checks pass and the Verifier does not return revise or blocked
- **THEN** the run SHALL enter awaiting-acceptance rather than marking itself succeeded

#### Scenario: Iteration limit reached
- **WHEN** the run reaches its configured maximum iterations without acceptance-ready evidence
- **THEN** it SHALL terminate as failed with terminal reason `max-iterations`

#### Scenario: Runtime limit reached
- **WHEN** elapsed time, consecutive runtime errors, or another configured hard limit is reached
- **THEN** the run SHALL terminate with the corresponding stable terminal reason and preserve completed evidence

### Requirement: Objective no-progress detection
The runtime SHALL detect consecutive no-progress revisions from Git diff and required-check failure fingerprints.

#### Scenario: Repeated objective state
- **WHEN** consecutive revision decisions have the same diff fingerprint and required-check failure fingerprint without new passing required evidence
- **THEN** the runtime SHALL increment the no-progress count

#### Scenario: No-progress limit reached
- **WHEN** the consecutive no-progress count reaches the configured threshold
- **THEN** the run SHALL terminate as failed with terminal reason `no-progress`

### Requirement: Human acceptance gate
The system SHALL require an explicit user decision before an acceptance-ready run becomes successful.

#### Scenario: Accept result
- **WHEN** a user accepts a run in awaiting-acceptance state
- **THEN** the runtime SHALL mark the run succeeded while preserving its worktree and evidence

#### Scenario: Continue with feedback
- **WHEN** a user submits non-empty continuation feedback and another iteration is permitted
- **THEN** the runtime SHALL persist the feedback and start the next Worker iteration

#### Scenario: Reject result
- **WHEN** a user rejects an awaiting-acceptance run
- **THEN** the runtime SHALL mark it cancelled and preserve the worktree, sessions, and evidence for review

### Requirement: Pause, cancellation, and restart recovery
The runtime SHALL provide phase-boundary pause, immediate cancellation, and conservative recovery for interrupted Loop runs.

#### Scenario: Pause after current step
- **WHEN** a user requests pause during an active child operation
- **THEN** the runtime SHALL reconcile that operation and SHALL NOT schedule the next phase
- **AND** the run SHALL become paused at a durable boundary

#### Scenario: Stop immediately
- **WHEN** a user stops an active run
- **THEN** the runtime SHALL request cancellation of the owned Agent or verification process and mark the reconciled run cancelled

#### Scenario: Recover interrupted run
- **WHEN** application startup finds a nonterminal run without a live in-memory execution lease
- **THEN** the runtime SHALL mark it paused with recovery-required detail
- **AND** it SHALL require explicit resume or cancellation rather than assuming a child process survived

### Requirement: Loop persistence and unified observability
The desktop runtime SHALL persist definitions, run snapshots, iterations, and bounded evidence in SQLite and SHALL route Loop diagnostics and operation output through unified log management.

#### Scenario: Inspect run after restart
- **WHEN** the application restarts after one or more completed iterations
- **THEN** the system SHALL return the same run status, definition snapshot, iteration history, terminal reason, and evidence references

#### Scenario: Persist operation output
- **WHEN** worktree preparation, Worker execution, verification, Verifier execution, cancellation, or recovery emits output or diagnostics
- **THEN** the runtime SHALL associate redacted entries with the Loop run and child operation in the active unified log directory
- **AND** it SHALL NOT create a Loop-specific log file

### Requirement: Loop frontend service parity
Loop management and control SHALL remain behind the frontend Agent service boundary with Tauri and Web/mock adapter parity.

#### Scenario: React manages Loop state
- **WHEN** React lists, creates, edits, starts, monitors, pauses, resumes, cancels, accepts, continues, or rejects a Loop
- **THEN** it SHALL call the frontend service interface and SHALL NOT call Tauri `invoke()` directly

#### Scenario: Web runtime simulates Loop lifecycle
- **WHEN** the Loop Center runs through the Web/mock adapter
- **THEN** it SHALL expose contract-equivalent definitions, asynchronous phase transitions, iterations, evidence, controls, and terminal outcomes without local Git, SQLite, or Agent CLIs

