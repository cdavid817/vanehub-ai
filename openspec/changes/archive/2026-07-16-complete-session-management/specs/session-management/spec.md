## ADDED Requirements

### Requirement: Session metadata parity across runtimes
The system SHALL keep session metadata behavior consistent between desktop and Web runtimes.

#### Scenario: Web runtime default title parity
- **WHEN** a session is created in Web mode without an explicit title
- **THEN** the Web adapter SHALL assign the title "新会话"

#### Scenario: UI displays selected session metadata
- **WHEN** the main layout shows session configuration or runtime context
- **THEN** it SHALL display metadata from the active session or service-backed runtime details
- **AND** it SHALL NOT show hard-coded placeholder session names as current session data

### Requirement: Session lifecycle coherence
The system SHALL keep session lifecycle state coherent with message generation operations.

#### Scenario: Send message updates session lifecycle
- **WHEN** a message generation starts for a session
- **THEN** the session lifecycle SHALL reflect active generation state
- **AND** session lists SHALL expose the updated lifecycle after refresh

#### Scenario: Terminal message state updates session lifecycle
- **WHEN** an assistant message reaches `completed`, `failed`, or `cancelled`
- **THEN** the owning session lifecycle SHALL transition to the corresponding idle, failed, or stopped state

#### Scenario: Switching session reflects stored lifecycle
- **WHEN** a user switches to a non-archived session
- **THEN** active workflow state and visible session status SHALL reflect the selected session's current lifecycle

## MODIFIED Requirements

### Requirement: Session entity contract
The system SHALL expose sessions as durable records with id, title, agent id, interaction mode, lifecycle state, folder, optional project/worktree metadata, pinned, archived, created timestamp, and updated timestamp fields.

#### Scenario: Create session with required metadata
- **WHEN** a session is created for a stable agent id and interaction mode
- **THEN** the system SHALL return a session record with a stable id, title, agent id, interaction mode, lifecycle state, pinned flag, archived flag, created timestamp, and updated timestamp

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
- **WHEN** a session references an agent
- **THEN** the session SHALL store the stable agent id rather than matching by display name

### Requirement: Session creation input
The system SHALL create sessions from a service-level input that includes stable agent id, interaction mode, selected project path, and optional worktree request.

#### Scenario: Create session for selected agent
- **WHEN** the user creates a session for Claude Code, Gemini CLI, Codex, or OpenCode
- **THEN** the created session SHALL store the selected stable agent id rather than matching by display name

#### Scenario: Reject unsupported agent
- **WHEN** session creation receives an unsupported agent id
- **THEN** the system SHALL reject the request without creating a session

#### Scenario: Create session uses selected folder
- **WHEN** the user creates a session without worktree creation
- **THEN** the created session SHALL use the selected project folder as the effective folder

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL accept the same session creation input and return equivalent mock session metadata
