## MODIFIED Requirements

### Requirement: Service-backed basic configuration
The Basic Configuration page SHALL render common application settings through the shared settings provider and frontend service boundary.

#### Scenario: Display common settings controls
- **WHEN** a user opens the Basic Configuration page
- **THEN** the page SHALL display controls for application language, font size, visual theme, default folder path, and read-only Node.js environment information

#### Scenario: Display every supported application locale
- **WHEN** the application-language control renders
- **THEN** it SHALL present `zh-CN`, `en`, `zh-TW`, `ja`, and `ko` from the supported-locale registry in deterministic order
- **AND** each option SHALL have a recognizable localized label rather than a binary Chinese-or-English fallback label

#### Scenario: Label size and theme options for people
- **WHEN** the font-size or theme control renders
- **THEN** each option SHALL carry a localized human label (for example small, default, large) derived from the option registry rather than a raw pixel value or a hard-coded per-theme branch

#### Scenario: Update common setting
- **WHEN** a user changes language, font size, visual theme, or default folder path from the Basic Configuration page
- **THEN** the page SHALL save the setting through the shared settings provider without directly calling a Tauri command

#### Scenario: Preserve settings page layout
- **WHEN** Basic Configuration renders common settings controls in any supported locale
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and internal scrolling behavior
- **AND** the language selector SHALL remain readable and operable at desktop and narrow viewport widths
- **AND** only the page's content area SHALL scroll: no screen-reader-only label or other positioned descendant SHALL extend the document so that the window itself scrolls

### Requirement: Basic Settings log management section
The Basic Settings page SHALL provide a log management section for the active runtime.

#### Scenario: Display desktop log directory
- **WHEN** the Basic Settings page loads in the Tauri desktop runtime
- **THEN** it SHALL display the active log directory from the settings service

#### Scenario: Change desktop log directory
- **WHEN** a user changes the log directory from Basic Settings
- **THEN** the page SHALL save the directory through the settings service without calling Tauri APIs directly
- **AND** the field SHALL commit on Enter as well as on blur, offer the native directory picker where one exists, and acknowledge a successful save briefly without resizing the row

#### Scenario: Open desktop log directory
- **WHEN** a user selects the open log directory action in the Tauri desktop runtime
- **THEN** the page SHALL request the action through the settings service

#### Scenario: Display logging policies
- **WHEN** the Basic Settings page displays log management
- **THEN** it SHALL show that retention is fixed at 30 days, archival is automatic, redaction is built in, and supported log levels are `error`, `warn`, `info`, and `debug`

#### Scenario: Disable native open action in Web runtime
- **WHEN** the Basic Settings page runs with the Web/mock adapter
- **THEN** it SHALL display the mock log path and keep the open log directory action disabled

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

#### Scenario: Keep implementation notes out of the page
- **WHEN** the data management group renders
- **THEN** it SHALL show the database location and the open action only
- **AND** SHALL NOT display developer-facing storage notes such as which table or browser storage backs the settings

#### Scenario: Refresh runtime information on demand
- **WHEN** the Node.js environment panel renders
- **THEN** it SHALL offer a re-detect action that refreshes the read-only information through the settings provider

#### Scenario: Preserve service-backed common settings
- **WHEN** a user changes language, font size, visual theme, default folder path, log directory, network proxy, launch-on-startup, or floating-assistant state
- **THEN** the page SHALL save through the relevant frontend service or settings provider without directly calling Tauri APIs

#### Scenario: Preserve responsive settings layout
- **WHEN** Basic Configuration renders on desktop or narrower viewports
- **THEN** setting rows SHALL keep stable spacing, readable text, non-overlapping controls, and internal page scrolling consistent with the settings center shell

### Requirement: Basic Configuration startup controls
The Basic Configuration page SHALL expose launch-on-startup controls through the settings provider.

#### Scenario: Show startup control in Basic Configuration
- **WHEN** Basic Configuration renders
- **THEN** it SHALL include a localized launch-on-startup control with current state, disabled state, and concise runtime-specific helper text
- **AND** the disabled state SHALL follow the settings response's explicit startup capability flag

#### Scenario: Report startup save failure
- **WHEN** saving launch-on-startup fails
- **THEN** Basic Configuration SHALL show localized user feedback and report a durable client diagnostic through the service boundary

### Requirement: Folder-opener settings section
The Basic Configuration page SHALL provide a service-backed workspace-defaults group for choosing the default opener and progressively disclosing detected-program management, enabled openers, ordering, and bounded discovery.

#### Scenario: Display default opener immediately
- **WHEN** a user opens Basic Configuration
- **THEN** the workspace-defaults group SHALL display the current default opener without requiring expansion of opener management

#### Scenario: Show an explicit empty default state
- **WHEN** no enabled opener is available on the host
- **THEN** the default-opener select SHALL show a localized empty-state option and be disabled
- **AND** SHALL NOT render as a blank control

#### Scenario: Display supported opener status
- **WHEN** a user expands opener management
- **THEN** the page SHALL list all supported opener ids with localized name, recognizable icon, availability state, and resolved version, edition, or executable path when provided
- **AND** availability SHALL be presented with the shared status pill so an installed program is distinguishable from a missing or unsupported one at a glance

#### Scenario: Present opener management without nested frames
- **WHEN** opener management renders inside the workspace-defaults group
- **THEN** its disclosure SHALL align with the group's setting rows rather than rendering as a framed card inside the framed group

#### Scenario: Keep reorder controls outside the checkbox label
- **WHEN** an opener card renders its enable checkbox and reorder buttons
- **THEN** the buttons SHALL NOT be descendants of the checkbox's label element, so assistive technology announces them as separate controls

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

### Requirement: Default project directory setting row
The Basic Configuration workspace-defaults group SHALL expose the existing default folder path through the shared settings provider.

#### Scenario: Save default project directory
- **WHEN** a user changes and commits the default project directory field
- **THEN** the page SHALL save `defaultFolderPath` through the shared settings provider
- **AND** SHALL NOT call a Tauri command directly

#### Scenario: Commit on Enter and acknowledge the save
- **WHEN** a user presses Enter in the field or the field loses focus with a changed value
- **THEN** the page SHALL commit once, show a brief localized saved acknowledgement, and show the service's error inline when the save is refused
- **AND** a stored path that only differs from the draft by its display normalization SHALL NOT be re-saved

#### Scenario: Pick the directory natively
- **WHEN** the runtime offers a directory picker and the user activates the browse action
- **THEN** the page SHALL request the picker through the settings provider and commit the chosen directory
- **AND** cancelling the picker SHALL leave the stored value unchanged

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

#### Scenario: Finish the reset even when one key fails
- **WHEN** resetting one key fails
- **THEN** the provider SHALL still attempt every remaining key
- **AND** SHALL report all failures together, naming the keys, instead of stopping at the first one
