# settings-floating-assistant-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Floating assistant basic setting
The Basic Configuration page SHALL provide a localized floating-assistant setting through a frontend service boundary and SHALL reflect whether the active runtime can provide the Windows native surface.

#### Scenario: Display the desktop setting
- **WHEN** a user opens Basic Configuration in the Windows Tauri runtime
- **THEN** the page SHALL display a shared-style enable switch, a concise description of main-window close behavior, and the current persisted value

#### Scenario: Enable or disable without restart
- **WHEN** a user changes the floating-assistant switch in the Windows Tauri runtime
- **THEN** the page SHALL persist the change through the floating-assistant service and SHALL show or destroy the native window without restarting VaneHub

#### Scenario: Default to disabled
- **WHEN** no floating-assistant preference has been saved
- **THEN** Basic Configuration SHALL show the feature as disabled and normal main-window close behavior SHALL remain active

#### Scenario: Show Web runtime limitation
- **WHEN** Basic Configuration runs through the Web/mock adapter
- **THEN** the page SHALL keep the settings center usable and SHALL show a localized unavailable state instead of claiming that a native floating window is active

#### Scenario: Preserve settings style and localization
- **WHEN** the floating-assistant setting renders in `futuristic` or `minimal` style and either supported language
- **THEN** it SHALL use shared settings primitives, semantic tokens, accessible focus/disabled states, and synchronized zh-CN/en translation keys

### Requirement: Bottom-positioned floating assistant setting
The Basic Configuration page SHALL place the floating-assistant setting after other Basic Configuration sections while preserving its localized service-backed behavior.

#### Scenario: Render floating assistant at bottom
- **WHEN** Basic Configuration renders common, startup, data, network, log, runtime, and storage sections
- **THEN** the floating-assistant setting SHALL appear after those sections instead of between application settings and network proxy

#### Scenario: Preserve floating assistant service boundary
- **WHEN** a user enables or disables the floating assistant from Basic Configuration
- **THEN** the page SHALL use the floating-assistant service and SHALL NOT call Tauri APIs directly

#### Scenario: Refine floating assistant setting presentation
- **WHEN** the floating-assistant setting renders
- **THEN** it SHALL use a compact shared-style presentation with status, supported-runtime copy, and stable switch dimensions in both registered visual styles
