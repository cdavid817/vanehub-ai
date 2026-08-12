# cli-parameter-management Specification

## Purpose
Defines safe, typed, persisted CLI launch-parameter profiles and their application across settings, Web/mock preview, interactive launches, and provider chat processes.
## Requirements
### Requirement: Managed CLI parameter profiles
The system SHALL provide one typed launch-parameter profile for each managed CLI stable agent id: `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`.

#### Scenario: List managed profiles
- **WHEN** the CLI Parameter Management page loads
- **THEN** the system SHALL return profiles for the five managed stable agent ids in their configured display order
- **AND** each profile SHALL contain definitions, effective selections, defaults, and a safe argument preview

#### Scenario: Reject unknown agent profile
- **WHEN** a client requests or saves a parameter profile for an unknown agent id
- **THEN** the service SHALL reject the request without persisting any selection

### Requirement: Typed and documented parameter catalog
Every exposed CLI parameter SHALL be defined by a backend-authoritative catalog entry with a stable parameter id, literal provider flag, control kind, localized name key, localized detailed-description key, default value, launch scope, risk classification, and allowed values when applicable. The `model` parameter for all four managed CLIs SHALL use a composite control that presents known catalog values in a dropdown alongside a free-text field for arbitrary model identifiers.

#### Scenario: Render composite model parameter
- **WHEN** a catalog entry has control kind `custom-text` with known enum values
- **THEN** the page SHALL render a dropdown containing the known catalog values plus a "Custom…" option
- **AND** when "Custom…" is selected, a free-text input SHALL appear for entering any model identifier

#### Scenario: Select known catalog model value
- **WHEN** a user selects a known catalog model value from the dropdown (e.g., "sonnet" for Claude Code)
- **THEN** the system SHALL behave identically to the current enum control

#### Scenario: Enter custom model value
- **WHEN** a user selects "Custom…" and enters `deepseek-chat` in the free-text field
- **THEN** the system SHALL save the value `deepseek-chat` as the model parameter selection

#### Scenario: Render enum parameter
- **WHEN** a catalog entry has control kind `enum`
- **THEN** the page SHALL render a single-select dropdown using only the catalog's allowed values
- **AND** it SHALL show the localized description for the selected value

#### Scenario: Render boolean parameter
- **WHEN** a catalog entry has control kind `boolean`
- **THEN** the page SHALL render an accessible switch that controls whether the mapped provider flag is effective

#### Scenario: Render repeatable enum parameter
- **WHEN** a catalog entry has control kind `multi-enum`
- **THEN** the page SHALL render a multi-select control using only the catalog's allowed values
- **AND** the service SHALL preserve the catalog-defined value order when producing arguments

### Requirement: Curated first-version parameter boundary
The first version SHALL expose only catalog-defined, non-secret parameters and SHALL NOT accept arbitrary raw argument strings, API keys, tokens, prompts, system prompts, or vendor flags absent from the catalog.

#### Scenario: Submit unknown parameter
- **WHEN** a save request contains an unknown parameter id or a value outside its catalog definition
- **THEN** the service SHALL reject the complete save atomically
- **AND** the previously persisted profile SHALL remain unchanged

#### Scenario: Dangerous bypass flag is requested
- **WHEN** a client attempts to save an explicit provider flag that bypasses both normal approval and sandbox controls but is not in the catalog
- **THEN** the service SHALL reject the request as unsupported

### Requirement: Explicit profile save and reset
The CLI Parameter Management page SHALL maintain per-CLI draft state and SHALL persist changes only through an explicit save action.

#### Scenario: Edit profile draft
- **WHEN** a user changes a parameter control
- **THEN** the page SHALL mark that CLI profile as having unsaved changes
- **AND** navigation to another CLI profile SHALL preserve the draft while the page remains mounted

#### Scenario: Save valid profile
- **WHEN** a user saves a valid CLI profile
- **THEN** the service SHALL persist all changed selections in one transaction or equivalent atomic Web/mock update
- **AND** the page SHALL clear the dirty state and show the returned effective profile

#### Scenario: Restore defaults
- **WHEN** a user confirms Restore Defaults for one CLI
- **THEN** the service SHALL remove that CLI's persisted overrides
- **AND** the page SHALL show catalog/provider defaults without changing another CLI profile

### Requirement: Runtime-specific persistence parity
The desktop runtime SHALL persist CLI parameter selections in SQLite, and the Web/mock runtime SHALL preserve the same service behavior using browser-local mock storage without claiming to launch local CLIs.

