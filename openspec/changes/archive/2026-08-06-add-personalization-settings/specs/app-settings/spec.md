# app-settings Specification (Delta)

## ADDED Requirements

### Requirement: Personalization settings model
The shared application settings model SHALL include two custom-instruction text fields ("about you" and "response style"), a custom-instructions enablement toggle, and two memory-preference toggles (overall memory enablement, and tool-assisted chat extraction), persisted through the same settings storage as other common settings without a dedicated table.

#### Scenario: Load default personalization settings
- **WHEN** no personalization settings have been saved
- **THEN** the system SHALL provide empty custom-instruction fields
- **AND** SHALL treat the custom-instructions, memory-enablement, and tool-assisted-extraction toggles as enabled

#### Scenario: Save personalization settings
- **WHEN** a user changes a custom-instruction field or any of the three personalization toggles
- **THEN** the system SHALL persist the change through the existing settings service boundary
- **AND** the change SHALL apply to subsequent OnePiece generations without an application restart

#### Scenario: Reject an oversized custom-instruction field
- **WHEN** a user saves a custom-instruction field exceeding 3,000 Unicode characters
- **THEN** the system SHALL reject the value without changing the previously saved field

#### Scenario: Restore saved personalization settings
- **WHEN** the application restarts after personalization settings have been saved
- **THEN** the system SHALL restore the saved field values and toggle states for the active runtime

#### Scenario: Preserve Web mock parity
- **WHEN** personalization settings are loaded or saved through the Web/mock adapter
- **THEN** the Web adapter SHALL preserve the same field and toggle shape without accessing SQLite
