## ADDED Requirements

### Requirement: Durable remote command templates
The system SHALL provide service-backed command templates with global, SSH-connection, or remote-workspace scope.

#### Scenario: Save valid template
- **WHEN** a user saves a non-empty template name and command with a supported scope
- **THEN** the runtime SHALL persist the template and return its stable id and timestamps

#### Scenario: Filter applicable templates
- **WHEN** a remote Terminal requests templates
- **THEN** the service SHALL return global templates and templates matching its connection or workspace scope

#### Scenario: Reject secret-bearing template
- **WHEN** a template contains a recognized password, token, private-key, bearer, or secret assignment
- **THEN** the system SHALL reject persistence with concise guidance

### Requirement: Explicit template insertion
The Shell UI SHALL allow a user to insert a template into the active remote PTY without treating raw interactive input as structured command history.

#### Scenario: Insert without execution
- **WHEN** a user selects the insert action
- **THEN** the system SHALL write the command text to the active PTY without appending an execution newline
- **AND** it SHALL NOT create a command-run record

#### Scenario: Interactive input is not history
- **WHEN** a user types keys or control sequences in xterm
- **THEN** the system SHALL forward them to the PTY and SHALL NOT reconstruct or persist them as command history

### Requirement: Structured quick command execution
The desktop runtime SHALL execute a quick command through an independent SSH exec channel and record its bounded lifecycle.

#### Scenario: Execute template
- **WHEN** a user quick-executes a valid template for a connected remote session
- **THEN** the runtime SHALL record the template id, immutable command snapshot, connection and session context, working directory, timestamps, status, and exit code when available

#### Scenario: Quick execution does not corrupt interactive PTY
- **WHEN** a quick command runs while an interactive Terminal is open
- **THEN** it SHALL use an independent channel and SHALL NOT inject sentinels or control text into the PTY

#### Scenario: Cancel quick command
- **WHEN** the user cancels a running quick command
- **THEN** the runtime SHALL close its exec channel, mark the run cancelled, and leave the shared transport and other channels available

### Requirement: Command run history
The system SHALL expose bounded, paginated quick-command history through the frontend service boundary.

#### Scenario: List run history
- **WHEN** the user opens history for a session or connection
- **THEN** the service SHALL return newest-first run summaries with filters and a stable pagination cursor

#### Scenario: Delete template preserves history
- **WHEN** a template is deleted after it has runs
- **THEN** prior runs SHALL retain their immutable command snapshot without requiring the template record

### Requirement: Web command simulation
The Web/mock adapter SHALL simulate template CRUD, insertion, quick execution, cancellation, and history without native SSH side effects.

#### Scenario: Execute Web template
- **WHEN** a Web user quick-executes a mock template
- **THEN** the adapter SHALL return deterministic simulated output and a completed mock run through the same service contract
