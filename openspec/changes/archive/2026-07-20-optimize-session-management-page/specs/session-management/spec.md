## ADDED Requirements

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
