# settings-basic-configuration-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Service-backed basic configuration
The Basic Configuration page SHALL render common application settings through the shared settings provider and frontend service boundary.

#### Scenario: Display common settings controls
- **WHEN** a user opens the Basic Configuration page
- **THEN** the page SHALL display controls for application language, font size, visual theme, default folder path, and read-only Node.js environment information

#### Scenario: Update common setting
- **WHEN** a user changes language, font size, visual theme, or default folder path from the Basic Configuration page
- **THEN** the page SHALL save the setting through the shared settings provider without directly calling a Tauri command

#### Scenario: Preserve settings page layout
- **WHEN** Basic Configuration renders common settings controls
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and internal scrolling behavior

### Requirement: Basic Settings network proxy section
The Basic Configuration page SHALL provide a Network Proxy section for configuring the active runtime's outbound proxy behavior.

#### Scenario: Display network proxy controls
- **WHEN** a user opens the Basic Configuration page
- **THEN** the page SHALL display proxy URL, editable `NO_PROXY` bypass list, optional username, optional password, save, clear, test, and scan controls through the shared settings UI style

#### Scenario: Save network proxy through service boundary
- **WHEN** a user saves a network proxy setting from Basic Configuration
- **THEN** the page SHALL submit the proxy URL and bypass list through the shared settings provider or settings service without directly calling Tauri APIs

#### Scenario: Test desktop network proxy
- **WHEN** a user tests a proxy URL in the Tauri desktop runtime
- **THEN** the page SHALL show a success or failure result with user-displayable latency or error information

#### Scenario: Scan desktop local proxies
- **WHEN** a user scans for local proxies in the Tauri desktop runtime
- **THEN** the page SHALL show detected local proxy candidates as selectable controls

#### Scenario: Show Web mock limitation
- **WHEN** the Basic Configuration page runs with the Web/mock adapter
- **THEN** desktop-only test and scan actions SHALL be disabled or show a clear unavailable state

#### Scenario: Preserve settings visual styles
- **WHEN** the Network Proxy section renders in either `futuristic` or `minimal` style
- **THEN** it SHALL use existing settings layout, semantic tokens, form controls, icons, focus states, and status styles consistently with the rest of Basic Configuration

### Requirement: Localized network proxy text
The settings center SHALL render Network Proxy user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Render localized Network Proxy section
- **WHEN** the active application language is Simplified Chinese or English
- **THEN** the Network Proxy section title, description, `NO_PROXY` text, labels, placeholders, actions, errors, status text, and desktop-only limitation text SHALL render in the active locale

#### Scenario: Keep network proxy translation parity
- **WHEN** a Network Proxy translation key is added or changed
- **THEN** zh-CN and en translation resources SHALL contain matching keys with equivalent product meaning

### Requirement: Basic Settings log management section
The Basic Settings page SHALL provide a log management section for the active runtime.

#### Scenario: Display desktop log directory
- **WHEN** the Basic Settings page loads in the Tauri desktop runtime
- **THEN** it SHALL display the active log directory from the settings service

#### Scenario: Change desktop log directory
- **WHEN** a user changes the log directory from Basic Settings
- **THEN** the page SHALL save the directory through the settings service without calling Tauri APIs directly

#### Scenario: Open desktop log directory
- **WHEN** a user selects the open log directory action in the Tauri desktop runtime
- **THEN** the page SHALL request the action through the settings service

#### Scenario: Display logging policies
- **WHEN** the Basic Settings page displays log management
- **THEN** it SHALL show that retention is fixed at 30 days, archival is automatic, redaction is built in, and supported log levels are `error`, `warn`, `info`, and `debug`

#### Scenario: Disable native open action in Web runtime
- **WHEN** the Basic Settings page runs with the Web/mock adapter
- **THEN** it SHALL display the mock log path and keep the open log directory action disabled
