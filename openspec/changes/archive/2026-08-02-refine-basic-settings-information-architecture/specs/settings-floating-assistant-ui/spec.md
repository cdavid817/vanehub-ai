## MODIFIED Requirements

### Requirement: Bottom-positioned floating assistant setting
The Basic Configuration page SHALL place the floating-assistant setting inside the startup and window behavior group while preserving its localized service-backed behavior.

#### Scenario: Render floating assistant at bottom
- **WHEN** Basic Configuration renders
- **THEN** the floating-assistant control SHALL appear with launch-on-startup under the localized startup and window behavior group
- **AND** SHALL remain available without opening advanced configuration

#### Scenario: Preserve floating assistant service boundary
- **WHEN** a user enables or disables the floating assistant from Basic Configuration
- **THEN** the page SHALL use the floating-assistant service and SHALL NOT call Tauri APIs directly

#### Scenario: Refine floating assistant setting presentation
- **WHEN** the floating-assistant setting renders
- **THEN** it SHALL use a compact shared-style presentation with supported-runtime copy and stable switch dimensions in both registered visual styles
