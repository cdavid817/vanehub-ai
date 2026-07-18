## ADDED Requirements

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
