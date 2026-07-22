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

### Requirement: Derived session visual identity
The system SHALL derive session icon identity from the session's stable agent id rather than persisting redundant icon metadata in the session entity.

#### Scenario: Store stable agent id only
- **WHEN** a session is created for Claude Code, Gemini CLI, Codex CLI, or OpenCode
- **THEN** the session record SHALL store the selected stable agent id
- **AND** it SHALL NOT require a persisted icon name, icon path, or icon color field

#### Scenario: Derive icon after reload
- **WHEN** persisted sessions are listed after app restart or Web/mock reload
- **THEN** the UI SHALL be able to render the CLI-specific icon from the stored stable agent id

### Requirement: Remote workspace session metadata
The system SHALL expose optional remote workspace metadata on durable session records.

#### Scenario: Create session with remote workspace metadata
- **WHEN** a session is created with a remote workspace request
- **THEN** the system SHALL return a session record with remote workspace host, user, path, display name, and effective folder set to a stable remote URI

#### Scenario: Search by remote workspace metadata
- **WHEN** a user searches historical sessions by remote host, user, path, display name, or remote URI
- **THEN** matching sessions SHALL be returned without requiring React to inspect remote state

### Requirement: Remote workspace creation input
The system SHALL allow session creation input to choose either a local project/worktree target or a remote workspace target.

#### Scenario: Reject incomplete remote workspace
- **WHEN** session creation receives a remote workspace request without host or path
- **THEN** the system SHALL reject the request without creating a session

#### Scenario: Reject mixed workspace targets
- **WHEN** session creation receives a remote workspace request and a Git worktree request
- **THEN** the system SHALL reject the request without executing Git commands

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL accept the same remote workspace input and return equivalent mock session metadata

### Requirement: New session defaults

The system SHALL generate a default new-session name from the selected/current project folder basename followed by a timestamp.

#### Scenario: Default name uses folder and timestamp

- **WHEN** a user opens the create-session flow for `D:\work\demo-app`
- **THEN** the default session name SHALL start with `demo-app-`
- **AND** the suffix SHALL be a timestamp suitable for distinguishing sessions.

### Requirement: User-safe session path display

The system SHALL strip Windows extended-length path prefixes from displayed paths and from values used only for display-derived labels.

#### Scenario: Extended-length path is displayed normally

- **WHEN** the selected folder is `\\?\D:\cdavid\Documents\code\claude-code`
- **THEN** the UI SHALL display `D:\cdavid\Documents\code\claude-code`
- **AND** the default session name SHALL use `claude-code` as the folder basename.

#### Scenario: Project grouping displays normal paths

- **WHEN** a listed session folder is stored as `\\?\D:\cdavid\Documents\code\claude-code`
- **THEN** project grouping labels SHALL display `D:\cdavid\Documents\code\claude-code`.

### Requirement: Recent project selection

The create-session local project section SHALL label persisted project choices as recently opened projects.

#### Scenario: Recent projects are listed

- **WHEN** known local projects are available during session creation
- **THEN** the create-session page SHALL present them under a recently opened projects label.

### Requirement: Single-Agent session mode
The system SHALL create first-version interactive CLI sessions as Single Agent sessions owned by the stable agent id selected in the create-session dialog.

#### Scenario: Create Single Agent session
- **WHEN** the user submits the create-session dialog in Single Agent mode for Claude Code, Gemini CLI, Codex CLI, or OpenCode
- **THEN** the created session SHALL store the selected stable agent id
- **AND** that selected agent id SHALL be the Agent used for automatic Agent Terminal startup

#### Scenario: Reject Multi Agent creation
- **WHEN** session creation receives a Multi Agent first-version request
- **THEN** the system SHALL reject or prevent the request without creating a session
- **AND** it SHALL report that Multi Agent sessions are not yet implemented

### Requirement: Agent terminal lifecycle coherence
The system SHALL keep session lifecycle state coherent with retained Agent Terminal processes.

#### Scenario: Terminal starts
- **WHEN** an Agent Terminal process starts for a session
- **THEN** the session lifecycle SHALL transition through `starting` to `running`
- **AND** session lists SHALL expose the updated lifecycle after refresh

#### Scenario: Terminal remains live after navigation
- **WHEN** the user switches away from a session whose Agent Terminal process is still live
- **THEN** the session lifecycle SHALL remain consistent with the retained live process
- **AND** selecting the session again SHALL reflect the attached process state

#### Scenario: Terminal exits
- **WHEN** an Agent Terminal process exits, fails to start, is stopped by idle cleanup, or is stopped during shutdown
- **THEN** the owning session lifecycle SHALL transition to `stopped` or `failed` according to the terminal outcome

