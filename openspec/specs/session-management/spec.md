# session-management Specification

## Purpose
Defines durable session records, active-session selection, session listing, mutation operations, and runtime persistence expectations shared by the Tauri desktop runtime and browser Web runtime.
## Requirements
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
The system SHALL create sessions from a service-level input that includes stable agent id, interaction mode, selected project path, and optional worktree request. The native and Web boundaries SHALL accept a declared `api` mode as well as existing supported modes and SHALL validate the selected Agent's identity, declared mode, and readiness before persisting the session.

#### Scenario: Create session for selected agent
- **WHEN** the user creates a session for Claude Code, Gemini CLI, Codex CLI, or OpenCode using a declared CLI mode
- **THEN** the created session SHALL store the selected stable agent id rather than matching by display name

#### Scenario: Create session for OnePiece
- **WHEN** the user selects a ready OnePiece and submits a local Single-Agent session
- **THEN** the frontend SHALL submit `agentId = onepiece` and `interactionMode = api`
- **AND** the created session SHALL persist those stable values

#### Scenario: Reject unsupported agent
- **WHEN** session creation receives an unknown Agent id or a mode that the selected Agent does not declare
- **THEN** the system SHALL reject the request without creating a session

#### Scenario: Reject a non-ready API Agent
- **WHEN** session creation receives an API Agent whose availability is not selectable
- **THEN** the native or Web boundary SHALL reject the request with a safe readiness reason
- **AND** it SHALL NOT contact the provider or create a session

#### Scenario: Create session uses selected folder
- **WHEN** the user creates a session without worktree creation
- **THEN** the created session SHALL use the selected project folder as the effective folder

#### Scenario: Reject remote OnePiece session
- **WHEN** session creation combines `agentId = onepiece` with a remote workspace request
- **THEN** the frontend SHALL prevent submission or the service SHALL reject it
- **AND** the system SHALL explain that first-version OnePiece sessions require a local project or local worktree

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL accept the same supported session creation input and return equivalent mock session metadata
- **AND** it SHALL enforce equivalent Agent mode, readiness, and workspace restrictions

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
The desktop runtime SHALL reconcile persisted active session states after application startup by correlating durable business evidence for the owning execution run and SHALL represent ambiguous recovery safety independently from lifecycle.

#### Scenario: Recover orphan running session
- **WHEN** startup recovery finds a session persisted as `starting` or `running` without a live generation handle and finds one conclusive terminal outcome for its active execution run
- **THEN** the runtime SHALL project that outcome to the session lifecycle, clear the active execution claim, preserve partial content and provider runtime session id, and write recovery diagnostics through unified logging

#### Scenario: Recover unfinished assistant message
- **WHEN** startup recovery finds a `pending` or `streaming` assistant message for the active execution run with no conflicting terminal or uncertain tool evidence
- **THEN** the runtime SHALL mark that message interrupted or failed while preserving already persisted content and SHALL return the session to a recovery-clean terminal lifecycle

#### Scenario: Preserve ambiguous active evidence for review
- **WHEN** an orphan active session contains conflicting execution evidence or unfinished tool activity whose effect is not conclusively known
- **THEN** the runtime SHALL preserve the evidence and place the session in `action_required` rather than treating a failed lifecycle projection as proof that no effect occurred

### Requirement: Derived session visual identity
The system SHALL derive session icon identity from the session's stable agent id rather than persisting redundant icon metadata in the session entity, including for OnePiece, built-in CLI Agents, and user-created API Agents.

#### Scenario: Store stable agent id only
- **WHEN** a session is created for OnePiece, Claude Code, Gemini CLI, Codex CLI, OpenCode, or another eligible API Agent
- **THEN** the session record SHALL store the selected stable agent id
- **AND** it SHALL NOT require a persisted icon name, icon path, or icon color field

#### Scenario: Derive icon after reload
- **WHEN** persisted sessions are listed after app restart or Web/mock reload
- **THEN** the UI SHALL render a known first-party identity from the stable Agent id
- **AND** it SHALL render the generic Agent identity for an unrecognized user-created id

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
The system SHALL create first-version interactive CLI or API chat sessions as Single Agent sessions owned by the stable agent id selected in the create-session dialog.

#### Scenario: Create Single Agent session
- **WHEN** the user submits the create-session dialog in Single Agent mode for Claude Code, Gemini CLI, Codex CLI, or OpenCode
- **THEN** the created session SHALL store the selected stable agent id
- **AND** that selected agent id SHALL be the Agent used for automatic Agent Terminal startup

#### Scenario: Create OnePiece Single Agent session
- **WHEN** the user submits the create-session dialog in Single Agent mode for a ready OnePiece
- **THEN** the created session SHALL store stable id `onepiece` with interaction mode `api`
- **AND** the system SHALL NOT start or offer an Agent Terminal for that session

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

### Requirement: Remote session SSH profile binding
The system SHALL preserve remote workspace snapshots while storing an optional operational SSH profile id and revision binding for remote Terminal use.

#### Scenario: Create remote session from profile
- **WHEN** a user creates a remote session from an SSH connection profile
- **THEN** the session SHALL store the profile id and current revision in addition to its independent host, port, user, path, display name, and URI snapshot

#### Scenario: Existing remote session migration
- **WHEN** an existing remote session predates SSH profile binding columns
- **THEN** it SHALL remain readable with its snapshot and SHALL require explicit binding before remote Terminal use

#### Scenario: Profile edit does not redirect session
- **WHEN** a bound profile changes endpoint or authentication configuration
- **THEN** the session snapshot SHALL remain unchanged and its old binding SHALL become stale rather than silently connecting to the changed target

