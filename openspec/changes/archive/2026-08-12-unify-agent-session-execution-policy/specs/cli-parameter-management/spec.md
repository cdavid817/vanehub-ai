## MODIFIED Requirements

### Requirement: Deterministic configuration precedence
For an ordinary logical parameter supported by the active provider, the native runtime SHALL resolve an explicit per-message value before a persisted CLI profile value and SHALL resolve a persisted value before the provider default. Policy-governed execution, approval, and sandbox values SHALL instead be resolved exclusively from the Agent policy and session execution mode and SHALL take final precedence.

#### Scenario: Message value overrides persisted default
- **WHEN** a chat message supplies a supported non-security value that is also saved in the CLI profile
- **THEN** the provider invocation SHALL use the message value for that process
- **AND** the persisted profile SHALL remain unchanged

#### Scenario: No message override
- **WHEN** a chat message does not supply a supported non-security value
- **THEN** the provider invocation SHALL use the saved profile value when present or the default otherwise

#### Scenario: Policy overrides a security parameter
- **WHEN** a launch resolves an effective execution policy
- **THEN** its execution, approval, and sandbox arguments SHALL come from that policy
- **AND** neither a message nor a saved profile SHALL override them

### Requirement: Agent Terminal uses interactive profile only
The Agent Terminal runtime SHALL use the selected Agent's saved CLI Parameter profile projected with the `interactive` launch scope for all non-security parameters. It SHALL resolve execution, approval, and sandbox behavior from the Agent policy rather than the saved profile or session-page controls.

#### Scenario: Start terminal with interactive profile
- **WHEN** an Agent Terminal process starts for a managed CLI stable agent id
- **THEN** the native runtime SHALL load that agent id's saved profile
- **AND** it SHALL inject only non-security arguments whose launch scope includes `interactive`

#### Scenario: Ignore removed chat controls
- **WHEN** an Agent Terminal process is built
- **THEN** it SHALL use the Agent policy directly and SHALL NOT read a session execution mode

#### Scenario: Profile changes affect next terminal process
- **WHEN** a CLI Parameter profile is saved while a retained Agent Terminal process is live
- **THEN** the live process SHALL continue with its original ordinary arguments
- **AND** the next process SHALL use the newly saved ordinary profile values

#### Scenario: Policy template overrides a governed parameter
- **WHEN** an Agent Terminal starts for any managed CLI
- **THEN** the launch SHALL use values projected from the Agent policy for every execution, approval, or sandbox parameter

### Requirement: Antigravity CLI parameter catalog
The backend-authoritative editable catalog SHALL define Antigravity CLI parameters for model selection (`--model`), reasoning effort (`--effort`), and agent selection (`--agent`). Execution mode, terminal sandbox, prompt transport, output format, conversation identity, and dangerous bypass flags SHALL remain runtime-owned and SHALL NOT be editable profile parameters.

#### Scenario: Load the Antigravity parameter catalog
- **WHEN** the `antigravity-cli` parameter catalog is loaded for settings
- **THEN** it SHALL contain entries for `--model`, `--effort`, and `--agent`
- **AND** it SHALL NOT contain editable entries for `--mode` or `--sandbox`

#### Scenario: Managed invocation arguments are absent from the catalog
- **WHEN** the `antigravity-cli` editable catalog is loaded
- **THEN** it SHALL NOT contain `-p`, `--output-format`, or `--conversation`

#### Scenario: The permission bypass flag is absent from the catalog
- **WHEN** the `antigravity-cli` editable catalog is loaded
- **THEN** it SHALL NOT contain a flag whose name contains `dangerously`

#### Scenario: Preview reflects saved selections
- **WHEN** a user saves a non-default Antigravity reasoning-effort value
- **THEN** the returned safe argument preview SHALL include `--effort` with that value

## ADDED Requirements

### Requirement: Policy-governed controls are not user-editable CLI profile fields
Editable CLI profiles SHALL exclude every field that directly selects execution permission, approval behavior, automatic approval, or sandbox posture for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`.

#### Scenario: Load any managed profile
- **WHEN** the CLI Parameter Management page loads a managed CLI profile
- **THEN** its editable definitions SHALL omit policy-governed security controls
- **AND** the page SHALL direct users to Agent Policies to change that behavior

#### Scenario: Submit a removed security field
- **WHEN** a client submits a removed execution, approval, automatic-approval, or sandbox field
- **THEN** the service SHALL reject the complete save atomically as an unknown parameter