#### Scenario: Restore desktop selections
- **WHEN** the desktop application restarts after a valid profile was saved
- **THEN** the CLI profile SHALL be restored from SQLite and returned through the frontend service boundary

#### Scenario: Restore Web mock selections
- **WHEN** the Web runtime reloads after a valid mock profile was saved
- **THEN** the Web adapter SHALL restore the profile from its namespaced browser storage
- **AND** it SHALL NOT access SQLite or a local executable

### Requirement: Provider-specific argument injection
The native runtime SHALL convert logical selections into distinct argv tokens through the selected provider's argument builder and SHALL place those tokens according to the provider's interactive, fresh-chat, and resume command grammar.

#### Scenario: Start interactive CLI
- **WHEN** the user launches an interactive managed CLI with saved parameters applicable to the `interactive` scope
- **THEN** the native runtime SHALL inject the validated mapped tokens before spawning the process

#### Scenario: Start fresh chat CLI
- **WHEN** a new chat generation starts a provider CLI process with saved parameters applicable to the `chat` scope
- **THEN** the native runtime SHALL inject those selections while preserving the provider's required structured-output and prompt-delivery contract

#### Scenario: Resume provider session
- **WHEN** a chat generation resumes a provider session
- **THEN** the provider builder SHALL place saved selections in positions accepted by the resume grammar
- **AND** it SHALL preserve the native session id and stdin/prompt contract

### Requirement: VaneHub-owned arguments remain reserved
The system SHALL keep provider subcommands, structured output flags, prompt transport, session/resume identifiers, and stdin markers under native runtime ownership and SHALL NOT expose them as editable profile parameters.

#### Scenario: Selection conflicts with reserved argument
- **WHEN** a submitted logical selection would replace or invalidate a VaneHub-owned argument
- **THEN** native validation SHALL reject the selection before process creation
- **AND** it SHALL NOT rely on last-argument-wins behavior

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

### Requirement: Saved changes affect only future processes
Saving or resetting a CLI profile SHALL affect child processes spawned after the successful mutation and SHALL NOT restart, signal, or mutate an already running CLI process.

#### Scenario: Save during active generation
- **WHEN** a profile is saved while a provider process is streaming output
- **THEN** the active process SHALL continue with its original arguments
- **AND** the next process spawn SHALL read the newly saved profile

### Requirement: Safe effective argument preview
The settings page SHALL show the validated user-controlled argument segment as separate escaped tokens and SHALL omit prompts, session identifiers, secrets, and other runtime-owned values.

#### Scenario: Display preview after save
- **WHEN** a profile is loaded or successfully saved
- **THEN** the page SHALL display the service-returned effective user argument tokens
- **AND** the preview SHALL NOT be presented as a shell command to execute

### Requirement: Localized and theme-consistent page
The CLI Parameter Management page SHALL provide aligned Simplified Chinese and English UI resources and SHALL render through shared semantic tokens in both `futuristic` and `minimal` themes.

#### Scenario: Switch locale
- **WHEN** the page renders in `zh-CN` or `en`
- **THEN** page labels, parameter descriptions, value descriptions, warnings, validation states, and actions SHALL use the active locale
- **AND** literal CLI flags, provider names, and stable ids MAY remain untranslated

#### Scenario: Switch theme
- **WHEN** the active theme changes between `futuristic` and `minimal`
- **THEN** all parameter controls, descriptions, warnings, previews, and action states SHALL remain readable and usable without page-specific theme branches

### Requirement: CLI parameter management uses branded CLI identity

The CLI parameter management settings page SHALL show the branded icon for each managed CLI.

#### Scenario: CLI parameter agent list shows tool icons

- **WHEN** the CLI parameter management page lists managed CLI profiles
- **THEN** each profile entry SHALL render the corresponding branded CLI icon from the stable agent id.

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

### Requirement: Custom-text parameter control kind
The parameter catalog SHALL support a `custom-text` control kind that combines a dropdown of known values with an optional free-text input. This control kind SHALL be used for parameters where the provider accepts both known values and arbitrary identifiers.

#### Scenario: Validation accepts known enum values
- **WHEN** a `custom-text` parameter receives a value matching one of the known catalog entries
- **THEN** validation SHALL accept the value

#### Scenario: Validation accepts arbitrary non-empty values
- **WHEN** a `custom-text` parameter receives a value not in the known catalog entries
- **THEN** validation SHALL accept the value provided it is non-empty and contains no control characters

