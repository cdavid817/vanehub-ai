## ADDED Requirements

### Requirement: Launch-on-startup application setting
The system SHALL include launch-on-startup in common application settings and apply it through centralized settings side effects.

#### Scenario: Load startup setting
- **WHEN** application settings are loaded
- **THEN** the settings service SHALL return a boolean launch-on-startup value with a safe default of disabled

#### Scenario: Save startup setting
- **WHEN** the launch-on-startup setting is saved
- **THEN** the settings service SHALL validate and persist the boolean value
- **AND** desktop runtime side effects SHALL remain owned by the settings/native layer

#### Scenario: Preserve Web mock parity
- **WHEN** app settings are loaded or saved in the Web/mock runtime
- **THEN** the Web adapter SHALL preserve the launch-on-startup key shape without claiming native startup registration is active
