## ADDED Requirements

### Requirement: Agent Terminal uses interactive profile only
The Agent Terminal runtime SHALL use the selected Agent's saved CLI Parameter profile projected with the `interactive` launch scope and SHALL NOT accept first-version session-page configuration overrides.

#### Scenario: Start terminal with interactive profile
- **WHEN** an Agent Terminal process starts for a managed CLI stable agent id
- **THEN** the native runtime SHALL load that agent id's saved profile
- **AND** it SHALL inject only arguments whose launch scope includes `interactive`

#### Scenario: Ignore removed chat controls
- **WHEN** the Agent Terminal process is built
- **THEN** the runtime SHALL NOT read session-page model, provider, permission, reasoning, thinking, or streaming selector values as launch overrides
- **AND** the persisted CLI Parameter profile SHALL remain the single first-version user-controlled argument source

#### Scenario: Profile changes affect next terminal process
- **WHEN** a CLI Parameter profile is saved while a retained Agent Terminal process is live
- **THEN** the live process SHALL continue with its original arguments
- **AND** the next fresh or resume Agent Terminal process for that Agent SHALL use the newly saved profile
