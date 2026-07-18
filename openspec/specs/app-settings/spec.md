# app-settings Specification

## Purpose
TBD - created by archiving change complete-general-settings-i18n-font-fix. Update Purpose after archive.
## Requirements
### Requirement: Common settings model
The system SHALL manage common application settings for application language, font size, visual theme, and default folder path through a shared settings model.

#### Scenario: Load default settings
- **WHEN** no persisted common settings exist
- **THEN** the system SHALL provide valid defaults for language, font size, visual theme, and default folder path

#### Scenario: Reject invalid setting value
- **WHEN** a setting value is outside the supported values for its setting key
- **THEN** the system SHALL reject the value before applying it to the application UI

### Requirement: Settings side effects
The system SHALL apply common settings through centralized side effects owned by the settings provider.

#### Scenario: Apply language setting
- **WHEN** the application language setting changes between Chinese and English
- **THEN** the system SHALL synchronize the active i18next language with the selected value

#### Scenario: Apply font size setting
- **WHEN** the font size setting changes to 12px, 14px, 16px, or 18px
- **THEN** the system SHALL set the root `html` font size so rem-based Tailwind sizing scales with the selected value

#### Scenario: Apply visual theme setting
- **WHEN** the visual theme setting changes between futuristic and minimal styles
- **THEN** the system SHALL update the document theme attribute used by CSS variable groups

### Requirement: Settings persistence
The system SHALL persist common settings through the active runtime adapter.

#### Scenario: Persist desktop setting
- **WHEN** the application runs in the Tauri desktop runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through a Tauri command backed by SQLite storage

#### Scenario: Persist Web setting
- **WHEN** the application runs in the browser Web runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through the Web adapter without requiring a Tauri command

#### Scenario: Restore saved settings
- **WHEN** the application starts after common settings have been saved
- **THEN** the system SHALL restore and apply the saved setting values for the active runtime

### Requirement: Node.js environment display
The system SHALL expose read-only Node.js environment information for the Basic Configuration page.

#### Scenario: Node.js is available
- **WHEN** the runtime can resolve a Node.js executable and version
- **THEN** the settings page SHALL display the resolved path and version as read-only information

#### Scenario: Node.js is unavailable
- **WHEN** the runtime cannot resolve a Node.js executable or version
- **THEN** the settings page SHALL display an unavailable read-only state without blocking other settings controls

### Requirement: Logging settings model
The system SHALL include log directory and read-only logging policy values in the shared settings model.

#### Scenario: Load default logging settings
- **WHEN** no persisted logging settings exist
- **THEN** the system SHALL provide a valid default log directory and fixed first-version policies for 30-day retention, automatic archival, built-in redaction, and supported log levels

#### Scenario: Save log directory setting
- **WHEN** a user saves a log directory in the desktop runtime
- **THEN** the system SHALL persist the directory through the settings service and use it for newly written logs

#### Scenario: Reject invalid log directory
- **WHEN** the runtime cannot validate or create the requested log directory
- **THEN** the system SHALL reject the setting without changing the active log directory

#### Scenario: Restore log directory setting
- **WHEN** the application restarts after a custom log directory has been saved
- **THEN** the system SHALL restore that directory as the active log directory

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

### Requirement: Automatic archival settings
The system SHALL expose settings for automatic inactive session archival.

#### Scenario: Default archival settings
- **WHEN** no automatic archival settings have been saved
- **THEN** the system SHALL treat automatic archival as enabled with an inactivity threshold of 10 days

#### Scenario: Save archival settings
- **WHEN** a user changes automatic archival enablement or inactivity threshold
- **THEN** the system SHALL persist the settings through the existing settings service boundary

#### Scenario: Apply disabled setting
- **WHEN** automatic archival is disabled
- **THEN** the Rust background scheduler SHALL skip archival mutations while leaving manual archive operations available

