## ADDED Requirements

### Requirement: Remote session SSH profile binding
The system SHALL preserve remote workspace snapshots while storing an optional operational SSH profile id and revision binding for remote Terminal use.

#### Scenario: Create remote session from profile
- **WHEN** a user creates a remote session from an SSH connection profile
- **THEN** the session SHALL store the profile id and current revision in addition to its independent host, port, user, path, display name, and URI snapshot

#### Scenario: Existing remote session migration
- **WHEN** an existing remote session predates SSH profile binding columns
- **THEN** it SHALL remain readable with its snapshot and SHALL require explicit binding before remote Terminal use

#### Scenario: Profile edit does not redirect session
- **WHEN** a bound profile changes endpoint or authentication configuration
- **THEN** the session snapshot SHALL remain unchanged and its old binding SHALL become stale rather than silently connecting to the changed target

#### Scenario: Profile deletion preserves snapshot
- **WHEN** a bound SSH profile is deleted
- **THEN** the session SHALL retain its remote workspace snapshot and SHALL require rebind before opening Terminal

#### Scenario: Rebind remote session
- **WHEN** a user explicitly rebinds a remote session to a compatible SSH profile
- **THEN** the system SHALL update only the operational profile id and revision unless the user separately confirms a workspace-target change
