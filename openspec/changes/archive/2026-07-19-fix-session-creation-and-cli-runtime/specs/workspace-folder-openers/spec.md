## ADDED Requirements

### Requirement: Configure opening-method preference without launch

The system SHALL allow users to reorder opening methods and select the default opening method without launching an external application.

#### Scenario: User changes default opener

- **WHEN** the user changes the opening-method dropdown selection in management UI
- **THEN** the system SHALL persist the selected default opener
- **AND** it SHALL NOT launch the selected opener.

#### Scenario: Opener dropdown collapses on outside interaction

- **WHEN** the opening-method dropdown is open
- **AND** the user interacts outside the dropdown
- **THEN** the dropdown SHALL collapse without launching an opener.