### Requirement: Runtime session id resume metadata
The system SHALL persist provider runtime session ids on session records for Agent Terminal resume.

#### Scenario: Save terminal runtime session id
- **WHEN** the Agent Terminal runtime reports a provider session id for a session
- **THEN** the session record SHALL persist that value as its runtime session id
- **AND** the value SHALL remain available after desktop application restart

#### Scenario: Resume uses stored session id
- **WHEN** a session with a stored runtime session id is opened after its previous Agent Terminal process closed
- **THEN** the Agent Terminal runtime SHALL use the stored runtime session id to resume the provider CLI session when that provider supports resume

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL expose equivalent mock runtime session id metadata without requiring SQLite or local CLI execution

### Requirement: UI-driven multi-session deletion
The system SHALL support deleting multiple sessions from the session management UI through the frontend service boundary while preserving existing single-session deletion semantics.

#### Scenario: Delete selected sessions
- **WHEN** the user confirms deletion of multiple selected sessions
- **THEN** the UI SHALL request deletion through the frontend agent service for each selected session id
- **AND** React components SHALL NOT call Tauri `invoke()` or SQLite directly

#### Scenario: Refresh after multi-session deletion
- **WHEN** one or more selected sessions are deleted
- **THEN** the UI SHALL refresh active-visible sessions, archived sessions, active-session state, and workflow state

#### Scenario: Delete active session in batch
- **WHEN** the selected batch includes the active session
- **THEN** deletion SHALL clear the active session selection according to the existing active-session deletion behavior

#### Scenario: Report batch deletion failure
- **WHEN** deletion of one or more selected sessions fails
- **THEN** the UI SHALL show localized failure feedback
- **AND** it SHALL refresh session state so successful deletions and retained sessions are visible

### Requirement: Project-derived session grouping metadata
The system SHALL expose enough workspace metadata on listed session records for consumers to group sessions by project without querying SQLite or the filesystem from React components.

#### Scenario: Local project session grouping metadata
- **WHEN** sessions are listed and a session has worktree, project, or folder metadata
- **THEN** the returned session record SHALL include the existing worktree path, project path, and folder fields needed to derive an owning project group
- **AND** React components SHALL group from service-backed session records rather than direct native or database reads

#### Scenario: Session without project metadata
- **WHEN** sessions are listed and a session has no worktree, project, folder, or remote workspace metadata
- **THEN** the returned session record SHALL remain valid
- **AND** consumers SHALL be able to place it in a localized ungrouped project bucket

#### Scenario: Preserve list ordering inside project groups
- **WHEN** sessions are rendered in project groups
- **THEN** sessions within each group SHALL preserve the stable session listing order provided by the service

### Requirement: Remote session creation from SSH connection
The system SHALL allow remote session creation to derive remote workspace input from a selected SSH connection profile while preserving session-local remote metadata.

#### Scenario: Select SSH connection for remote session
- **WHEN** a user creates a remote session by selecting an SSH connection profile
- **THEN** the created session SHALL store a remote workspace snapshot derived from the profile host, port, user, effective path, display name, and stable URI
- **AND** the session SHALL remain readable without loading the source SSH connection profile

#### Scenario: Override SSH connection default path
- **WHEN** a user selects an SSH connection profile and changes the remote path before creating the session
- **THEN** the created session SHALL use the overridden path in its remote workspace snapshot
- **AND** the SSH connection default path SHALL remain unchanged

#### Scenario: Save temporary remote input as connection
- **WHEN** a user manually enters remote host, port, user, path, and authentication details in the create-session remote section and chooses to save them as a connection
- **THEN** the system SHALL create the SSH connection profile through the service boundary before or during session creation
- **AND** it SHALL still create the session from a remote workspace snapshot

#### Scenario: Preserve manual temporary remote session
- **WHEN** a user manually enters remote host, port, user, and path without saving them as a connection
- **THEN** the system SHALL create a remote session snapshot without creating a durable SSH connection profile

### Requirement: Remote workspace port schema upgrade
The desktop runtime SHALL add remote workspace port storage when upgrading an existing database that already applied the original remote workspace migration.

#### Scenario: Upgrade pre-SSH database
- **WHEN** a desktop database with migrations through version 23 starts against the SSH connection management release
- **THEN** migration 24 SHALL add the remote workspace history port column and session snapshot port column
- **AND** existing remote workspace and session records SHALL remain readable

#### Scenario: Initialize clean database
- **WHEN** the desktop runtime initializes a clean database
- **THEN** the final schema SHALL contain the SSH connection table and both remote workspace port columns

