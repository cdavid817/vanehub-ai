## ADDED Requirements

### Requirement: Newly created principals default to a configurable template
The system SHALL determine the policy template assigned to a newly created agent principal from a user-configurable default setting, falling back to `standard` when that setting is absent or unreadable.

#### Scenario: New agent inherits the configured default
- **WHEN** an agent principal is created for the first time and a default-template setting has been configured
- **THEN** the system SHALL assign that configured template to the new principal

#### Scenario: Missing or unreadable setting falls back to standard
- **WHEN** an agent principal is created for the first time and no default-template setting is configured, or it cannot be read
- **THEN** the system SHALL assign the `standard` template to the new principal

#### Scenario: Changing the default does not affect existing principals
- **WHEN** the default-template setting is changed after an agent principal already exists
- **THEN** the system SHALL NOT change that existing principal's already-assigned template
