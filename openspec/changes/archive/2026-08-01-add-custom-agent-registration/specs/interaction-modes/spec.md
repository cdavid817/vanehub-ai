## ADDED Requirements

### Requirement: Direct API interaction mode
The system SHALL support a direct API interaction mode for agents that operate by calling a provider's LLM API directly over HTTP rather than spawning a local CLI process, browser session, or native desktop window.

#### Scenario: Start direct API mode workflow
- **WHEN** a user starts a workflow with an agent that supports the direct API interaction mode
- **THEN** the system routes the workflow through the direct API mode execution path

#### Scenario: Direct API mode requires a stored credential
- **WHEN** a workflow is started for an agent using the direct API interaction mode
- **THEN** the system SHALL confirm a stored provider credential exists before starting generation

## MODIFIED Requirements

### Requirement: Interaction task lifecycle alignment
Interaction mode lifecycle reporting SHALL align with the common observable operation model while preserving mode-specific session details behind runtime adapters.

#### Scenario: Browser operation lifecycle
- **WHEN** a browser interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping browser-specific readiness details scoped to the browser adapter

#### Scenario: Native desktop operation lifecycle
- **WHEN** a native desktop interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping OS/window-specific details scoped to the native adapter

#### Scenario: Direct API operation lifecycle
- **WHEN** a direct API interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping provider-request-specific details scoped to the API adapter
