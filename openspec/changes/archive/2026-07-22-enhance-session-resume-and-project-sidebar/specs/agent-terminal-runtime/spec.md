## ADDED Requirements

### Requirement: Creation-time resume id capture
The Agent Terminal runtime SHALL persist a provider resume id for a newly created CLI-backed session as soon as the id is available from the terminal start result or runtime events.

#### Scenario: Start result includes resume id
- **WHEN** an Agent Terminal start for a newly created session returns a runtime session id
- **THEN** the desktop runtime SHALL persist that value on the owning session as the session runtime session id
- **AND** subsequent session list and session detail reads SHALL expose the same value

#### Scenario: Runtime event includes resume id
- **WHEN** an Agent Terminal process emits a runtime session id event after startup
- **THEN** the desktop runtime SHALL persist the latest non-empty value on the owning session
- **AND** the frontend SHALL refresh service-backed session state without writing persistence directly

#### Scenario: Web mock creation resume id
- **WHEN** the Web/mock runtime creates and opens a CLI-backed mock session
- **THEN** it SHALL assign or preserve deterministic mock runtime session id metadata through the Agent service contract

### Requirement: Resume id based terminal restore
The Agent Terminal runtime SHALL use a persisted session runtime session id as the provider resume id when opening a CLI terminal for a session without a retained live process.

#### Scenario: Reopen uses persisted resume id
- **WHEN** a user selects a CLI-backed session whose prior process is closed and whose session record has a runtime session id
- **THEN** the desktop runtime SHALL pass that id to the provider-specific resume invocation for the session's stable agent id
- **AND** the restored CLI process SHALL be associated with the same VaneHub session id

#### Scenario: Retained process takes precedence
- **WHEN** a session has both a retained live terminal process and a persisted runtime session id
- **THEN** the desktop runtime SHALL attach to the retained process
- **AND** it SHALL NOT spawn a provider resume invocation for the same session

#### Scenario: Missing resume id starts fresh
- **WHEN** a CLI-backed session has no retained live process and no persisted runtime session id
- **THEN** the desktop runtime SHALL start a fresh provider CLI process for the session's stable agent id
