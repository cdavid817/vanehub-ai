## ADDED Requirements

### Requirement: CLI management refresh feedback
The SDK and CLI management settings surfaces SHALL show a visible refresh-in-progress state while CLI detection refresh is starting or running.

#### Scenario: Refresh button shows running state
- **WHEN** a user starts CLI detection refresh from the settings UI
- **THEN** the refresh button SHALL be disabled until the refresh operation reaches a terminal state
- **AND** the refresh button SHALL show a dynamic loading indicator and refreshing label while disabled for the running refresh

#### Scenario: Refresh completion updates CLI data
- **WHEN** a CLI detection refresh operation succeeds, partially succeeds, or fails
- **THEN** the frontend SHALL refresh the listed CLI statuses through the service boundary
- **AND** the page SHALL preserve already loaded CLI cards while displaying any user-facing refresh error

### Requirement: Consistent selected-version CLI operations
The system SHALL support selected-version install, upgrade, and downgrade operations for every managed CLI definition through the same CLI package operation path.

#### Scenario: Install selected version for any managed CLI
- **WHEN** a user selects a stable version for Claude Code, OpenCode, Codex CLI, or Gemini CLI that is not currently installed and starts the package operation
- **THEN** the native runtime SHALL execute the backend-owned npm package spec for that CLI and selected version
- **AND** the operation SHALL be recorded as an install operation in the affected CLI card logs

#### Scenario: Upgrade selected version for any managed CLI
- **WHEN** a user selects a stable version newer than the detected installed version for Claude Code, OpenCode, Codex CLI, or Gemini CLI and starts the package operation
- **THEN** the native runtime SHALL execute the backend-owned npm package spec for that CLI and selected version
- **AND** the operation SHALL be recorded as an upgrade operation in the affected CLI card logs

#### Scenario: Downgrade selected version for any managed CLI
- **WHEN** a user selects a stable version older than the detected installed version for Claude Code, OpenCode, Codex CLI, or Gemini CLI and starts the package operation
- **THEN** the native runtime SHALL execute the backend-owned npm package spec for that CLI and selected version
- **AND** the operation SHALL be recorded as a downgrade operation in the affected CLI card logs

#### Scenario: Reject unsupported CLI package target
- **WHEN** a selected-version CLI package operation is requested for an agent id outside the managed CLI catalog
- **THEN** the system SHALL reject the request before executing npm
