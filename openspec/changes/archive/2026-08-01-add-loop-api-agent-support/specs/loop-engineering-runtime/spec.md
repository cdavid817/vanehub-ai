## MODIFIED Requirements

### Requirement: Durable Loop definition contract
The system SHALL persist Loop definitions with a stable id, name, enabled state, local Git project path, base branch, goal, acceptance criteria, allowed and protected paths, stable Worker and Verifier Agent ids, structured verification commands, stop limits, version, and timestamps.

#### Scenario: Create valid Loop definition
- **WHEN** a user submits a valid first-phase Loop configuration
- **THEN** the system SHALL return a durable definition with a stable id and version
- **AND** it SHALL preserve stable Agent ids rather than matching display names

#### Scenario: Reject unsupported first-phase scope
- **WHEN** a definition targets a non-Git project, remote workspace, missing Agent, unsafe path scope, or invalid limit
- **THEN** the system SHALL reject the definition without starting an Agent or creating a worktree

#### Scenario: Accept a CLI-launched Worker or Verifier Agent
- **WHEN** a definition names a Worker or Verifier Agent that supports CLI interaction
- **THEN** the system SHALL accept that Agent for the role, unchanged from existing behavior

#### Scenario: Accept a trusted API Worker or Verifier Agent
- **WHEN** a definition names a Worker or Verifier Agent that only supports API interaction and has tool-use trust enabled
- **THEN** the system SHALL accept that Agent for the role

#### Scenario: Reject an untrusted API Worker or Verifier Agent
- **WHEN** a definition names a Worker or Verifier Agent that only supports API interaction and does not have tool-use trust enabled
- **THEN** the system SHALL reject the definition with an error identifying that the Agent requires tool-use trust
- **AND** it SHALL NOT start an Agent or create a worktree

### Requirement: Manual bounded Loop start
The first-phase system SHALL start a Loop only through an explicit user action and SHALL snapshot the selected definition before asynchronous work begins.

#### Scenario: Start enabled Loop
- **WHEN** a user manually starts an enabled definition with available role Agents
- **THEN** the system SHALL persist a queued run with an immutable definition snapshot
- **AND** it SHALL return a stable run or operation identifier before variable-duration preparation completes

#### Scenario: Reject concurrent run for definition
- **WHEN** a definition already has a queued, running, paused, or awaiting-acceptance run
- **THEN** the system SHALL reject another start for that definition without creating a second worktree

#### Scenario: Reject start when a role Agent is no longer eligible
- **WHEN** a user starts a definition whose Worker or Verifier Agent no longer supports CLI or trusted API interaction (for example, tool-use trust was disabled after the definition was saved)
- **THEN** the system SHALL reject the start with an error identifying the ineligible Agent
- **AND** it SHALL NOT persist a queued run or create a worktree
