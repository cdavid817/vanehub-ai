## MODIFIED Requirements

### Requirement: Session entity contract
The system SHALL expose sessions as durable records with id, title, an ordered seat list, a stable agent id, interaction mode, lifecycle state, folder, optional project/worktree metadata, pinned, archived, created timestamp, and updated timestamp fields. Each seat SHALL carry a stable Agent id and an optional expert role id, a single-Agent session SHALL be represented as a session holding exactly one seat, and the record's agent id SHALL always equal the first seat's agent id so existing readers keep working unchanged.

#### Scenario: Create session with required metadata
- **WHEN** a session is created for a stable agent id and interaction mode
- **THEN** the system SHALL return a session record with a stable id, title, a one-seat list holding that agent id, interaction mode, lifecycle state, pinned flag, archived flag, created timestamp, and updated timestamp

#### Scenario: Create a multi-seat session
- **WHEN** a session is created with two or more seats, each pairing a stable agent id with an expert role id
- **THEN** the system SHALL return a session record preserving the seat order
- **AND** each seat SHALL retain its agent id and role id independently, so the same agent id may appear under different roles in different sessions

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
- **WHEN** a seat references an agent
- **THEN** the seat SHALL store the stable agent id rather than matching by display name

#### Scenario: Session agent id mirrors the first seat
- **WHEN** a session's seats change
- **THEN** the record's agent id SHALL be updated to the first seat's agent id
- **AND** a reader that only knows about the agent id SHALL continue to observe the session's primary Agent

#### Scenario: Migrate an existing single-Agent session
- **WHEN** a session persisted before seats existed is read
- **THEN** the system SHALL present it as a one-seat session carrying its original agent id and no role id
- **AND** no existing session SHALL become unreadable because of the seat model
