## ADDED Requirements

### Requirement: Network proxy settings model
The system SHALL include a persisted network proxy URL and editable proxy bypass list in the shared application settings model.

#### Scenario: Load default network proxy setting
- **WHEN** no persisted network proxy setting exists
- **THEN** the system SHALL provide an empty proxy URL representing direct connection
- **AND** provide a default proxy bypass list for localhost and loopback traffic

#### Scenario: Save valid desktop network proxy setting
- **WHEN** a user saves a supported proxy URL in the Tauri desktop runtime
- **THEN** the system SHALL validate and persist the URL through the settings service
- **AND** apply it to new VaneHub-managed native outbound requests and newly launched child processes

#### Scenario: Save valid desktop network proxy bypass setting
- **WHEN** a user saves a proxy bypass list in the Tauri desktop runtime
- **THEN** the system SHALL validate, normalize, and persist the bypass value through the settings service
- **AND** apply it to new VaneHub-managed native outbound requests and newly launched child processes

#### Scenario: Clear desktop network proxy setting
- **WHEN** a user clears the network proxy URL in the Tauri desktop runtime
- **THEN** the system SHALL persist direct connection mode
- **AND** stop applying proxy environment variables to newly launched child processes

#### Scenario: Reject invalid desktop network proxy setting
- **WHEN** a user saves a malformed proxy URL or unsupported proxy scheme
- **THEN** the system SHALL reject the setting without changing the active proxy configuration

#### Scenario: Reject invalid desktop network proxy bypass setting
- **WHEN** a user saves a proxy bypass value containing unsafe control characters
- **THEN** the system SHALL reject the setting without changing the active proxy bypass configuration

#### Scenario: Restore saved network proxy setting
- **WHEN** the application starts after a valid network proxy setting has been saved
- **THEN** the system SHALL restore the proxy URL and bypass list and apply them before starting new VaneHub-managed network work

#### Scenario: Preserve Web mock proxy limitation
- **WHEN** the application runs with the Web/mock settings adapter
- **THEN** the system SHALL NOT claim browser or OS traffic is routed through the saved proxy setting

### Requirement: Network proxy runtime scope
The system SHALL define network proxy application scope as VaneHub-managed traffic in the first version.

#### Scenario: Apply proxy to child processes
- **WHEN** VaneHub launches a network-capable subprocess after a proxy has been configured
- **THEN** the subprocess environment SHALL include standard proxy variables for the configured proxy URL
- **AND** include `NO_PROXY` and `no_proxy` variables for the configured bypass list

#### Scenario: Do not mutate existing processes
- **WHEN** a proxy setting changes while subprocesses are already running
- **THEN** the system SHALL NOT forcibly restart or reconfigure those running subprocesses

#### Scenario: Do not promise system-wide proxying
- **WHEN** network proxy behavior is described in settings or documentation
- **THEN** the system SHALL describe the supported scope as VaneHub-managed native requests and VaneHub-launched subprocesses, not OS-wide interception
