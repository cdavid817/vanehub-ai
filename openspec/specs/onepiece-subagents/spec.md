# onepiece-subagents Specification

## Purpose
TBD - created by archiving change add-onepiece-subagents. Update Purpose after archive.
## Requirements
### Requirement: Bounded child attempt admission
The system SHALL expose a `delegate_subagent` tool, eligible only for stable Agent id `onepiece`, that admits one bounded child attempt bound to the parent session, the parent generation, the parent's canonical workspace, and an immutable snapshot of the active provider Profile. The child SHALL receive its own context window and SHALL NOT inherit the parent's conversation transcript. The tool SHALL NOT accept a workspace path, provider identity, credential, model identifier, or nested runtime configuration from the caller.

#### Scenario: Child attempt starts
- **WHEN** OnePiece calls `delegate_subagent` with a valid bounded task in an eligible session
- **THEN** the system SHALL start one child attempt bound to the parent session, generation, workspace, and captured Profile snapshot
- **AND** the child SHALL start with its own context rather than a copy of the parent's transcript

#### Scenario: Caller supplies a forged scope
- **WHEN** a call supplies a workspace path, provider, credential, model, or nested configuration
- **THEN** the system SHALL reject it without starting an attempt

#### Scenario: Non-OnePiece Agent calls the tool
- **WHEN** a user-created API Agent calls `delegate_subagent`
- **THEN** the registry SHALL deny eligibility because its stable id is not `onepiece`

#### Scenario: Profile changes mid-attempt
- **WHEN** the active OnePiece Profile changes while a child attempt is running
- **THEN** the running attempt SHALL continue on its captured snapshot
- **AND** its usage SHALL remain attributed to that snapshot

### Requirement: Restricted child tool pool
A child attempt SHALL receive a restricted tool pool rather than the parent's full catalog. By default the pool SHALL be read-only exploration: file read, content search, filename search, code intelligence, and the task-list tool. Workspace mutation SHALL be available to a child only when the parent's own session already permits mutation and the caller explicitly requests a mutating child. A child SHALL NOT receive `ask_user_question`, `delegate_cli`, `apply_delegation_changes`, or `delegate_subagent`.

#### Scenario: Default read-only pool
- **WHEN** a child attempt starts without an explicit mutating request
- **THEN** its tool pool SHALL contain only read-only exploration tools and the task-list tool

#### Scenario: Mutating child in a plan-mode parent
- **WHEN** a caller requests a mutating child from a session whose permission mode forbids mutation
- **THEN** the system SHALL reject the request rather than granting the child more authority than its parent has

#### Scenario: Child requests a prohibited tool
- **WHEN** a child requests a tool outside its pool
- **THEN** the runtime SHALL reject the call regardless of model output
- **AND** the attempt SHALL continue rather than failing the parent generation

#### Scenario: Child cannot ask the user
- **WHEN** a child attempt requests `ask_user_question`
- **THEN** the system SHALL refuse it as a non-interactive context

### Requirement: No nesting
A child attempt SHALL NOT start another child attempt. The system SHALL reject a nested request with an explicit error and SHALL NOT queue, defer, or silently flatten it.

#### Scenario: Child delegates again
- **WHEN** a child attempt calls `delegate_subagent`
- **THEN** the system SHALL reject the call with an explicit nesting error
- **AND** no further attempt SHALL start

### Requirement: Child attempt bounds
Each child attempt SHALL enforce configured ceilings for tool calls, tokens, wall-clock duration, and returned result size, plus a maximum number of concurrent children per parent session. Reaching any ceiling SHALL stop the attempt at the nearest safe boundary and return a classified limit outcome. A failed or limit-stopped attempt SHALL NOT be retried automatically.

#### Scenario: Attempt reaches a ceiling
- **WHEN** a child attempt reaches its tool-call, token, duration, or result-size ceiling
- **THEN** the system SHALL stop it at the nearest safe boundary
- **AND** it SHALL return a classified limit outcome together with whatever verified work it completed

#### Scenario: Concurrency limit reached
- **WHEN** a parent session already has the maximum number of running children and requests another
- **THEN** the system SHALL reject the request with an explicit limit error
- **AND** it SHALL NOT terminate a running child to make room

