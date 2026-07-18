## ADDED Requirements

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
