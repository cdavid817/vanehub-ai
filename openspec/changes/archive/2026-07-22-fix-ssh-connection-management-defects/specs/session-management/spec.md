## ADDED Requirements

### Requirement: Remote workspace port schema upgrade
The desktop runtime SHALL add remote workspace port storage when upgrading an existing database that already applied the original remote workspace migration.

#### Scenario: Upgrade pre-SSH database
- **WHEN** a desktop database with migrations through version 23 starts against the SSH connection management release
- **THEN** migration 24 SHALL add the remote workspace history port column and session snapshot port column
- **AND** existing remote workspace and session records SHALL remain readable

#### Scenario: Initialize clean database
- **WHEN** the desktop runtime initializes a clean database
- **THEN** the final schema SHALL contain the SSH connection table and both remote workspace port columns