#### Scenario: Failed attempt is not retried
- **WHEN** a child attempt fails or is stopped at a limit
- **THEN** the system SHALL NOT start a replacement attempt automatically

### Requirement: Child results are bounded and do not enter the parent transcript
A child attempt SHALL return a bounded structured result to the parent as its tool result. The child's own conversation turns, tool inputs, and tool outputs SHALL NOT be appended to the parent's transcript. Child progress SHALL be reported through the parent's task list and through execution observability rather than by streaming the child's transcript into the parent's context.

#### Scenario: Result returns without the child's transcript
- **WHEN** a child attempt completes
- **THEN** the parent SHALL receive a bounded structured result
- **AND** the child's turns, tool inputs, and tool outputs SHALL NOT be appended to the parent's transcript

#### Scenario: Progress is visible without context cost
- **WHEN** a child attempt is running
- **THEN** its progress SHALL be observable through the parent's task list and execution observability
- **AND** observing it SHALL NOT add the child's transcript to the parent's context

#### Scenario: Oversized result
- **WHEN** a child's result exceeds the declared result-size bound
- **THEN** the system SHALL truncate at that bound and state that truncation occurred

### Requirement: Mutating children are isolated and sealed
A mutating child attempt SHALL run in an isolated worktree rather than in the parent's workspace, and SHALL return its changes as a sealed ChangeSet applied through the existing once-only exact-ChangeSet approval. A child SHALL NOT write into the parent's workspace directly, and two concurrent children SHALL NOT share a worktree.

#### Scenario: Mutating child writes in isolation
- **WHEN** a mutating child attempt modifies files
- **THEN** it SHALL do so in its own isolated worktree
- **AND** the parent's workspace SHALL remain unmodified until a ChangeSet is applied

#### Scenario: Applying a child's changes
- **WHEN** a child's sealed ChangeSet is applied
- **THEN** it SHALL go through the existing once-only approval bound to the ChangeSet's content hash, diff hash, repository identity, exact base commit, and clean-state witness

#### Scenario: Concurrent mutating children
- **WHEN** two mutating children run at once
- **THEN** each SHALL own a distinct worktree

### Requirement: Child cancellation and cleanup
Cancelling the parent generation or ending the parent session SHALL cancel every running child attempt and reap its processes and worktrees. A cancelled child SHALL return a classified cancellation outcome rather than a partial result presented as complete.

#### Scenario: Parent generation is cancelled
- **WHEN** a user cancels a generation that has running children
- **THEN** the system SHALL cancel every running child and reap its processes and worktrees

#### Scenario: Parent session ends
- **WHEN** a session with running children ends
- **THEN** the system SHALL cancel and reap them

#### Scenario: Cancelled child reports honestly
- **WHEN** a child attempt is cancelled mid-work
- **THEN** its outcome SHALL be classified as cancelled rather than returned as a completed result

### Requirement: Child accounting and logging
Child attempts SHALL account for provider usage through the existing invocation accounting contract, distinguishable from the parent's own visible-response, tool-continuation, compaction, and memory-extraction purposes while still rolling up to the parent session. Durable logs SHALL contain bounded identifiers, outcome codes, counts, and timing only, and SHALL NOT contain child prompts, transcripts, or raw tool output. A child SHALL reuse the parent's Profile-scoped credential through the existing credential boundary without copying it into any record.

#### Scenario: Child usage is attributable
- **WHEN** a child attempt consumes provider tokens
- **THEN** usage consumers SHALL be able to distinguish child consumption from the parent's other purposes
- **AND** it SHALL still roll up to the parent session's total

#### Scenario: Child logging is content-free
- **WHEN** the system logs a child attempt
- **THEN** the durable log SHALL contain identifiers, outcome codes, counts, and timing only

#### Scenario: Credential is not copied
- **WHEN** a child attempt runs
- **THEN** it SHALL use the parent's Profile-scoped credential through the existing credential boundary
- **AND** the credential SHALL NOT be copied into attempt records, prompts, or telemetry

