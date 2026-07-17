## ADDED Requirements

### Requirement: Managed CLI parameter profiles
The system SHALL provide one typed launch-parameter profile for each managed CLI stable agent id: `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`.

#### Scenario: List managed profiles
- **WHEN** the CLI Parameter Management page loads
- **THEN** the system SHALL return profiles for the four managed stable agent ids in their configured display order
- **AND** each profile SHALL contain definitions, effective selections, defaults, and a safe argument preview

#### Scenario: Reject unknown agent profile
- **WHEN** a client requests or saves a parameter profile for an unknown agent id
- **THEN** the service SHALL reject the request without persisting any selection

### Requirement: Typed and documented parameter catalog
Every exposed CLI parameter SHALL be defined by a backend-authoritative catalog entry with a stable parameter id, literal provider flag, control kind, localized name key, localized detailed-description key, default value, launch scope, risk classification, and allowed values when applicable.

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
For a logical parameter supported by the active provider, the native runtime SHALL resolve an explicit per-message chat value before a persisted CLI profile value and SHALL resolve a persisted value before the VaneHub/provider default.

#### Scenario: Message value overrides persisted default
- **WHEN** a chat message supplies a supported logical value that is also saved in the CLI profile
- **THEN** the provider invocation SHALL use the message value for that process
- **AND** the persisted profile SHALL remain unchanged

#### Scenario: No message override
- **WHEN** a chat message does not supply a supported logical value
- **THEN** the provider invocation SHALL use the saved profile value when present or the default otherwise

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

