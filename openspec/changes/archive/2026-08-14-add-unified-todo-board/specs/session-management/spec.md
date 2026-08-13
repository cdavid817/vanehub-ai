## ADDED Requirements

### Requirement: Session execution lineage
The system SHALL persist an origin kind and optional origin identifier for Sessions created directly by a user, by a Plan attempt, or by a Scheduled Task run.

#### Scenario: Create direct Session
- **WHEN** a user directly creates a Session
- **THEN** the Session SHALL expose a user origin without a fabricated parent identifier

#### Scenario: Create child Session
- **WHEN** Plan or Scheduled Task execution creates a Session
- **THEN** the Session SHALL expose the corresponding origin kind and durable parent identifier

#### Scenario: Load legacy Session
- **WHEN** a Session predating lineage metadata is loaded
- **THEN** it SHALL remain readable and default to user origin unless an existing durable relationship proves another origin
