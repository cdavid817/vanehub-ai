## MODIFIED Requirements

### Requirement: Creation-time resume id capture
The Agent Terminal runtime SHALL persist the exact provider resume id owned by a newly created CLI-backed VaneHub session as soon as the id can be assigned or correlated, without depending solely on structured PTY output.

#### Scenario: Caller-assigned id is persisted after launch
- **WHEN** a fresh Agent Terminal starts for a stable Agent whose CLI accepts a caller-supplied session id
- **THEN** the desktop runtime SHALL generate a provider-valid id and pass it in the fresh-session invocation
- **AND** it SHALL persist that id on the owning VaneHub session after the provider process spawns successfully

#### Scenario: Provider-allocated id is correlated exactly
- **WHEN** a fresh Agent Terminal starts for a stable Agent whose CLI allocates its own session id
- **THEN** the desktop runtime SHALL capture a provider-store baseline before launch
- **AND** it SHALL persist a newly created provider id only when it is absent from the baseline and its provider metadata uniquely matches the terminal working directory

#### Scenario: Ambiguous provider records are not associated
- **WHEN** provider session discovery finds more than one new candidate that could belong to the same VaneHub session
- **THEN** the desktop runtime SHALL NOT persist any candidate as the session runtime session id
- **AND** it SHALL record a redacted warning through unified logging

#### Scenario: Start result includes resume id
- **WHEN** an Agent Terminal start for a newly created session returns a runtime session id
- **THEN** the desktop runtime SHALL persist that value on the owning session as the session runtime session id
- **AND** subsequent session list and session detail reads SHALL expose the same value

#### Scenario: Runtime event includes resume id
- **WHEN** an Agent Terminal process emits an exact runtime session id event after startup
- **THEN** the desktop runtime SHALL persist the latest non-empty value on the owning session
- **AND** the frontend SHALL refresh service-backed session state without writing persistence directly

#### Scenario: Web mock creation resume id
- **WHEN** the Web/mock runtime creates and opens a CLI-backed mock session
- **THEN** it SHALL assign or preserve deterministic mock runtime session id metadata through the Agent service contract

### Requirement: Resume id based terminal restore
The Agent Terminal runtime SHALL use the owning VaneHub session's persisted runtime session id as the provider resume id when opening a CLI terminal without a retained live process, and SHALL NOT substitute a provider-global or working-directory "most recent" session.

#### Scenario: Reopen uses persisted resume id
- **WHEN** a user selects a CLI-backed session whose prior process is closed and whose session record has a runtime session id
- **THEN** the desktop runtime SHALL pass that exact id to the provider-specific resume invocation for the session's stable Agent id
- **AND** the restored CLI process SHALL be associated with the same VaneHub session id

#### Scenario: Retained process takes precedence
- **WHEN** a session has both a retained live terminal process and a persisted runtime session id
- **THEN** the desktop runtime SHALL attach to the retained process
- **AND** it SHALL NOT spawn a provider resume invocation for the same session

#### Scenario: Missing resume id starts fresh
- **WHEN** a CLI-backed session has no retained live process and no persisted runtime session id
- **THEN** the desktop runtime SHALL start a fresh provider CLI process for the session's stable Agent id
- **AND** it SHALL NOT invoke a provider-global or working-directory "resume most recent" operation
