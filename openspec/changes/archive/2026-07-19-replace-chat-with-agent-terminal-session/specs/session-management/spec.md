## ADDED Requirements

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
