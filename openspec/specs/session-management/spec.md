# session-management Specification

## Purpose
Defines durable session records, active-session selection, session listing, mutation operations, and runtime persistence expectations shared by the Tauri desktop runtime and browser Web runtime.
## Requirements
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

### Requirement: Session metadata parity across runtimes
The system SHALL keep session metadata behavior consistent between desktop and Web runtimes.

#### Scenario: Web runtime default title parity
- **WHEN** a session is created in Web mode without an explicit title
- **THEN** the Web adapter SHALL assign the title "新会话"

#### Scenario: UI displays selected session metadata
- **WHEN** the main layout shows session configuration or runtime context
- **THEN** it SHALL display metadata from the active session or service-backed runtime details
- **AND** it SHALL NOT show hard-coded placeholder session names as current session data

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

### Requirement: Session listing
The system SHALL provide service operations to list active-visible sessions and archived sessions.

#### Scenario: List sessions in stable order
- **WHEN** sessions are listed for the normal sidebar view
- **THEN** the system SHALL return sessions ordered with pinned sessions before unpinned sessions and most recently updated sessions before older sessions within each group

#### Scenario: List archived sessions separately
- **WHEN** archived sessions are requested
- **THEN** the system SHALL return archived sessions without requiring the caller to filter the normal session list

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL provide the same session listing contract without requiring SQLite

### Requirement: Active session selection
The system SHALL track one active session independently from the full session list.

#### Scenario: Switch active session
- **WHEN** a user switches to an existing non-archived session
- **THEN** the system SHALL make that session the active session and align the active workflow agent id, interaction mode, and lifecycle state with the selected session

#### Scenario: Get active session
- **WHEN** an active session id is stored and the session still exists
- **THEN** the system SHALL return that session as the active session

#### Scenario: Clear missing active session
- **WHEN** the stored active session id no longer matches an existing session
- **THEN** the system SHALL return no active session rather than returning stale session data

### Requirement: Session mutation operations
The system SHALL provide service operations to rename, pin, unpin, archive, unarchive, and delete sessions.

#### Scenario: Rename session
- **WHEN** a user renames a session to a non-empty title
- **THEN** the system SHALL update the session title and updated timestamp

#### Scenario: Pin and unpin session
- **WHEN** a user pins or unpins a session
- **THEN** the system SHALL update the pinned flag and updated timestamp

#### Scenario: Archive active session
- **WHEN** a user archives the active session
- **THEN** the system SHALL mark the session archived and clear the active session selection

#### Scenario: Restore archived session
- **WHEN** a user restores an archived session
- **THEN** the system SHALL mark the session unarchived and keep the session available for normal listing and selection

#### Scenario: Delete active session
- **WHEN** a user deletes the active session
- **THEN** the system SHALL remove the session and clear the active session selection

### Requirement: Session messages belong to their session
The system SHALL associate persisted chat messages with their owning session record.

#### Scenario: List messages for selected session
- **WHEN** messages are listed for a session id
- **THEN** only messages owned by that session SHALL be returned

#### Scenario: Delete session removes messages
- **WHEN** a session with persisted messages is deleted
- **THEN** persisted messages for that session SHALL be deleted through the session ownership relationship

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** session-owned mock messages SHALL follow the same ownership contract without requiring SQLite

### Requirement: Desktop session persistence
The desktop runtime SHALL persist sessions through the Rust/Tauri SQLite layer and SHALL expose session actions through Tauri commands behind the frontend adapter.

#### Scenario: Persist sessions across desktop restart
- **WHEN** a session is created in the desktop runtime and the app is restarted
- **THEN** the session SHALL remain available from the desktop session list

#### Scenario: Keep SQLite out of React components
- **WHEN** React UI code creates, lists, switches, or mutates sessions
- **THEN** the UI SHALL call the frontend service interface rather than calling Tauri commands or SQLite directly

#### Scenario: Keep invoke in Tauri adapter
- **WHEN** the desktop frontend performs a session operation
- **THEN** Tauri `invoke()` usage SHALL remain in the Tauri-specific frontend adapter

### Requirement: Historical session search
The system SHALL search historical sessions by title, project metadata, and persisted message content.

#### Scenario: Search by title
- **WHEN** a user submits a non-empty session search query matching a session title
- **THEN** the system SHALL return bounded matching sessions with stable ids, title, agent id, project metadata, archived state, category id, and updated timestamp

#### Scenario: Search by project metadata
- **WHEN** a user submits a query matching a session project path, worktree path, worktree name, or worktree branch
- **THEN** the system SHALL return the matching sessions without requiring React to inspect SQLite or local filesystem state

#### Scenario: Search by message content
- **WHEN** a user submits a query matching persisted message content
- **THEN** the system SHALL return the owning sessions with bounded match context and SHALL NOT return messages from unrelated sessions

#### Scenario: Include archived sessions
- **WHEN** historical search is performed
- **THEN** the result set SHALL include both active-visible and archived sessions and SHALL identify archived results

### Requirement: Session category linkage
The system SHALL expose a nullable category id on durable session records.

#### Scenario: List categorized sessions
- **WHEN** sessions are listed
- **THEN** each session SHALL include its current category id or null when uncategorized

#### Scenario: Delete category preserves sessions
- **WHEN** a category is deleted
- **THEN** sessions assigned to that category SHALL become uncategorized rather than being deleted or archived

### Requirement: Automatic inactive session archival
The desktop runtime SHALL automatically archive inactive eligible sessions using Rust-owned background work.

#### Scenario: Startup archival check
- **WHEN** the desktop application starts
- **THEN** the native runtime SHALL check for inactive sessions using the configured threshold and archive eligible sessions before the next regular hourly check

#### Scenario: Hourly archival check
- **WHEN** the desktop application remains running and automatic archival is enabled
- **THEN** the native runtime SHALL check for eligible inactive sessions once per hour

#### Scenario: Archive eligible inactive session
- **WHEN** a non-pinned, non-archived session has not been updated for more than the configured number of days
- **THEN** the native runtime SHALL archive that session and record the action through unified logging

#### Scenario: Skip protected session
- **WHEN** a session is pinned, already archived, `starting`, or `running`
- **THEN** automatic archival SHALL leave that session unchanged

### Requirement: Startup session state recovery
The desktop runtime SHALL reconcile persisted active session states after application startup.

#### Scenario: Recover orphan running session
- **WHEN** startup recovery finds a session persisted as `starting` or `running` without a live generation handle
- **THEN** the runtime SHALL mark the session `failed`, preserve its partial content and provider runtime session id, and write recovery diagnostics through unified logging

#### Scenario: Recover unfinished assistant message
- **WHEN** startup recovery finds a `pending` or `streaming` assistant message for an orphan active session
- **THEN** the runtime SHALL mark that message `failed` while preserving already persisted content

