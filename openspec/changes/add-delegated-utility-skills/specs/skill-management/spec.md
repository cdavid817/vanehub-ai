## ADDED Requirements

### Requirement: Utility delegation management metadata
Skill management responses SHALL expose a Utility's declared and effective tool capabilities, requested and effective limits, trust, assignment eligibility, delegation availability, unavailable reason, use count, last-used time, and bounded delegation-history summary.

#### Scenario: Eligible Utility listed
- **WHEN** an effective trusted Utility is valid for delegation
- **THEN** management responses SHALL identify it as available and distinguish declared capabilities from effective capped capabilities

#### Scenario: Utility metadata invalid
- **WHEN** a Utility declares an unknown capability or invalid limit
- **THEN** management responses SHALL retain the Skill in inventory with a safe delegation-unavailable reason

#### Scenario: Role metadata unchanged
- **WHEN** a Role Skill is listed
- **THEN** Utility delegation fields SHALL be absent or explicitly not applicable rather than implying it can be delegated

### Requirement: Utility assignment eligibility
The system SHALL allow Utility assignment to native API Agents that support delegation and SHALL reject assignment as a delegated tool to unsupported CLI Agents or API runtimes without the native delegation capability.

#### Scenario: Assign Utility to native API Agent
- **WHEN** a user assigns an eligible Utility using a stable native API Agent id
- **THEN** the system SHALL persist the assignment without eagerly injecting Utility instructions

#### Scenario: Assign Utility to unsupported CLI Agent
- **WHEN** a user attempts to assign a Utility as delegated capability to a CLI Agent without a delegation adapter
- **THEN** the system SHALL reject the delegated assignment without changing existing Role mounts or bindings

#### Scenario: Existing unsupported Utility assignment
- **WHEN** an older record associates a Utility with an unsupported Agent
- **THEN** the system SHALL preserve the record for repair visibility but SHALL mark delegation unavailable and SHALL NOT advertise it to that Agent

### Requirement: Bounded Utility delegation history
The Skill management service SHALL provide bounded paginated delegation-history queries by canonical Utility id and workspace context, with filters for parent stable Agent id, status, and time range.

#### Scenario: Query Utility history
- **WHEN** a user requests the first history page for a Utility
- **THEN** the service SHALL return newest-first safe attempt summaries and a continuation cursor when more entries exist

#### Scenario: History excludes another project
- **WHEN** a Project-scoped Utility history is queried from a different workspace
- **THEN** the service SHALL not return attempts belonging only to the other canonical workspace