#### Scenario: Profile deletion preserves snapshot
- **WHEN** a bound SSH profile is deleted
- **THEN** the session SHALL retain its remote workspace snapshot and SHALL require rebind before opening Terminal

#### Scenario: Rebind remote session
- **WHEN** a user explicitly rebinds a remote session to a compatible SSH profile
- **THEN** the system SHALL update only the operational profile id and revision unless the user separately confirms a workspace-target change

### Requirement: Bounded historical search scheduling
The session search UI MUST suppress trivial or superseded requests and MUST preserve the service boundary in both desktop and Web/mock runtimes.

#### Scenario: Type a historical search query
- **WHEN** a user changes the search text repeatedly within 250 milliseconds
- **THEN** the UI SHALL submit only the latest trimmed query after the quiet period
- **AND** it SHALL NOT submit a query shorter than two characters

#### Scenario: Search through either runtime
- **WHEN** a debounced query is submitted in desktop or Web/mock mode
- **THEN** React SHALL use the shared frontend session service interface
- **AND** the result count SHALL remain bounded by the requested service limit

### Requirement: Indexed desktop message search
Desktop historical-session search MUST use an SQLite full-text index for ordinary persisted message substring queries and MUST return session records plus bounded match context without per-result database loads. Search MUST bound the work it performs, not only the number of rows it returns: it MUST NOT rank or sort every matching message in the database in order to produce a limited result page.

#### Scenario: Search existing indexed messages
- **WHEN** a query of at least three characters matches persisted message content
- **THEN** SQLite SHALL use the maintained message-content FTS index
- **AND** the repository SHALL return the owning sessions and match context from one result query

#### Scenario: Keep the message index synchronized
- **WHEN** a persisted message is inserted, its content is updated, or it is deleted
- **THEN** the SQLite FTS index SHALL reflect that change transactionally

#### Scenario: Upgrade an existing database
- **WHEN** the database migration runs with existing persisted messages
- **THEN** it SHALL backfill those messages into the FTS index
- **AND** historical search SHALL continue to include archived sessions

#### Scenario: Search a two-character query
- **WHEN** the desktop repository receives a two-character query that cannot use the trigram index
- **THEN** it SHALL use a bounded compatibility query
- **AND** it SHALL return the same service result shape

#### Scenario: Short query resolves match context through the session index
- **WHEN** the desktop repository runs the short-query compatibility path
- **THEN** it SHALL resolve each candidate session's newest matching message through the session/created-at message index
- **AND** it SHALL NOT materialize and rank the full set of matching messages across all sessions

#### Scenario: Short query returns the same results as the ranking form
- **WHEN** the same two-character query is served by the bounded compatibility path
- **THEN** it SHALL return the same sessions, in the same order, as ranking every matching message would
- **AND** each returned session's match context SHALL be its newest matching message

### Requirement: Service-backed session Agent discovery
The create-session UI SHALL derive candidate Agents from service-backed registry entries and their declared interaction modes and availability rather than from a fixed stable-id allowlist.

#### Scenario: List CLI and API session candidates
- **WHEN** the registry contains Agents that declare `cli` or `api` interaction support
- **THEN** the create-session UI SHALL present those Agents as Single-Agent candidates
- **AND** it SHALL NOT require their ids to appear in a frontend eligibility allowlist

#### Scenario: Exclude non-chat-only candidates
- **WHEN** a registry entry declares only `browser` or `native-desktop` interaction support
- **THEN** the create-session UI SHALL NOT present that entry as a chat-session candidate

#### Scenario: Show unconfigured OnePiece
- **WHEN** OnePiece is present but non-selectable because its provider configuration or credential is incomplete
- **THEN** the create-session UI SHALL show a disabled OnePiece candidate with its readiness reason
- **AND** it SHALL provide an action to open OnePiece configuration

#### Scenario: Group candidate presentation
- **WHEN** the UI renders built-in OnePiece, built-in CLI Agents, and user-created API Agents
- **THEN** it SHALL present OnePiece in the VaneHub-native group, CLI Agents in the built-in CLI group, and user API Agents in a custom API group
- **AND** it SHALL order those groups as built-in CLI, VaneHub native, then custom API
- **AND** the built-in CLI group SHALL order Claude Code, Codex CLI, OpenCode, Antigravity CLI, then Gemini CLI and use the first selectable built-in CLI as the default
- **AND** grouping SHALL NOT change the stable id submitted for the selected Agent

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

### Requirement: Durable session recovery metadata
The system SHALL persist recovery status, recovery revision, state revision, history revision, and the optional active execution run identifier with each session without replacing its existing lifecycle state.

#### Scenario: Load an existing session after migration
- **WHEN** a session created before durable recovery metadata is loaded after migration and has no unresolved active state
- **THEN** it SHALL remain readable with recovery status `clean`, initialized revisions, and no fabricated active execution run

#### Scenario: List recovery metadata across runtimes
- **WHEN** sessions are listed through either the desktop or Web/mock service adapter
- **THEN** each normalized session record SHALL expose equivalent lifecycle and recovery fields

### Requirement: Deterministic session message order
Every newly persisted session message SHALL receive a stable, monotonically increasing sequence within its owning session, and historical messages SHALL be deterministically backfilled without relying on timestamp uniqueness.

#### Scenario: Order messages with equal timestamps
- **WHEN** two historical messages in the same session have the same creation timestamp
- **THEN** migration SHALL assign a deterministic relative sequence using stable persisted identity as the tie-breaker

#### Scenario: Page messages by durable order
- **WHEN** a caller pages a session transcript after sequencing is available
- **THEN** messages SHALL neither be skipped nor duplicated because multiple records share a timestamp

