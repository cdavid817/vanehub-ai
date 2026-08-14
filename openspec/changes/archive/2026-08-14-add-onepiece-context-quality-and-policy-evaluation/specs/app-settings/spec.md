## ADDED Requirements

### Requirement: Context quality history retention setting
Application settings SHALL persist a validated local context-quality history retention window, defaulting to 30 days and supporting only the documented bounded options consistently across desktop and Web/mock runtimes.

#### Scenario: Existing installation has no saved retention value
- **WHEN** settings are loaded without a stored context-quality retention value
- **THEN** the effective retention window SHALL be 30 days

#### Scenario: User selects a supported retention value
- **WHEN** the user saves a documented retention option
- **THEN** subsequent history pruning and settings loads SHALL use that value

#### Scenario: Stored retention value is invalid
- **WHEN** a persisted or incoming retention value is outside the supported options
- **THEN** the settings boundary SHALL reject the mutation or normalize corrupted stored data to the safe default

