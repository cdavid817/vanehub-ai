## ADDED Requirements

### Requirement: Guided multi-Agent session creation
The system SHALL provide a responsive creation dialog for multi-Agent sessions that lets a user name the session, add or remove seats, select available role and Agent assignments, and see validation before creation.

#### Scenario: Create a valid multi-Agent session
- **WHEN** the user submits a dialog with a valid name and at least two available seats
- **THEN** the system creates the session with the selected seats
- **AND** the dialog provides a clear confirmation without hiding the created session

#### Scenario: Invalid dialog assignment
- **WHEN** a required seat assignment is missing or unavailable
- **THEN** the dialog keeps the entered data and presents a localized validation message
