## ADDED Requirements

### Requirement: Bounded Skill configuration context
For native API Agent contexts that load a configured Skill, the system SHALL append a deterministic, bounded configuration section containing only effective non-secret values and secret-presence states required by that Skill. Configuration content SHALL be counted against the applicable Skill context budget and MUST NOT displace Agent core instructions or expose values from another Skill, workspace, session, or revision.

#### Scenario: Configured API Skill context is assembled
- **WHEN** an eligible configured Skill is loaded for a native API Agent
- **THEN** the Skill receives its own revision-bound non-secret configuration section in stable property-key order

#### Scenario: Configuration exceeds its budget
- **WHEN** serialized non-secret configuration would exceed the declared configuration context limit
- **THEN** the system fails that Skill activation with an actionable bounded error rather than silently truncating values

#### Scenario: Secret property is configured
- **WHEN** an effective configuration contains a secret property
- **THEN** prompt assembly includes at most its configured/missing state and never its value or reversible reference

### Requirement: Configuration lookup failure is isolated
Failure to resolve required Skill configuration SHALL prevent only the affected Skill activation or delegation and SHALL produce redacted diagnostics through the unified logging boundary. It MUST NOT remove Agent core instructions, unrelated valid Skills, or scoped memories.

#### Scenario: One Skill has invalid configuration
- **WHEN** prompt assembly includes multiple Skills and one has migration-required configuration
- **THEN** the invalid Skill is not activated and the remaining valid prompt sections retain deterministic ordering

