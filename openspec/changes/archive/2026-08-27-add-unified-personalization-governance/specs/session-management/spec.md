# session-management Delta Specification

## ADDED Requirements

### Requirement: Session personalization mode
The system SHALL persist a `personalizationMode` value of `standard`, `project-only`, or `temporary` on every session and SHALL provide that value to every generation and active participant seat as part of the session runtime context.

#### Scenario: Create a standard session by default
- **WHEN** session creation does not explicitly provide a personalization mode
- **THEN** the system SHALL create the session with `personalizationMode = standard`

#### Scenario: Create a project-only session
- **WHEN** session creation provides `personalizationMode = project-only` and a resolvable local or remote workspace
- **THEN** the system SHALL persist the mode and workspace identity
- **AND** later generations SHALL resolve project-only memory behavior through the personalization service

#### Scenario: Reject project-only without a workspace
- **WHEN** session creation provides `personalizationMode = project-only` without a resolvable workspace
- **THEN** desktop and Web/mock service boundaries SHALL reject the request without creating the session
- **AND** SHALL return a typed localized validation reason

#### Scenario: Create a temporary session
- **WHEN** session creation provides `personalizationMode = temporary`
- **THEN** the system SHALL persist the mode
- **AND** later generations SHALL suppress VaneHub long-term memory read, write, and extraction while preserving current-session history and runtime-owned internal compaction

#### Scenario: Persist mode across restart and lifecycle operations
- **WHEN** a session is restarted, selected, archived, unarchived, renamed, pinned, or restored after application restart
- **THEN** its personalization mode SHALL remain unchanged unless an explicit supported session update changes it

#### Scenario: Migrate an existing session
- **WHEN** a session persisted before this field existed is read
- **THEN** the system SHALL project and persist `personalizationMode = standard`
- **AND** the session SHALL remain readable in desktop and Web/mock runtimes

#### Scenario: Propagate mode to a multi-Agent session
- **WHEN** a multi-Agent session starts a turn for any participant seat
- **THEN** the system SHALL provide the session's common personalization mode and workspace context to the resolver
- **AND** SHALL resolve Agent-specific policy using that seat's stable Agent id

#### Scenario: Propagate mode to a worktree session
- **WHEN** a session operates in a Git worktree
- **THEN** the personalization resolver SHALL use the effective worktree/project workspace identity defined by the workspace identity service
- **AND** SHALL retain the selected personalization mode

#### Scenario: Preserve Web runtime parity
- **WHEN** sessions are created, listed, restored, or used in Web/mock mode
- **THEN** the adapter SHALL preserve the same personalization-mode values, validation, defaults, and runtime context shape
