# session-runtime-management Specification

## Purpose
Defines how durable sessions own active generation state, runtime lifecycle transitions, cancellation, cleanup, and Agent runtime diagnostics.

## Requirements
### Requirement: Session-scoped runtime ownership
The desktop runtime SHALL track active Agent generation state by session id and SHALL prevent unrelated sessions from sharing generation handles.

#### Scenario: Start generation for a session
- **WHEN** a message is sent for a non-archived session
- **THEN** the desktop runtime SHALL associate the generation handle with that session id
- **AND** the session lifecycle SHALL transition to `starting` and then `running` when execution begins

#### Scenario: Isolate concurrent sessions
- **WHEN** two sessions have independent generation state
- **THEN** stopping or completing one session SHALL NOT stop, complete, or mutate the other session's active generation

### Requirement: Runtime terminal states update sessions
The desktop runtime SHALL persist terminal runtime outcomes back to the owning session record.

#### Scenario: Generation completes
- **WHEN** a session generation completes successfully
- **THEN** the assistant message SHALL be marked `completed`
- **AND** the owning session lifecycle SHALL be set to `idle`
- **AND** the session updated timestamp SHALL be refreshed

#### Scenario: Generation fails
- **WHEN** a session generation fails before completion
- **THEN** the assistant message SHALL be marked `failed`
- **AND** the owning session lifecycle SHALL be set to `failed`
- **AND** user-displayable error context SHALL be available through the message or session details contract

#### Scenario: Generation is cancelled
- **WHEN** a user stops an active generation for a session
- **THEN** the active assistant message SHALL be marked `cancelled`
- **AND** the owning session lifecycle SHALL be set to `stopped`
- **AND** already persisted message content SHALL remain available

### Requirement: Runtime cleanup on session removal
The desktop runtime SHALL clean up active generation state when a session is archived or deleted.

#### Scenario: Archive running session
- **WHEN** an active running session is archived
- **THEN** the runtime SHALL request cancellation for that session before hiding it from the normal session list
- **AND** the active session selection SHALL be cleared
- **AND** detailed cancellation diagnostics SHALL be persisted through unified logging rather than shown as raw chat output

#### Scenario: Delete running session
- **WHEN** a running session is deleted
- **THEN** the runtime SHALL stop the active generation for that session
- **AND** the session's persisted messages SHALL be removed through the ownership relationship

### Requirement: Runtime details remain behind adapters
Session runtime implementation details SHALL remain behind the frontend service and runtime adapter boundaries.

#### Scenario: React reads runtime status
- **WHEN** React UI needs session runtime status or details
- **THEN** it SHALL call the frontend service interface
- **AND** it SHALL NOT call Tauri `invoke()` or inspect SQLite directly

#### Scenario: UI limits generation to active session
- **WHEN** the first-version UI exposes send and stop generation controls
- **THEN** those controls SHALL operate only on the active session
- **AND** the desktop runtime SHALL still isolate generation handles by session id

#### Scenario: Web runtime mirrors lifecycle
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL expose the same session lifecycle, cancellation, and message status contract using mock runtime state
