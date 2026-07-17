## ADDED Requirements

### Requirement: Chat configuration reaches provider invocation
The desktop chat runtime SHALL apply supported per-message model, reasoning, permission, and related CLI configuration through the selected stable agent id's provider argument builder.

#### Scenario: Apply supported message configuration
- **WHEN** a user sends a message with a configuration value supported by the active provider
- **THEN** the native provider invocation for that message SHALL contain the provider-specific mapped value
- **AND** the mapping SHALL use the stable agent id rather than display-name matching

#### Scenario: Unsupported message configuration
- **WHEN** a message contains a configuration value that has no safe mapping for the active provider
- **THEN** the runtime SHALL omit or reject that value with a concise user-displayable reason
- **AND** it SHALL NOT guess an argument or silently replace a reserved runtime token

### Requirement: Chat invocation parameter precedence
The desktop chat runtime SHALL resolve a supported per-message value before the corresponding persisted CLI profile value and SHALL resolve a persisted value before the provider default.

#### Scenario: Per-message override is temporary
- **WHEN** a message overrides a value saved in the active CLI profile
- **THEN** only the process spawned for that message SHALL use the message value
- **AND** later messages without the override SHALL continue using the persisted profile

#### Scenario: Persisted default is applied
- **WHEN** a message does not override a saved logical parameter
- **THEN** the process spawned for that message SHALL use the saved profile value

### Requirement: Chat profile changes use next-process semantics
Saving or resetting a CLI profile SHALL NOT alter a provider process that is already running and SHALL be read again before the next fresh or resume process spawn.

#### Scenario: Change profile during stream
- **WHEN** a user changes the active provider's profile while a response is streaming
- **THEN** the current response SHALL continue using its original invocation arguments
- **AND** the next message process SHALL use the newly effective profile

