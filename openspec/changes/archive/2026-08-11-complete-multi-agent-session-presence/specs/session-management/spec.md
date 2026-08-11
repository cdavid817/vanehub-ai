## MODIFIED Requirements

### Requirement: Session entity contract
The system SHALL expose sessions as durable records with id, title, an ordered participant list, a stable agent id, interaction mode, lifecycle state, folder, optional project/worktree metadata, pinned, archived, created timestamp, and updated timestamp fields. Each participant SHALL carry a stable seat id, stable Agent id, captured expert-role presentation, join timestamp, and optional leave timestamp. A single-Agent session SHALL be represented as a session holding exactly one active participant, and the record's agent id SHALL equal the first active participant's agent id for compatibility.

#### Scenario: Create session with required metadata
- **WHEN** a session is created for a stable agent id and interaction mode
- **THEN** the system SHALL return a session record with a stable id, title, one active participant holding a stable seat id and that agent id, interaction mode, lifecycle state, pinned flag, archived flag, created timestamp, and updated timestamp

#### Scenario: Create a multi-seat session
- **WHEN** a session is created with two or more seats, each pairing a stable agent id with an expert role id
- **THEN** the system SHALL return a session record preserving the participant order
- **AND** each participant SHALL receive a distinct stable seat id and a snapshot of its role presentation

#### Scenario: Publish an asynchronously created multi-seat session
- **WHEN** desktop session creation completes through an asynchronous operation
- **THEN** the frontend SHALL resolve the created id to the canonical session record before publishing it as the active conversation
- **AND** the first render SHALL preserve every selected participant, stable seat id, role snapshot, and membership lifecycle field

#### Scenario: Create session with project metadata
- **WHEN** a session is created with a selected project folder
- **THEN** the system SHALL return a session record with the selected project path and effective folder path

#### Scenario: Create session with worktree metadata
- **WHEN** a session is created with a Git worktree
- **THEN** the system SHALL return a session record with the original project path, worktree path, worktree name, worktree branch, and effective folder path set to the worktree path

#### Scenario: Use default session title
- **WHEN** a session is created without an explicit title
- **THEN** the system SHALL assign the title "新会话"

#### Scenario: Preserve stable agent identity
- **WHEN** a participant references an Agent
- **THEN** the participant SHALL store the stable Agent id rather than matching by display name
- **AND** the participant's stable seat id SHALL NOT change when other participants join or leave

#### Scenario: Session agent id mirrors the first seat
- **WHEN** a session's active roster changes
- **THEN** the record's agent id SHALL be updated to the first active participant's agent id
- **AND** a reader that only knows about the agent id SHALL continue to observe the session's primary Agent

#### Scenario: Migrate an existing single-Agent session
- **WHEN** a session persisted before stable seat ids existed is read
- **THEN** the system SHALL assign a deterministic stable seat id and present it as a one-participant session carrying its original agent id and no role id
- **AND** no existing session SHALL become unreadable because of the participant model

#### Scenario: Migrate indexed message attribution
- **WHEN** a legacy message contains a seat index but no stable speaker seat id
- **THEN** the system SHALL resolve that index against the migrated participant order and persist or project the corresponding stable seat id
- **AND** an invalid legacy index SHALL degrade to unattributed rendering rather than another participant's identity

## ADDED Requirements

### Requirement: Service-backed participant membership
The frontend session service SHALL expose one runtime-neutral operation for replacing the active roster while preserving departed participant history.

#### Scenario: Update membership in desktop runtime
- **WHEN** the user adds or removes a participant in the desktop runtime
- **THEN** the frontend SHALL call the session service boundary and the native layer SHALL persist the membership change atomically
- **AND** the native layer SHALL reject a roster with no active participant

#### Scenario: Update membership in Web runtime
- **WHEN** the user adds or removes a participant in Web mode
- **THEN** the Web adapter SHALL implement the same input, validation, and output contract without requiring SQLite

#### Scenario: Reject stale membership update
- **WHEN** a membership update is based on an outdated session revision
- **THEN** the system SHALL reject it without overwriting a newer roster
- **AND** the UI SHALL reload the current roster and show a localized conflict message

### Requirement: Additive message-ordering schema compatibility
The desktop session repository SHALL remain writable when the shared SQLite database contains an additive, uniquely indexed per-session message sequence column.

#### Scenario: Insert consecutive messages into an upgraded shared database
- **WHEN** two or more messages are inserted for one session and `messages.session_sequence` has a unique `(session_id, session_sequence)` index
- **THEN** each insert SHALL allocate the next positive sequence for that session
- **AND** no insert SHALL fail because the column default repeats an existing sequence

#### Scenario: Insert into the current message schema
- **WHEN** the additive message sequence column is absent
- **THEN** message insertion SHALL preserve the current schema behavior
