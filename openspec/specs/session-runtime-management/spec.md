# session-runtime-management Specification

## Purpose
Defines how durable sessions own active generation state, runtime lifecycle transitions, cancellation, cleanup, and Agent runtime diagnostics.
## Requirements
### Requirement: Session-scoped runtime ownership
The desktop runtime SHALL correlate active Agent generation state with a stable execution run and session id, SHALL claim that ownership durably before execution, and SHALL prevent unrelated or competing work from sharing the claim or generation handle.

#### Scenario: Start generation for a session
- **WHEN** a message is accepted for a non-archived, recovery-clean session with no active execution run
- **THEN** the desktop runtime SHALL durably associate the execution run with that session before provider or CLI execution
- **AND** the session lifecycle SHALL transition to `starting` and then `running` when execution begins

#### Scenario: Reject duplicate same-session generation
- **WHEN** another submission races to start while the session already owns an active execution run
- **THEN** the durable claim SHALL reject the competing generation without starting another provider request or child process

#### Scenario: Isolate concurrent sessions
- **WHEN** two sessions have independent generation state
- **THEN** stopping, completing, or recovering one session SHALL NOT stop, complete, recover, or mutate the other session's active generation

### Requirement: Runtime terminal states update sessions
The desktop runtime SHALL persist terminal runtime outcomes back to the owning session record.

#### Scenario: Generation completes
- **WHEN** a session generation completes successfully
- **THEN** the assistant message SHALL be marked `completed`
- **AND** the owning session lifecycle SHALL be set to `idle`
- **AND** the session updated timestamp SHALL be refreshed

#### Scenario: Generation fails
- **WHEN** a session generation fails before completion
- **THEN** the assistant message SHALL be marked `failed`
- **AND** the owning session lifecycle SHALL be set to `failed`
- **AND** user-displayable error context SHALL be available through the message or session details contract

#### Scenario: Generation is cancelled
- **WHEN** a user stops an active generation for a session
- **THEN** the active assistant message SHALL be marked `cancelled`
- **AND** the owning session lifecycle SHALL be set to `stopped`
- **AND** already persisted message content SHALL remain available

### Requirement: Runtime cleanup on session removal
The desktop runtime SHALL clean up active generation state when a session is archived or deleted.

#### Scenario: Archive running session
- **WHEN** an active running session is archived
- **THEN** the runtime SHALL request cancellation for that session before hiding it from the normal session list
- **AND** the active session selection SHALL be cleared
- **AND** detailed cancellation diagnostics SHALL be persisted through unified logging rather than shown as raw chat output

#### Scenario: Delete running session
- **WHEN** a running session is deleted
- **THEN** the runtime SHALL stop the active generation for that session
- **AND** the session's persisted messages SHALL be removed through the ownership relationship

### Requirement: Runtime details remain behind adapters
Session runtime implementation details SHALL remain behind the frontend service and runtime adapter boundaries.

#### Scenario: React reads runtime status
- **WHEN** React UI needs session runtime status or details
- **THEN** it SHALL call the frontend service interface
- **AND** it SHALL NOT call Tauri `invoke()` or inspect SQLite directly

#### Scenario: UI limits generation to active session
- **WHEN** the first-version UI exposes send and stop generation controls
- **THEN** those controls SHALL operate only on the active session
- **AND** the desktop runtime SHALL still isolate generation handles by session id

#### Scenario: Web runtime mirrors lifecycle
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL expose the same session lifecycle, cancellation, and message status contract using mock runtime state

### Requirement: Session runtime owns cancellable CLI processes
The desktop runtime SHALL associate each active CLI chat generation with a cancellable child process owned by the session id.

#### Scenario: Start cancellable CLI generation
- **WHEN** a CLI chat generation starts for a session
- **THEN** the runtime SHALL store the active process handle with that session id
- **AND** stopping another session SHALL NOT affect the stored process

