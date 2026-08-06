## MODIFIED Requirements

### Requirement: Interactive CLI profile injection
The Agent Terminal runtime SHALL inject only the selected Agent's saved CLI Parameter profile values that apply to the `interactive` launch scope. For `codex-cli`, `gemini-cli`, and `opencode`, the agent's assigned policy template SHALL additionally override the specific parameters it governs, as defined by `cli-agent-permission-launch-flags`.

#### Scenario: Use interactive profile
- **WHEN** an Agent Terminal starts for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the desktop runtime SHALL load that stable agent id's saved CLI parameter selections
- **AND** it SHALL project only parameters whose launch scope includes `interactive`

#### Scenario: No session-page overrides
- **WHEN** the Agent Terminal starts
- **THEN** model, permission, reasoning, thinking, and streaming values from the removed session-page chat controls SHALL NOT override the saved CLI profile

#### Scenario: Policy template overrides its governed parameters
- **WHEN** an Agent Terminal starts for `codex-cli`, `gemini-cli`, or `opencode` with an assigned policy template
- **THEN** the parameters that template governs SHALL use the template's projected value instead of the saved CLI profile's value
- **AND** every other injected parameter SHALL come from the saved CLI profile unchanged
