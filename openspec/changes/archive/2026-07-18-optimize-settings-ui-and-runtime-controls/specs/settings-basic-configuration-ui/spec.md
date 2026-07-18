## ADDED Requirements

### Requirement: Polished Basic Configuration information architecture
The Basic Configuration page SHALL organize common settings, desktop behavior, data management, network proxy, log management, runtime information, storage notes, and floating assistant controls into a scannable layout.

#### Scenario: Render optimized Basic Configuration sections
- **WHEN** a user opens Basic Configuration
- **THEN** the page SHALL group controls into clear localized sections for application preferences, startup or system behavior, data management, network proxy, logs, runtime information, storage notes, and desktop floating assistant
- **AND** the desktop floating assistant section SHALL appear after the other Basic Configuration sections

#### Scenario: Preserve service-backed common settings
- **WHEN** a user changes language, font size, visual theme, default folder path, log directory, network proxy, launch-on-startup, or floating-assistant state
- **THEN** the page SHALL save through the relevant frontend service or settings provider without directly calling Tauri APIs

#### Scenario: Preserve responsive settings layout
- **WHEN** Basic Configuration renders on desktop or narrower viewports
- **THEN** sections SHALL keep stable spacing, readable text, non-overlapping controls, and internal page scrolling consistent with the settings center shell

### Requirement: Basic Configuration startup controls
The Basic Configuration page SHALL expose launch-on-startup controls through the settings provider.

#### Scenario: Show startup control in Basic Configuration
- **WHEN** Basic Configuration renders
- **THEN** it SHALL include a localized launch-on-startup control with current state, disabled state, and concise runtime-specific helper text

#### Scenario: Report startup save failure
- **WHEN** saving launch-on-startup fails
- **THEN** Basic Configuration SHALL show localized user feedback and report a durable client diagnostic through the service boundary
