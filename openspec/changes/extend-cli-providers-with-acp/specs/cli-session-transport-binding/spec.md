## ADDED Requirements

### Requirement: Persistent explicit CLI session binding

New managed CLI sessions SHALL persist a binding to provider id, installation identity, distribution, version/profile, transport, adapter revision, canonical workspace/worktree and external session id. Credential values SHALL not be persisted in this binding. Missing historical fields MUST remain distinguishable from verified metadata.

#### Scenario: A new ACP session is created

- **WHEN** an agent returns an external session id
- **THEN** the runtime SHALL persist it with the correct VaneHub session and execution identity

#### Scenario: Two same-provider sessions run

- **WHEN** two seats use the same CLI in separate workspaces
- **THEN** their bindings SHALL remain distinct and SHALL NOT be resolved by most-recent-session discovery

### Requirement: Negotiated resume without session guessing

Resume SHALL require peer support, a verified compatible binding, and an explicit external session id. Failure SHALL preserve local history and return a classified reason. The runtime MUST NOT substitute a recent or unrelated session.

#### Scenario: Compatible session loads

- **WHEN** session loading is supported and its stored binding remains compatible
- **THEN** the runtime SHALL request the recorded external session with the correct workspace

#### Scenario: External session is missing

- **WHEN** the CLI cannot find the recorded session
- **THEN** the UI SHALL preserve local history and offer an explicit new-session path without calling it successful resume

#### Scenario: Critical identity changes

- **WHEN** distribution, account environment, transport, or workspace changes without a verified compatibility rule
- **THEN** resume SHALL be refused until an explicit compatible path is selected

### Requirement: Replay provenance and deduplication

History emitted while loading a session SHALL be marked as replay and reconciled separately from live generation. Replay SHALL NOT trigger tool execution, duplicate transcript persistence, repeat approvals, or increment usage. Legitimate repeated live text MUST NOT be removed solely by content hashing.

#### Scenario: Session load replays old messages

- **WHEN** the agent emits historical message and tool updates during load
- **THEN** the runtime SHALL display or reconcile them without duplicating existing stored messages or accounting

#### Scenario: Identical text occurs in a new turn

- **WHEN** a live turn intentionally repeats a previous message
- **THEN** the runtime SHALL retain the new message as live content

### Requirement: Safe additive persistence migration

New metadata SHALL use the existing additive SQLite migration system and preserve original-provider sessions, settings and logs. Existing unknown fields SHALL remain null or explicitly legacy. Reopening the application MUST NOT reinterpret all old CLI sessions as ACP.

#### Scenario: Old database is migrated

- **WHEN** the database contains original-provider sessions without ACP fields
- **THEN** migration SHALL preserve the records and original execution routing

#### Scenario: New providers are disabled for rollback

- **WHEN** a user or release disables a new adapter
- **THEN** existing history SHALL remain readable and SHALL NOT be deleted as cleanup

### Requirement: No silent transport switching or task replay

Changing between terminal, headless and ACP SHALL require an explicit compatible transition or a new session. A connection interruption after possible side effects SHALL be recorded as interrupted with unknown effects and SHALL NOT automatically resend the previous prompt.

#### Scenario: User switches from ACP to terminal

- **WHEN** no tested cross-transport resume mapping exists
- **THEN** the UI SHALL explain that a new session is required rather than claiming seamless continuation

#### Scenario: Connection is lost after a write

- **WHEN** the agent disconnects after it may have modified a file
- **THEN** the runtime SHALL preserve evidence and SHALL NOT automatically repeat the instruction

### Requirement: Scoped teardown and session deletion

Stopping or deleting a CLI session SHALL release its owned connections, pending interactions and child processes according to existing session semantics. It SHALL NOT delete shared CLI credential stores, global histories, unrelated sessions, or worktrees without the existing separate explicit cleanup decision.

#### Scenario: One session is deleted

- **WHEN** two sessions use the same provider and one is explicitly deleted
- **THEN** only the deleted session runtime resources SHALL be cleaned up

#### Scenario: Session references a worktree

- **WHEN** a session teardown occurs for a worktree-backed session
- **THEN** worktree deletion SHALL remain governed by the existing workspace cleanup workflow
