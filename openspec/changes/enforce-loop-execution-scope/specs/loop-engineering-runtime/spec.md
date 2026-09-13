## MODIFIED Requirements

### Requirement: Durable Loop definition contract
The system SHALL persist Loop definitions with a stable id, name, enabled state, local Git project path, base branch, goal, acceptance criteria, allowed and protected paths, stable Worker and Verifier Agent ids, structured verification commands, stop limits, version, timestamps, versioned mutation scope, and explicitly selected requested enforcement mode. New definitions SHALL default to preventive-required. CLI or trusted-API role eligibility SHALL remain necessary but SHALL NOT imply execution-scope capability.

#### Scenario: Create valid Loop definition
- **WHEN** a user submits a valid first-phase Loop configuration
- **THEN** the system SHALL return a durable definition with a stable id and version
- **AND** it SHALL preserve stable Agent ids rather than matching display names

#### Scenario: Reject unsupported first-phase scope
- **WHEN** a definition targets a non-Git project, remote workspace, missing Agent, unsafe path scope, or invalid limit
- **THEN** the system SHALL reject the definition without starting an Agent or creating a worktree

#### Scenario: Accept a CLI-launched Worker or Verifier Agent
- **WHEN** a definition names a Worker or Verifier Agent that supports CLI interaction
- **THEN** the system SHALL accept that Agent for the role, unchanged from existing behavior

#### Scenario: Accept a trusted API Worker or Verifier Agent
- **WHEN** a definition names a Worker or Verifier Agent that only supports API interaction and has tool-use trust enabled
- **THEN** the system SHALL accept that Agent for the role

#### Scenario: Reject an untrusted API Worker or Verifier Agent
- **WHEN** a definition names a Worker or Verifier Agent that only supports API interaction and does not have tool-use trust enabled
- **THEN** the system SHALL reject the definition with an error identifying that the Agent requires tool-use trust
- **AND** it SHALL NOT start an Agent or create a worktree

#### Scenario: Preserve role eligibility separately from launch capability
- **WHEN** a role Agent is eligible for selection but cannot enforce the requested runtime scope
- **THEN** the definition MAY retain that stable Agent selection, but authoritative start SHALL remain blocked until the requested mode and all hard role constraints can be satisfied

### Requirement: Manual bounded Loop start
The first-phase system SHALL start a Loop only through an explicit user action and SHALL snapshot the selected definition before asynchronous work begins. Readiness and authoritative start SHALL use the same native scope and execution-capability validator; readiness MUST NOT launch or create a run/worktree/session, and start MUST reload and revalidate current state before its atomic queued-run commit. Start SHALL reject known scope, capability or acknowledgement blockers before queuing. Worktree-dependent binding SHALL finish during preparation before Agent or verification effects.

#### Scenario: Start enabled Loop
- **WHEN** a user manually starts an enabled definition with available role Agents, valid scope, satisfied execution capability and any required current acknowledgement
- **THEN** the system SHALL persist a queued run with an immutable definition snapshot
- **AND** it SHALL return a stable run or operation identifier before variable-duration preparation completes

#### Scenario: Reject concurrent run for definition
- **WHEN** a definition already has a queued, running, paused, or awaiting-acceptance run
- **THEN** the system SHALL reject another start for that definition without creating a second worktree

#### Scenario: Reject start when a role Agent is no longer eligible
- **WHEN** a user starts a definition whose Worker or Verifier Agent no longer supports CLI or trusted API interaction (for example, tool-use trust was disabled after the definition was saved)
- **THEN** the system SHALL reject the start with an error identifying the ineligible Agent
- **AND** it SHALL NOT persist a queued run or create a worktree

#### Scenario: Bypass UI preflight with an invalid scope
- **WHEN** a caller directly invokes start with an invalid or unsupported scope or stale capability assessment
- **THEN** native start SHALL reject without creating a queued run, worktree or role session

### Requirement: Independent Worker and Verifier roles
The runtime SHALL execute Worker and Verifier activity in separate sessions and SHALL enforce read-only Verifier behavior before effects across every enabled mutation surface in both requested modes. Role sessions SHALL carry trusted run scope ownership. A Verifier whose unmediated channels cannot be disabled or proven read-only MUST NOT execute.

#### Scenario: Verify Worker result
- **WHEN** deterministic checks finish for an iteration
- **THEN** the Verifier SHALL receive the immutable goal, acceptance criteria, bounded Git diff, and check evidence in a new session
- **AND** it SHALL return a structured pass, revise, or blocked recommendation with findings

#### Scenario: Prevent Verifier mutation
- **WHEN** the Verifier attempts to write files, execute a mutating project command, or change run state
- **THEN** the runtime SHALL deny that action and record a redacted diagnostic associated with the run