#### Scenario: Stop running CLI generation
- **WHEN** the user stops a session with an active CLI generation
- **THEN** the runtime SHALL request termination of that session's child process
- **AND** the active assistant message SHALL be marked `cancelled`
- **AND** already captured assistant content SHALL remain persisted

#### Scenario: Cleanup on archive or delete
- **WHEN** a running CLI session is archived or deleted
- **THEN** the runtime SHALL stop the owned child process before completing the archive or delete operation
- **AND** lifecycle and message state SHALL reflect the stopped or removed session ownership

### Requirement: Session runtime stores provider resume metadata
The desktop runtime SHALL store provider runtime session metadata when a CLI reports a native session id that can be used for future resume calls.

#### Scenario: Capture provider session id
- **WHEN** a provider CLI event includes a native runtime session id for the active generation
- **THEN** the desktop runtime SHALL persist that id with the owning VaneHub session
- **AND** later CLI invocations for the same session SHALL pass that id through the provider-specific resume path when supported

#### Scenario: Continue without provider session id
- **WHEN** a provider CLI does not report a native runtime session id
- **THEN** the desktop runtime SHALL continue the current generation without failing solely due to missing resume metadata
- **AND** it SHALL record the missing metadata condition in diagnostics when useful

### Requirement: Non-activating IM session creation
The native session runtime SHALL support creating an IM-owned session without changing the desktop workflow's active session.

#### Scenario: Create session for external chat
- **WHEN** the IM router creates a session for a new external-chat binding
- **THEN** the session SHALL be persisted with the configured Agent, CLI interaction mode, project path, IM source type, and source connector id
- **AND** `workflow_state.active_session_id` SHALL remain unchanged

### Requirement: IM session continuity
IM-owned sessions SHALL preserve the same provider runtime-session continuity as desktop-created sessions.

#### Scenario: Reuse IM-owned session
- **WHEN** a later external message targets an existing IM binding
- **THEN** the Agent invocation SHALL reuse the bound session and its persisted provider runtime-session id when supported

### Requirement: Shared message execution service
Desktop and IM message submission SHALL use one internal native execution service.

#### Scenario: Desktop submits message after refactor
- **WHEN** the frontend submits a desktop chat message through the existing Agent service
- **THEN** existing message persistence, streaming events, lifecycle state, CLI parsing, token accounting, and error behavior SHALL remain available

#### Scenario: Native IM router submits message
- **WHEN** the IM router submits a message directly to the internal service
- **THEN** it SHALL not require a frontend window, React event listener, or native-to-native Tauri command invocation

### Requirement: IM completion notification
The native chat runtime SHALL expose an event-driven internal terminal completion signal for IM-originated assistant messages and SHALL NOT require fixed-interval database polling to detect completion.

#### Scenario: Assistant completes
- **WHEN** an IM-originated assistant message reaches completed, failed, or cancelled state
- **THEN** the waiting IM job SHALL receive exactly one terminal result associated with the session and assistant message after terminal state persistence

#### Scenario: Assistant terminates before a waiter observes the signal
- **WHEN** terminal state is already persisted before the waiting path begins receiving
- **THEN** the native runtime SHALL return the persisted terminal result without waiting indefinitely or emitting a duplicate result

#### Scenario: Completion waiter is dropped
- **WHEN** an IM job times out, is cancelled, or drops its terminal receiver
- **THEN** the runtime SHALL release completion registration state without retaining an unbounded sender or polling worker

### Requirement: Deleted IM session recovery
Deleting an IM-owned session SHALL not leave a permanently unusable external-chat binding.

#### Scenario: User deletes IM session
- **WHEN** an IM-owned session is deleted from the existing session UI
- **THEN** its binding SHALL be removed or recognized as stale so the next external message can create a replacement session

### Requirement: Crash recovery reconciles orphan generations
The desktop runtime SHALL reconcile persisted generation state after a crash or unclean shutdown without assuming provider API requests, CLI child processes, or provider-internal tool activity survived, completed, or had no side effects.

#### Scenario: Startup detects orphan generation
- **WHEN** the runtime starts and a persisted session is `starting` or `running` but no live generation handle exists for that session
- **THEN** the runtime SHALL enter reconciliation for that session and SHALL NOT attempt to stop an unrelated process

