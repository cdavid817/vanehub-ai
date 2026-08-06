## MODIFIED Requirements

### Requirement: Agent Terminal uses interactive profile only
The Agent Terminal runtime SHALL use the selected Agent's saved CLI Parameter profile projected with the `interactive` launch scope and SHALL NOT accept first-version session-page configuration overrides. For `codex-cli`, `gemini-cli`, and `opencode`, the parameters governed by that agent's assigned policy template (see `cli-agent-permission-launch-flags`) SHALL take precedence over the saved profile's value for those specific parameters only; every other parameter SHALL continue to come from the saved profile alone.

#### Scenario: Start terminal with interactive profile
- **WHEN** an Agent Terminal process starts for a managed CLI stable agent id
- **THEN** the native runtime SHALL load that agent id's saved profile
- **AND** it SHALL inject only arguments whose launch scope includes `interactive`

#### Scenario: Ignore removed chat controls
- **WHEN** the Agent Terminal process is built
- **THEN** the runtime SHALL NOT read session-page model, provider, permission, reasoning, thinking, or streaming selector values as launch overrides
- **AND** the persisted CLI Parameter profile SHALL remain the argument source for every parameter not governed by an assigned policy template

#### Scenario: Profile changes affect next terminal process
- **WHEN** a CLI Parameter profile is saved while a retained Agent Terminal process is live
- **THEN** the live process SHALL continue with its original arguments
- **AND** the next fresh or resume Agent Terminal process for that Agent SHALL use the newly saved profile

#### Scenario: Policy template overrides a governed parameter
- **WHEN** an Agent Terminal starts for `codex-cli`, `gemini-cli`, or `opencode` and its assigned policy template governs a parameter also present in the saved CLI Parameter profile
- **THEN** the launch SHALL use the value the policy template projects for that parameter, not the saved profile's value
