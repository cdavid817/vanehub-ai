## ADDED Requirements

### Requirement: Connector-scoped session IM selection
The session information panel SHALL let the user select among healthy configured built-in connectors and SHALL display and mutate access for the selected or bound connector only.

#### Scenario: Select an available connector
- **WHEN** an unbound session has more than one healthy configured connector
- **THEN** the information panel SHALL offer each connector by its localized name
- **AND** selecting one SHALL show that connector's persisted access state without changing another connector

#### Scenario: Enable and pair the selected connector
- **WHEN** the user enables access for a selected connector and begins pairing
- **THEN** both operations SHALL target the same stable connector id
- **AND** the UI SHALL prevent a stale selection change from pairing a different connector

#### Scenario: Display an existing binding
- **WHEN** a session already has a binding
- **THEN** the information panel SHALL select the bound connector and display its connector-specific access and lifecycle state
- **AND** connector selection SHALL remain unavailable until the binding is removed or replaced

#### Scenario: No connector is ready
- **WHEN** no built-in connector is both configured, enabled, and connected
- **THEN** the information panel SHALL keep pairing unavailable
- **AND** connector configuration and lifecycle controls SHALL remain available in global settings

#### Scenario: Use the Web mock runtime
- **WHEN** connector selection and access are used in the Web/mock runtime
- **THEN** each session and connector pair SHALL retain deterministic isolated state through the same typed service contract