#### Scenario: Reconcile conclusive orphan evidence
- **WHEN** an orphan generation has one conclusive terminal business outcome correlated with its active execution run
- **THEN** the owning assistant message and session lifecycle SHALL reflect that outcome, the active execution claim SHALL be cleared, and partial assistant content SHALL remain available

#### Scenario: Mark orphan generation failed
- **WHEN** an orphan generation has a conclusive failed outcome or an unfinished tool-free assistant response that can be terminated without ambiguity
- **THEN** the owning unfinished assistant message SHALL be marked `failed`, the session lifecycle SHALL be set to `failed`, the active execution claim SHALL be cleared, and partial assistant content SHALL remain available

#### Scenario: Require review for inconclusive activity
- **WHEN** an orphan generation has conflicting evidence or incomplete provider, CLI, or tool activity whose outcome cannot be proven
- **THEN** the session SHALL become recovery-action-required and the runtime SHALL NOT automatically retry or fabricate a terminal tool result

#### Scenario: Preserve resume metadata
- **WHEN** crash recovery reconciles an orphan session
- **THEN** the runtime SHALL preserve that session's provider runtime session id so a later explicit invocation can use the existing resume path when supported
- **AND** preserving the id SHALL NOT automatically resume the interrupted generation

### Requirement: Recovery diagnostics
Crash recovery SHALL persist redacted diagnostics through the unified logging service.

#### Scenario: Log recovered orphan state
- **WHEN** startup recovery mutates an orphan session or message
- **THEN** the runtime SHALL write a unified log entry with session id, agent id when available, previous lifecycle, new lifecycle, and recovery reason

### Requirement: Codex CLI message routing

The system SHALL route messages sent in Codex CLI sessions to the Codex runtime and SHALL surface either response events or actionable runtime errors in the session.

#### Scenario: User sends a message to Codex

- **WHEN** a session is created with the Codex CLI agent
- **AND** the user sends a chat message
- **THEN** the system SHALL invoke the Codex CLI with the configured profile
- **AND** the session SHALL show Codex output events or a visible error explaining why output could not be produced.

### Requirement: OpenCode CLI output normalization

The system SHALL parse current OpenCode JSON output into visible chat events.

#### Scenario: OpenCode emits current JSON event shape

- **WHEN** OpenCode emits events using fields such as `sessionID`, `part.text`, `step_start`, or `step_finish`
- **THEN** the runtime SHALL normalize them into provider session id, token, and completed chat events.

#### Scenario: OpenCode npm shim is resolved on Windows

- **WHEN** the configured OpenCode executable resolves to a Windows npm shim
- **THEN** the runtime SHALL resolve the real executable before starting chat generation when the real executable is available.

### Requirement: Shell cwd uses command-compatible Windows paths

Interactive shell sessions SHALL strip Windows extended-length prefixes before launching or resetting the shell current directory.

#### Scenario: CMD starts in selected folder

- **WHEN** a session folder resolves to `\\?\D:\cdavid\Documents\code\claude-code`
- **THEN** the shell runtime SHALL launch or reset the shell cwd using `D:\cdavid\Documents\code\claude-code`.

### Requirement: Shared message execution creates trace context
The shared native message execution service SHALL create and carry one execution context across preparation, Agent invocation, background monitoring, event handling, cancellation, and terminal persistence.

#### Scenario: Generation crosses a monitor thread
- **WHEN** an Agent generation starts a managed process and transfers monitoring to a background thread
- **THEN** the monitor and its emitted events SHALL retain the owning run, trace, Agent, session, message, and operation correlation

#### Scenario: Preparation fails before process start
- **WHEN** prompt assembly, CLI profile loading, or process construction fails before a child process starts
- **THEN** the run SHALL terminate with the corresponding failed stage and safe error classification

### Requirement: Managed Agent process telemetry
Managed Agent CLI execution SHALL record process spawn, start, output milestones, cancellation, exit, and monitoring failure within the owning Agent trace.

