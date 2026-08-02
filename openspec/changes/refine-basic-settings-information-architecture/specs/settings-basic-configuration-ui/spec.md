## MODIFIED Requirements

### Requirement: Polished Basic Configuration information architecture
The Basic Configuration page SHALL organize common preferences, startup and window behavior, workspace defaults, and advanced operational configuration into a scannable intent-based layout.

#### Scenario: Render prioritized Basic Configuration groups
- **WHEN** a user opens Basic Configuration
- **THEN** the page SHALL present common preferences, startup and window behavior, and workspace defaults before advanced operational configuration
- **AND** language, font size, visual theme, default folder path, default folder opener, launch-on-startup, and floating-assistant controls SHALL be available without opening the advanced disclosure

#### Scenario: Disclose advanced configuration progressively
- **WHEN** Basic Configuration first renders
- **THEN** network proxy, logs, data management, storage notes, and runtime information SHALL be grouped in a collapsed localized advanced disclosure
- **AND** opening the disclosure SHALL expose the existing service-backed controls without changing their behavior

#### Scenario: Preserve service-backed common settings
- **WHEN** a user changes language, font size, visual theme, default folder path, log directory, network proxy, launch-on-startup, or floating-assistant state
- **THEN** the page SHALL save through the relevant frontend service or settings provider without directly calling Tauri APIs

#### Scenario: Preserve responsive settings layout
- **WHEN** Basic Configuration renders on desktop or narrower viewports
- **THEN** setting rows SHALL keep stable spacing, readable text, non-overlapping controls, and internal page scrolling consistent with the settings center shell

### Requirement: Folder-opener settings section
The Basic Configuration page SHALL provide a service-backed workspace-defaults group for choosing the default opener and progressively disclosing detected-program management, enabled openers, ordering, and bounded discovery.

#### Scenario: Display default opener immediately
- **WHEN** a user opens Basic Configuration
- **THEN** the workspace-defaults group SHALL display the current default opener without requiring expansion of opener management

#### Scenario: Display supported opener status
- **WHEN** a user expands opener management
- **THEN** the page SHALL list all supported opener ids with localized name, recognizable icon, availability state, and resolved version, edition, or executable path when provided

#### Scenario: Configure enabled openers
- **WHEN** a user changes the multi-select opener list
- **THEN** the page SHALL keep File Explorer selected as the required fallback
- **AND** SHALL save the complete preference aggregate through the service boundary

#### Scenario: Configure the default opener
- **WHEN** a user selects an enabled available opener as default
- **THEN** the page SHALL atomically save it with the enabled list
- **AND** the session toolbar SHALL observe the coherent preference change

#### Scenario: Prevent an unavailable default
- **WHEN** an opener is not installed, invalid, unsupported, or failed detection
- **THEN** the expanded management view SHALL display its status
- **AND** SHALL prevent selecting it as a new default while retaining any existing enabled selection

#### Scenario: Refresh local discovery
- **WHEN** the user activates the refresh action
- **THEN** the page SHALL show a non-blocking detection state and request a fresh bounded scan through the service boundary
- **AND** SHALL update per-opener results without changing saved preference selections

#### Scenario: Render Web preview limitations
- **WHEN** the settings section runs through the Web/mock adapter
- **THEN** it SHALL remain interactive with deterministic data
- **AND** SHALL identify native installation status and launch behavior as simulated or unavailable

## ADDED Requirements

### Requirement: Default project directory setting row
The Basic Configuration workspace-defaults group SHALL expose the existing default folder path through the shared settings provider.

#### Scenario: Save default project directory
- **WHEN** a user changes and commits the default project directory field
- **THEN** the page SHALL save `defaultFolderPath` through the shared settings provider
- **AND** SHALL NOT call a Tauri command directly

#### Scenario: Display runtime-restored default directory
- **WHEN** Basic Configuration loads after a default folder path has been persisted
- **THEN** the workspace-defaults group SHALL display the restored path in the localized setting row

### Requirement: Deliberate global settings reset
The Basic Configuration page SHALL present global reset as a low-frequency footer action with explicit confirmation.

#### Scenario: Confirm global reset
- **WHEN** a user activates the reset action
- **THEN** the page SHALL describe that application settings will return to defaults and require confirmation before invoking reset

#### Scenario: Cancel global reset
- **WHEN** a user declines the reset confirmation
- **THEN** the page SHALL leave every persisted setting unchanged
