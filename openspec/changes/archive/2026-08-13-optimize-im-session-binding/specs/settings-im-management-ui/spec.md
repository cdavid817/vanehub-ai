## ADDED Requirements

### Requirement: Connector-focused setup
The IM settings page SHALL present connector credentials, authorization, access posture, connection tests, lifecycle controls, and health independently from any Agent or project selection.

#### Scenario: Enable configured connector without routing defaults
- **WHEN** a connector has valid credentials or authorization and the user enables it without selecting an Agent or project
- **THEN** the page SHALL allow the request and display the asynchronous connector lifecycle result

#### Scenario: Explain session binding
- **WHEN** the user views the IM settings page
- **THEN** the page SHALL explain that projects and Agents are selected by creating a session and connecting IM from that session

#### Scenario: Open an eligible session
- **WHEN** the user chooses the settings-page action to connect a session
- **THEN** the UI SHALL navigate to or prompt for an eligible existing session without duplicating session binding controls inside connector credential rows

## MODIFIED Requirements

### Requirement: Service-backed IM settings page
The settings center SHALL include a localized IM entry and service-backed page before Usage Statistics and About.

#### Scenario: Navigate to IM settings
- **WHEN** the settings navigation renders
- **THEN** it SHALL show an icon-backed IM entry that opens the IM management page without a full-page reload

#### Scenario: Load IM settings
- **WHEN** the IM page opens
- **THEN** it SHALL load connector descriptors, current status, credential-presence metadata, and access posture through the frontend IM service without requiring session routing settings

## REMOVED Requirements

### Requirement: IM routing controls
**Reason**: A global default Agent and project conflict with session-owned Agent, project, and worktree configuration and make multi-project routing ambiguous.

**Migration**: Remove the routing form and its enablement validation from the settings page. Direct users to session-level IM binding while preserving existing connector configuration and legacy bindings.