#### Scenario: Process starts successfully
- **WHEN** the runtime starts a managed Agent CLI child process
- **THEN** it SHALL record a safe executable classification, process identity, start timestamp, Agent id, and observation fidelity
- **AND** it SHALL NOT record the raw prompt argument or sensitive environment values

#### Scenario: Process exits unsuccessfully
- **WHEN** the child process exits with a non-success status or cannot be monitored
- **THEN** the process and Agent spans SHALL carry an error status and bounded error classification
- **AND** detailed redacted diagnostics SHALL remain available through unified logging

#### Scenario: Generation is cancelled
- **WHEN** a user, archive flow, delete flow, or owning runtime cancels generation
- **THEN** the trace SHALL record the cancellation initiator and terminal cancelled state
- **AND** cancellation of another session SHALL NOT mutate the run

### Requirement: Stream performance milestones
The runtime SHALL record first output and terminal stream milestones without persisting raw generated content under metadata-only capture.

#### Scenario: First visible output arrives
- **WHEN** the provider emits the first token, thinking block, tool event, rich block, or other visible output
- **THEN** the Agent execution SHALL record time to first output exactly once

#### Scenario: Stream completes without visible output
- **WHEN** the process completes successfully without a visible stream event
- **THEN** the run SHALL preserve that outcome without fabricating a first-output timestamp

### Requirement: Provider tool lifecycle normalization
Provider output adapters SHALL normalize tool lifecycle events by stable call id and SHALL preserve incomplete, duplicated, out-of-order, failed, and completed observations without inventing missing facts.

#### Scenario: Matching tool terminal event arrives
- **WHEN** a provider emits start and terminal events for the same stable tool-call id
- **THEN** the runtime SHALL update one correlated tool span with the terminal status and duration

#### Scenario: Duplicate tool event arrives
- **WHEN** a provider emits a duplicate lifecycle event for an already applied tool-call phase
- **THEN** normalization SHALL remain idempotent and SHALL NOT create a duplicate tool span

#### Scenario: Tool identity is unavailable
- **WHEN** a provider reports tool activity without a stable call id or name
- **THEN** the runtime SHALL preserve a bounded inferred observation when useful
- **AND** it SHALL NOT merge unrelated tool calls based only on display text

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

### Requirement: Claude Code CLI error result normalization

The system SHALL treat a claude-code `result` event that reports an error as a generation failure and SHALL surface the CLI's own diagnostic text rather than a generic exit status.

#### Scenario: Result event reports an error

- **WHEN** claude-code emits a `result` event whose payload marks it as an error
- **THEN** the runtime SHALL normalize it into a failed generation event
- **AND** the failure diagnostic SHALL be the CLI's reported result text
- **AND** the runtime SHALL NOT normalize it into a completed event

#### Scenario: Error result carries a structured code

- **WHEN** an error result carries a structured error code, status, type, or reason
- **THEN** the runtime SHALL use it to classify the failure as retryable or non-retryable
- **AND** an authentication or policy rejection SHALL NOT be classified as retryable

#### Scenario: Result event reports success

- **WHEN** claude-code emits a `result` event that does not report an error
- **THEN** the runtime SHALL normalize it into a completed event with its token accounting unchanged

### Requirement: Process failure prefers a parsed diagnostic

The system SHALL report the most specific failure text available when a managed Agent process exits non-zero, and SHALL fall back to the exit status only when no diagnostic was parsed or written.

#### Scenario: Process exits non-zero after an error result

- **WHEN** a managed Agent process exits non-zero and the output stream already yielded a failure diagnostic
- **THEN** the runtime SHALL report that diagnostic
- **AND** it SHALL NOT replace it with the process exit status

#### Scenario: Process exits non-zero with empty stderr and no diagnostic

- **WHEN** a managed Agent process exits non-zero, writes nothing to standard error, and produced no parsed failure diagnostic
- **THEN** the runtime SHALL report the exit status, because no better information exists