#### Scenario: Read-only prompt is the only Verifier protection
- **WHEN** a selected Verifier can mutate through unguarded internal tools, commands or MCP despite a read-only prompt
- **THEN** the runtime SHALL reject that execution combination rather than rely on post-run diff detection

### Requirement: Guarded deterministic verification
The native runtime SHALL execute verification as structured program, argument, working-directory, timeout, and required-policy records without shell command concatenation, using a versioned kind of process or native-check. Existing records without a kind SHALL decode as process. Commands and all descendants SHALL additionally satisfy the bound scope, current permissions and requested mode; executable allowlists and cwd alone MUST NOT establish confinement. Reused verification evidence MUST match command definition, workspace/input fingerprint, scope digest, policy revision and executor witness.

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

#### Scenario: Verification reuses an obsolete command id
- **WHEN** a command id matches old evidence but command content, inputs, scope, policy revision or executor witness differs
- **THEN** the runtime SHALL NOT reuse that evidence as a passing current verification

### Requirement: Native decision and stop policy
The native runtime SHALL determine continuation and terminal outcomes from deterministic evidence, Verifier advice, user feedback, and configured hard limits rather than Worker self-assessment alone. Complete, current and violation-free scope evidence SHALL be required for acceptance readiness. Confirmed scope violation SHALL fail with scope-violation; incomplete scope evidence SHALL pause with scope-unverifiable and SHALL preserve artifacts.

#### Scenario: Automated evidence is acceptable
- **WHEN** all required checks pass, the Verifier does not return revise or blocked, and current complete scope evidence contains no violation
- **THEN** the run SHALL enter awaiting-acceptance rather than marking itself succeeded

#### Scenario: Iteration limit reached
- **WHEN** the run reaches its configured maximum iterations without acceptance-ready evidence
- **THEN** it SHALL terminate as failed with terminal reason `max-iterations`

#### Scenario: Runtime limit reached
- **WHEN** elapsed time, consecutive runtime errors, or another configured hard limit is reached
- **THEN** the run SHALL terminate with the corresponding stable terminal reason and preserve completed evidence

#### Scenario: Functional checks pass but scope evidence does not
- **WHEN** required checks and Verifier advice pass but scope evidence is violated or unverifiable
- **THEN** the runtime SHALL fail or pause according to the scope result and SHALL NOT enter awaiting-acceptance

### Requirement: Human acceptance gate
The system SHALL require an explicit user decision before an acceptance-ready run becomes successful. Every acceptance entry point SHALL perform the native sealed-evidence gate defined by loop-execution-scope through an asynchronous operation; user approval MUST NOT bypass it.

#### Scenario: Accept result
- **WHEN** a user accepts a run in awaiting-acceptance state and the native sealed-evidence gate succeeds
- **THEN** the runtime SHALL mark the run succeeded while preserving its worktree and evidence

#### Scenario: Continue with feedback
- **WHEN** a user submits non-empty continuation feedback, another iteration is permitted, and the unchanged scope/mode remains valid with any required renewed audit acknowledgement
- **THEN** the runtime SHALL persist the feedback and start the next Worker iteration

#### Scenario: Reject result
- **WHEN** a user rejects an awaiting-acceptance run
- **THEN** the runtime SHALL mark it cancelled and preserve the worktree, sessions, and evidence for review

#### Scenario: Direct acceptance has stale evidence
- **WHEN** any caller attempts acceptance with stale, violated, missing or incomplete scope evidence
- **THEN** the runtime SHALL reject the acceptance without marking the Loop or canonical Run succeeded

### Requirement: Pause, cancellation, and restart recovery
The runtime SHALL provide phase-boundary pause, immediate cancellation, and conservative recovery for interrupted Loop runs by consuming the shared recovery projection of each owned Worker or Verifier session. Resume and continuation SHALL revalidate immutable scope binding, root identity, current policy, executor capability and evidence before clearing blockers or scheduling effects. Missing binding, unverifiable evidence and changed capability SHALL use supported paused reasons scope-binding-missing, scope-unverifiable and scope-capability-changed. Recovery MUST NOT silently downgrade mode or replay uncertain side effects; artifact-audited recovery SHALL require fresh explicit human acknowledgement within the original upper bound.

#### Scenario: Pause after current step
- **WHEN** a user requests pause during an active child operation
- **THEN** the runtime SHALL reconcile that operation and SHALL NOT schedule the next phase
- **AND** the run SHALL become paused at a durable boundary

#### Scenario: Stop immediately
- **WHEN** a user stops an active run
- **THEN** the runtime SHALL request cancellation of the owned Agent or verification process and mark the reconciled run cancelled

