## ADDED Requirements

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
