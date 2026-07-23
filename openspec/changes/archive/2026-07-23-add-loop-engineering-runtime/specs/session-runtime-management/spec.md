## ADDED Requirements

### Requirement: Non-activating Loop role sessions
The native session runtime SHALL support creating Loop-owned Worker and Verifier sessions without changing the desktop workflow's active session.

#### Scenario: Create Worker role session
- **WHEN** a Loop iteration enters the acting phase
- **THEN** the session SHALL be persisted with the run id, iteration id, Worker role, stable Agent id, and Loop worktree root
- **AND** the desktop active session id SHALL remain unchanged

#### Scenario: Create Verifier role session
- **WHEN** a Loop iteration enters Agent verification
- **THEN** a separate session SHALL be persisted with the run id, iteration id, Verifier role, stable Agent id, and the same Loop worktree root
- **AND** it SHALL NOT reuse the Worker provider session or generation handle

### Requirement: Loop role session navigation visibility
Loop-owned role sessions SHALL be excluded from normal session navigation by default while remaining directly inspectable from their owning run.

#### Scenario: List normal sessions
- **WHEN** the workspace requests the normal or archived session list
- **THEN** Loop-owned Worker and Verifier sessions SHALL NOT appear unless the request explicitly includes Loop-owned sessions

#### Scenario: Inspect owned session
- **WHEN** the Loop Center requests a role session by its stable id
- **THEN** the existing transcript, files, changes, terminal, logs, report, and usage service behavior SHALL remain available subject to the session root and role policy

### Requirement: Loop role generation isolation and cleanup
Each Loop role session SHALL own its generation process independently and SHALL participate in run pause, cancellation, and terminal cleanup.

#### Scenario: Cancel active Loop role
- **WHEN** a Loop run requests immediate stop while a role generation is active
- **THEN** the runtime SHALL cancel only that owned generation and preserve already persisted messages and evidence

#### Scenario: Complete role generation
- **WHEN** a Worker or Verifier assistant message reaches completed, failed, or cancelled state
- **THEN** the Loop application service SHALL receive exactly one terminal result associated with the role session and iteration

### Requirement: Loop Verifier read-only policy
Verifier role sessions SHALL expose bounded inspection while denying project mutation and mutating shell or Agent tool actions.

#### Scenario: Verifier reads project state
- **WHEN** a Verifier requests a file preview, Git status, diff, transcript, or supplied evidence under the Loop worktree root
- **THEN** the runtime SHALL allow the bounded read-only operation

#### Scenario: Verifier requests mutation
- **WHEN** a Verifier requests a file write, mutating Git action, arbitrary shell process, or Agent action with write capability
- **THEN** the runtime SHALL reject it with a typed policy error and record redacted diagnostics