#### Scenario: Validation rejects control characters
- **WHEN** a `custom-text` parameter receives a value containing control characters (e.g., newlines, null bytes)
- **THEN** validation SHALL reject the value

#### Scenario: Validation rejects empty values
- **WHEN** a `custom-text` parameter receives an empty or whitespace-only string
- **THEN** validation SHALL reject the value or normalize it to the catalog default

### Requirement: All four managed CLIs use custom-text for model parameter
The `model` parameter for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode` SHALL change from `Enum` control to `custom-text` control, preserving all existing known values as the dropdown options.

#### Scenario: Claude Code model parameter
- **WHEN** the `claude-code` model parameter catalog is loaded
- **THEN** SHALL have control kind `custom-text`
- **AND** SHALL present known values `default`, `sonnet`, `opus`, `haiku` in the dropdown
- **AND** SHALL accept arbitrary model identifiers via free-text input

#### Scenario: Codex CLI model parameter
- **WHEN** the `codex-cli` model parameter catalog is loaded
- **THEN** SHALL have control kind `custom-text`
- **AND** SHALL present known values `default`, `gpt-5.5`, `gpt-5.4`, `gpt-5.2-codex`, `gpt-5.1-codex-max` in the dropdown
- **AND** SHALL accept arbitrary model identifiers via free-text input

#### Scenario: Gemini CLI model parameter
- **WHEN** the `gemini-cli` model parameter catalog is loaded
- **THEN** SHALL have control kind `custom-text`
- **AND** SHALL present known values `default`, `gemini-2.5-pro`, `gemini-2.5-flash` in the dropdown
- **AND** SHALL accept arbitrary model identifiers via free-text input

#### Scenario: OpenCode model parameter (no model parameter currently)
- **WHEN** the `opencode` parameter catalog is loaded
- **THEN** the catalog SHALL remain as-is with agent/thinking/autoApprove parameters only
- **AND** OpenCode model discovery SHALL rely solely on native config file reading

### Requirement: Argument preview renders custom model values directly
When a `custom-text` parameter has a value that is not in the known enum list, the argument preview SHALL render it as-is in the CLI flag sequence.

#### Scenario: Custom model value in argument preview
- **WHEN** Claude Code has model value `deepseek-chat` and the preview is generated for the `chat` scope
- **THEN** the argument preview SHALL include `--model deepseek-chat`

#### Scenario: Known model value in argument preview (unchanged)
- **WHEN** Claude Code has model value `sonnet` and the preview is generated
- **THEN** the argument preview SHALL include `--model sonnet` as before

#### Scenario: Default model value omitted from preview (unchanged)
- **WHEN** any CLI has model value `default` and the preview is generated
- **THEN** the argument preview SHALL omit the `--model` flag entirely

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

### Requirement: Audited user-editable CLI parameter catalog
The user-editable CLI parameter catalog SHALL match the current supported launch arguments and meanings for Claude Code, Codex CLI, OpenCode, Antigravity CLI, and Gemini CLI, while policy-governed arguments remain managed only by Agent Policies.

#### Scenario: Compare frontend and native catalogs
- **WHEN** a managed CLI parameter profile is loaded in desktop or Web mode
- **THEN** both runtimes SHALL expose the same parameter ids, controls, launch scopes, defaults, flags, known values, and risk semantics

#### Scenario: Describe a managed parameter
- **WHEN** a managed parameter is displayed in any supported locale
- **THEN** its label and description SHALL state the effect of the actual emitted CLI argument
- **AND** known values SHALL reflect current supported aliases or choices without preventing a valid custom model value

#### Scenario: Keep policy controls single-sourced
- **WHEN** an argument controls approval, sandboxing, or another Agent policy
- **THEN** the CLI Parameters page SHALL omit that argument
- **AND** the effective argument preview SHALL continue to receive it from the Agent policy mapping when applicable

### Requirement: Policy-governed controls are not user-editable CLI profile fields
Editable CLI profiles SHALL exclude every field that directly selects execution permission, approval behavior, automatic approval, or sandbox posture for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`.

#### Scenario: Load any managed profile
- **WHEN** the CLI Parameter Management page loads a managed CLI profile
- **THEN** its editable definitions SHALL omit policy-governed security controls
- **AND** the page SHALL direct users to Agent Policies to change that behavior

#### Scenario: Submit a removed security field
- **WHEN** a client submits a removed execution, approval, automatic-approval, or sandbox field
- **THEN** the service SHALL reject the complete save atomically as an unknown parameter