#### Scenario: Recover a conclusive role session
- **WHEN** startup session reconciliation reports a conclusive terminal outcome for the active Worker or Verifier execution run
- **THEN** the Loop runtime SHALL project that shared outcome to the owning iteration without deriving a conflicting result from message timestamps or diagnostic telemetry

#### Scenario: Recover interrupted run
- **WHEN** application startup finds a nonterminal run whose owned session recovery is action-required, quarantined, or otherwise inconclusive
- **THEN** the runtime SHALL mark it paused with recovery-required detail
- **AND** it SHALL require explicit resume or cancellation rather than assuming a child process survived or redispatching the interrupted role

#### Scenario: Resume after capability drift
- **WHEN** a paused run no longer has valid enforcement witnesses for its frozen requested mode
- **THEN** resume SHALL retain an actionable blocker and SHALL NOT clear the reason, change mode or launch work

### Requirement: Loop frontend service parity
Loop management and control SHALL remain behind the frontend Agent service boundary with Tauri and Web/mock adapter parity, including versioned scope, requested mode, assessments, typed failures, approval context and asynchronous acceptance evidence. The native runtime SHALL own real filesystem and process authority; Web/mock values MUST remain explicitly simulated.

#### Scenario: React manages Loop state
- **WHEN** React lists, creates, edits, starts, monitors, pauses, resumes, cancels, accepts, continues, or rejects a Loop
- **THEN** it SHALL call the frontend service interface and SHALL NOT call Tauri `invoke()` directly

#### Scenario: Web runtime simulates Loop lifecycle
- **WHEN** the Loop Center runs through the Web/mock adapter
- **THEN** it SHALL expose contract-equivalent definitions, asynchronous phase transitions, iterations, evidence, controls, and terminal outcomes without local Git, SQLite, or Agent CLIs

## ADDED Requirements

### Requirement: Versioned scope migration preserves historical truth
Migration SHALL preserve existing definitions and run history without inventing authorization. Definitions lacking explicit supported scope/mode SHALL be editable as legacy-unverified but MUST require explicit valid configuration before start. Nonterminal runs lacking a valid original scope binding SHALL pause with scope-binding-missing and MUST NOT be resumed by synthesizing historical scope; terminal history SHALL retain its original result with no new enforcement claim.

#### Scenario: Migrate an old empty allowed scope
- **WHEN** an existing definition lacks scope version or has an empty allowed set
- **THEN** the system SHALL preserve it for editing and SHALL NOT infer whole-workspace access or start it silently

#### Scenario: Recover an old active run without a binding
- **WHEN** startup finds a nonterminal pre-migration run without original trustworthy scope binding
- **THEN** the runtime SHALL require cancellation and a new run from explicitly confirmed configuration rather than manufacture authority for the old run

#### Scenario: Display an old successful run
- **WHEN** a historical terminal run predates scope enforcement
- **THEN** the UI SHALL preserve its recorded result and SHALL indicate that new enforcement evidence is unavailable

### Requirement: In-process deterministic verification supports a complete strict Loop
The product SHALL provide a real preventive-required Loop path with mediated Worker tools, at least one required native-check, a read-only Verifier and native sealed acceptance. The initial native-check SHALL use program patch-whitespace, empty arguments and worktree-relative cwd `.` to inspect newly added text lines for trailing spaces or tabs against safe baseline/current artifact snapshots without executing a process, shell, Git command, script or plugin. It SHALL preserve required/timeout/status/evidence contracts, reject unknown native-check programs or arguments, mark incomplete reads/decoding unverifiable and clearly report binary text-rule exclusions while retaining complete binary scope checks. It MUST NOT replace user-configured required process checks or claim build/test correctness.

#### Scenario: Complete a native strict Loop
- **WHEN** a valid definition uses a mediated-only Worker, required patch-whitespace native-check and a proven read-only Verifier
- **THEN** the native runtime SHALL support start through actual scoped file mutation, required verification, Verifier advice, awaiting-acceptance and sealed acceptance without enabling an uncovered side-effect channel

#### Scenario: A new text line has trailing whitespace
- **WHEN** patch-whitespace finds a newly added text line ending with a space or tab
- **THEN** the required native check SHALL fail and SHALL prevent acceptance readiness

#### Scenario: Native check parameters or snapshot are invalid
- **WHEN** an unknown check/argument is requested or its bounded text inspection cannot complete consistently
- **THEN** the runtime SHALL reject the parameters or return unverifiable evidence rather than execute an arbitrary program or report pass

#### Scenario: Legacy process checks remain required
- **WHEN** a saved definition has required process checks with no kind field
- **THEN** the runtime SHALL preserve them as process checks and require appropriate execution capability rather than substitute the native check
