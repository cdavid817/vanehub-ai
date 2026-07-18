## ADDED Requirements

### Requirement: Session category entity
The system SHALL provide durable user-defined session categories with stable id, display name, sort order, created timestamp, and updated timestamp fields.

#### Scenario: Create category
- **WHEN** a user creates a category with a non-empty unique display name
- **THEN** the system SHALL persist the category and return its stable id and metadata

#### Scenario: Reject invalid category name
- **WHEN** a user creates or renames a category with an empty name or a duplicate name
- **THEN** the system SHALL reject the request without mutating existing category assignments

### Requirement: Session category assignment
The system SHALL allow each session to belong to zero or one user-defined category.

#### Scenario: Assign session to category
- **WHEN** a user moves a session to an existing category
- **THEN** the system SHALL persist that session's category id and refresh its updated timestamp

#### Scenario: Move session to uncategorized
- **WHEN** a user removes a session from its category
- **THEN** the system SHALL clear the session category id and keep the session visible in the uncategorized group

#### Scenario: Preserve archived category assignment
- **WHEN** a categorized session is archived or restored
- **THEN** the system SHALL preserve its category assignment

### Requirement: Category management parity
The desktop and Web runtimes SHALL expose the same category list and assignment service contract.

#### Scenario: Desktop category operations
- **WHEN** React creates, lists, renames, deletes, or assigns categories in desktop mode
- **THEN** it SHALL call the frontend service interface and the Tauri adapter SHALL route the request to Rust commands

#### Scenario: Web category operations
- **WHEN** the application runs through the Web/mock adapter
- **THEN** category operations SHALL use deterministic mock state with the same request and response shapes
