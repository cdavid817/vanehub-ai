## ADDED Requirements

### Requirement: Safe CLI Skill mount roots
The system SHALL preflight every existing component of a CLI Agent's configured Skill mount root without following linked components before it creates, repairs, or migrates a managed per-Skill link.

#### Scenario: Use an existing normal mount root
- **WHEN** every existing component of the configured mount root is a normal directory
- **THEN** the system SHALL create or repair the requested managed per-Skill link through the existing binding transaction

#### Scenario: Create an absent normal mount root
- **WHEN** the configured mount root or one of its normal descendants does not exist and no existing ancestor is linked
- **THEN** the system SHALL create the required normal directories before creating the managed per-Skill link

#### Scenario: Reject a live external directory link
- **WHEN** the configured mount root or an existing component below the canonical scope root is a symlink, junction, or reparse point that resolves to a directory
- **THEN** the system SHALL reject the binding with an actionable error identifying the stable Agent id
- **AND** SHALL NOT follow, delete, replace, or write through the external directory link

#### Scenario: Reject a broken directory link
- **WHEN** the configured mount root or an existing component below the canonical scope root is a symlink, junction, or reparse point whose target is missing or unavailable
- **THEN** the system SHALL reject the binding with an actionable broken-link error identifying the stable Agent id
- **AND** SHALL NOT delete or replace the broken link

#### Scenario: Preserve state after mount-root rejection
- **WHEN** mount-root preflight rejects a CLI Skill assignment
- **THEN** the system SHALL leave the Skill source, current CLI/API Agent assignments, SQLite records, external link, and external target unchanged

### Requirement: Agent-specific Skill binding diagnostics
The system SHALL write CLI Skill bind and unbind results through the unified logging service with safe stable-Agent context and without raw home or external target paths.

#### Scenario: Record rejected mount-root binding
- **WHEN** a CLI Skill binding fails mount-root preflight
- **THEN** the unified error log SHALL include the binding action, Skill id, and stable Agent id
- **AND** SHALL NOT include the absolute mount-root path or external link target
