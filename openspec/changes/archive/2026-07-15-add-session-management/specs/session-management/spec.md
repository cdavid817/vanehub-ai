## ADDED Requirements

### Requirement: Session entity contract
The system SHALL expose sessions as durable records with id, title, agent id, interaction mode, lifecycle state, folder, pinned, archived, created timestamp, and updated timestamp fields.

#### Scenario: Create session with required metadata
- **WHEN** a session is created for a stable agent id and interaction mode
- **THEN** the system SHALL return a session record with a stable id, title, agent id, interaction mode, lifecycle state, pinned flag, archived flag, created timestamp, and updated timestamp

#### Scenario: Use default session title
- **WHEN** a session is created without an explicit title
- **THEN** the system SHALL assign the title "新会话"

#### Scenario: Preserve stable agent identity
- **WHEN** a session references an agent
- **THEN** the session SHALL store the stable agent id rather than matching by display name

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
