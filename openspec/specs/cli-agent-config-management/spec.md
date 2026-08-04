# cli-agent-config-management Specification

## Purpose
TBD - created by archiving change add-cli-agent-global-config-switching. Update Purpose after archive.
## Requirements
### Requirement: Supported CLI Agent configuration profiles
The system SHALL manage user-level global configuration profiles for the stable Agent ids `claude-code`, `opencode`, and `codex-cli`, and SHALL reject profile operations for unsupported Agent ids without reading or writing a CLI configuration file.

#### Scenario: List profiles for a supported Agent
- **WHEN** a user opens global configuration management for Claude Code, OpenCode, or Codex CLI
- **THEN** the system SHALL return only profiles belonging to that stable Agent id
- **AND** each profile SHALL include a stable profile id, display name, validation state, credential-presence state, and applied-state metadata

#### Scenario: Unsupported Agent requested
- **WHEN** a profile operation targets `gemini-cli`, an API Agent, or an unknown Agent id
- **THEN** the system SHALL reject the operation before accessing any global CLI configuration file

### Requirement: Bundled common-provider presets
The system SHALL provide a bundled, versioned, secret-free provider preset catalog covering official Anthropic/OpenAI configuration where compatible, OpenRouter, DeepSeek, Zhipu GLM, Kimi/Moonshot, SiliconFlow, Alibaba Bailian, and Volcengine Ark, and SHALL identify the supported stable Agent ids for every preset.

#### Scenario: Browse presets for an Agent
- **WHEN** a user opens profile creation for Claude Code, OpenCode, or Codex CLI
- **THEN** the system SHALL show only presets declaring compatibility with that stable Agent id
- **AND** SHALL offer a custom-provider option

#### Scenario: Create a profile from a preset
- **WHEN** a user selects a compatible preset
- **THEN** the system SHALL copy its Agent-specific endpoint, protocol, authentication strategy, and recommended model defaults into a new editable profile draft
- **AND** SHALL NOT apply the draft globally until the user saves and explicitly applies it

#### Scenario: Preset is incompatible with the selected Agent
- **WHEN** a caller requests profile creation from a preset that does not declare compatibility with the target Agent
- **THEN** the system SHALL reject the request without guessing or translating protocol-specific values
- **AND** SHALL persist neither a profile nor a credential

#### Scenario: Preset catalog contains no secrets
- **WHEN** the bundled preset catalog is read by the desktop or Web runtime
- **THEN** its entries SHALL NOT contain API keys, credential references, authorization headers with secret values, executable scripts, or remote markup

#### Scenario: Bundled catalog is upgraded
- **WHEN** a VaneHub upgrade changes, deprecates, or replaces a bundled preset
- **THEN** existing profiles created from an earlier preset version SHALL retain their user-owned values
- **AND** adopting the newer preset values SHALL require explicit user action

### Requirement: Profile lifecycle management
The system SHALL let users create, import-current, read, update, duplicate, and delete normalized CLI configuration profiles without exposing stored credentials through frontend responses.

#### Scenario: Create a valid profile
- **WHEN** a user submits a valid Agent-specific profile and any required credential
- **THEN** the system SHALL persist the non-secret profile using a stable profile id
- **AND** SHALL store the credential through the platform credential store
- **AND** SHALL return only whether a credential is configured

#### Scenario: Import current global configuration
- **WHEN** a user imports a supported Agent's current global configuration
- **THEN** the native runtime SHALL extract the adapter-owned fields into a normalized profile
- **AND** SHALL move detected managed credentials into the platform credential store without returning them to the frontend
- **AND** SHALL leave the live configuration unchanged

#### Scenario: Reject an invalid profile
- **WHEN** a profile contains an invalid endpoint, duplicate or path-like provider id, control characters, an empty required model, unsupported syntax, or an out-of-scope advanced section
- **THEN** the system SHALL return field-level validation errors
- **AND** SHALL persist neither profile metadata nor credentials

#### Scenario: Delete an applied profile
- **WHEN** a user attempts to delete the profile recorded as globally applied for that Agent
- **THEN** the system SHALL require the user to apply or import another profile, or explicitly detach the applied-state record, before deletion
- **AND** SHALL NOT silently modify the live configuration

### Requirement: CC Switch-inspired startup synchronization
The desktop runtime SHALL synchronize standard user-level CLI configurations into profile storage during startup using exclusive semantics for Claude Code and Codex and additive semantics for OpenCode, without continuously watching configuration files.

#### Scenario: Bootstrap an empty exclusive Agent
- **WHEN** desktop startup finds no saved profile for Claude Code or Codex and its standard live configuration is parseable and contains supported managed fields
- **THEN** the system SHALL import one stable `default` profile, store any detected credential through the platform credential store, and record that profile as applied
- **AND** SHALL NOT modify the live configuration

#### Scenario: Skip an initialized exclusive Agent
- **WHEN** desktop startup finds any saved profile for Claude Code or Codex
- **THEN** the system SHALL skip automatic live import for that Agent
- **AND** SHALL NOT recreate or overwrite a `default` profile

#### Scenario: Synchronize additive OpenCode providers
- **WHEN** desktop startup reads a parseable `opencode.json` containing compatible entries under `provider`
- **THEN** the system SHALL create profiles for new provider ids and update existing live-managed profiles whose normalized live values changed
- **AND** SHALL store detected credentials through the platform credential store without exposing them through DTOs or logs
- **AND** SHALL NOT delete saved profiles merely because their provider ids are absent from the current live file

#### Scenario: Startup live configuration is absent or malformed
- **WHEN** a supported standard live configuration is absent, empty, or malformed
- **THEN** the affected synchronization pass SHALL remain best-effort, make no partial profile or credential changes, and emit only redacted diagnostics
- **AND** SHALL NOT prevent the desktop application from starting or block synchronization of another Agent

#### Scenario: Web runtime starts
- **WHEN** the Web/mock runtime initializes
- **THEN** it SHALL NOT fabricate local profiles, paths, parse results, or credential-presence claims

### Requirement: Secret isolation and materialization
The system SHALL keep profile credentials out of SQLite, frontend DTOs, Web storage, operation results, and persisted logs, while allowing the desktop runtime to materialize a credential into a CLI-owned live file only when that CLI requires it.

#### Scenario: Read a credential-backed profile
- **WHEN** the frontend reads a profile that has a stored credential
- **THEN** the response SHALL report `credentialConfigured = true`
- **AND** SHALL NOT contain the credential, an authorization header, or a reversible credential reference

#### Scenario: Apply with missing stored credential
- **WHEN** a profile requires a credential but its credential-store entry is missing
- **THEN** the system SHALL reject application before changing any live file
- **AND** SHALL mark the profile as requiring credential repair

#### Scenario: Persist operation logs
- **WHEN** a profile is imported, changed, deleted, or applied
- **THEN** unified logs SHALL contain only redacted metadata such as Agent id, profile id, operation id, status, and safe path context
- **AND** SHALL omit credentials and configuration bodies

### Requirement: Claude Code global projection
Applying a Claude Code profile SHALL merge the profile-owned provider and model environment keys into the resolved user-level Claude Code `settings.json`, remove keys owned by the previously applied VaneHub profile, and preserve all unrelated user settings.

#### Scenario: Switch Claude Code profile
- **WHEN** a user applies a valid Claude Code profile
- **THEN** the system SHALL atomically write the selected profile's managed environment and model keys to the resolved global `settings.json`
- **AND** SHALL preserve unrelated hooks, permissions, plugins, top-level fields, and environment keys
- **AND** SHALL record that profile as globally applied only after the write succeeds

#### Scenario: Claude Code configuration is malformed
- **WHEN** the existing Claude Code live file cannot be parsed safely
- **THEN** the system SHALL reject the switch with the resolved path and a user-actionable parse error
- **AND** SHALL leave the file and applied-state record unchanged

### Requirement: Codex CLI global projection
Applying a Codex CLI profile SHALL update only the VaneHub-owned top-level settings and selected model-provider table in the resolved `config.toml`, SHALL preserve unrelated TOML sections, and SHALL protect official authentication state unless the user explicitly confirms a profile that owns `auth.json`.

#### Scenario: Apply third-party Codex profile while preserving official auth
- **WHEN** a third-party Codex profile uses a credential strategy that does not require replacing `auth.json`
- **THEN** the system SHALL apply the provider, endpoint, model, wire API, and allowed reasoning fields to `config.toml`
- **AND** SHALL leave existing ChatGPT/Codex official authentication material unchanged

#### Scenario: Apply Codex profile that replaces auth
- **WHEN** a Codex profile requires replacing `auth.json`
- **THEN** the system SHALL require explicit confirmation identifying both affected files
- **AND** SHALL validate both new documents before writing either file

#### Scenario: Second Codex file write fails
- **WHEN** one Codex live file has been replaced and a later required file replacement fails
- **THEN** the system SHALL restore the exact prior bytes or prior absence of every file already changed
- **AND** SHALL leave the applied profile unchanged

### Requirement: OpenCode global projection
Applying an OpenCode profile SHALL upsert its provider definition in the resolved user-level `opencode.json`, set the profile's declared provider/model as the global default model, and preserve unrelated providers and other settings.

#### Scenario: Apply OpenCode profile
- **WHEN** a user applies a valid OpenCode profile with at least one model and a declared default model
- **THEN** the system SHALL atomically upsert `provider.<profile-provider-id>` with its npm package, options, headers, credential, and model definitions
- **AND** SHALL update the global default model to the selected provider/model
- **AND** SHALL preserve unrelated provider entries, plugins, and other top-level settings

#### Scenario: Existing OpenCode file uses supported JSON5 syntax
- **WHEN** the current `opencode.json` contains supported JSON5 syntax
- **THEN** the native runtime SHALL parse and preserve its semantic unmanaged values while applying the selected profile

### Requirement: Exclusive switch-away backfill and race protection
The system SHALL fingerprint each Agent's managed live fragment after application, SHALL report external changes as drift for visibility, and SHALL automatically preserve an exclusive Agent's current managed live values in the leaving profile before applying a different profile.

#### Scenario: Live managed values still match
- **WHEN** profile status is inspected and the managed live fragment matches the last projection fingerprint
- **THEN** the system SHALL report the profile as applied and not drifted

#### Scenario: User edited managed values outside VaneHub
- **WHEN** the managed live fragment differs from the last projection fingerprint and the user applies a different Claude Code or Codex profile
- **THEN** the system SHALL extract and validate the current managed values, update the leaving profile and its credential reference, and only then write the target profile
- **AND** SHALL abort before changing the target live configuration if the backfill cannot be persisted or compensated safely

#### Scenario: Reapply the current exclusive profile
- **WHEN** the user applies the profile already recorded as current for Claude Code or Codex
- **THEN** the system SHALL NOT backfill the profile from live
- **AND** SHALL treat the confirmed action as an explicit projection of the saved profile values

#### Scenario: Live file changes during apply
- **WHEN** the target live-file fingerprint changes after the apply plan is built but before replacement
- **THEN** the system SHALL abort with a drift conflict
- **AND** SHALL NOT overwrite the external edit

#### Scenario: OpenCode is externally edited while VaneHub is running
- **WHEN** an OpenCode provider changes outside VaneHub after startup
- **THEN** the system SHALL keep the current database view until the next desktop startup synchronization or an explicit manual import
- **AND** SHALL NOT claim real-time file watching

### Requirement: Atomic and observable profile application
The desktop runtime SHALL serialize profile application per Agent, prebuild and validate every output document, atomically replace live files, compensate partial failures, and expose the work as a redacted observable operation.

#### Scenario: Successful application
- **WHEN** every required live file is written and applied state is persisted
- **THEN** the operation SHALL complete with the Agent id, profile id, safe affected paths, warnings, drift resolution, and restart guidance
- **AND** SHALL NOT include configuration bodies or credentials

#### Scenario: Concurrent applications for one Agent
- **WHEN** two apply requests target the same Agent concurrently
- **THEN** the native runtime SHALL serialize them so their file writes cannot interleave

#### Scenario: Application fails
- **WHEN** validation, credential lookup, file replacement, compensation, or applied-state persistence fails
- **THEN** the operation SHALL fail with a user-actionable redacted error
- **AND** the system SHALL report whether the prior live configuration was fully restored

### Requirement: Dedicated Agent configuration management page
The settings experience SHALL provide one dedicated, lazy-loaded Agent Configuration page for OnePiece and CLI provider configuration that remains visually and behaviorally separate from runtime Agent selection and registered-Agent management.

#### Scenario: Navigate from Agent management
- **WHEN** the user chooses to manage global configuration from the Agents page or a supported Agent card
- **THEN** settings SHALL open the dedicated Agent Configuration page
- **AND** MAY preselect the originating stable Agent id without changing the selected Session, runtime Agent, or workflow

#### Scenario: Open the dedicated Agent Configuration page
- **WHEN** the user selects Agent Configuration in settings or follows a supported Agent configuration link
- **THEN** settings SHALL open the Agent Configuration page
- **AND** MAY preselect the originating configuration Agent id without changing the selected Session, runtime Agent, or workflow
- **AND** SHALL NOT expose a separate Agent Management page or registered-Agent management controls

#### Scenario: Switch configuration Agent
- **WHEN** the user selects the OnePiece, Claude Code, OpenCode, or Codex tab
- **THEN** the page SHALL show that Agent's provider configuration controls through the frontend service boundary
- **AND** a CLI Agent tab SHALL retain its compact status strip, focused add/optional-import/refresh/search toolbar, and saved profile list
- **AND** switching configuration tabs SHALL NOT invoke runtime Agent selection

#### Scenario: Review startup synchronization outcome
- **WHEN** startup synchronization imports, updates, skips, or cannot parse local configuration
- **THEN** the page SHALL expose a compact secret-free outcome or warning without requiring candidate selection
- **AND** SHALL keep saved-profile management usable

#### Scenario: Review saved provider profiles
- **WHEN** the selected Agent has saved profiles
- **THEN** each profile card SHALL show its provider identity, profile name, endpoint, primary or default model, credential presence, validation state, and available lifecycle actions
- **AND** the globally applied profile SHALL have persistent visual emphasis and an explicit applied label that does not depend on hover

#### Scenario: Search saved provider profiles
- **WHEN** the user searches the selected Agent's saved profiles
- **THEN** the page SHALL filter the primary profile list without exposing or searching credential values
- **AND** SHALL show a distinct filtered-empty state when no profile matches

#### Scenario: Discover a common provider while creating
- **WHEN** the user opens the add-profile flow and searches or filters the common-provider catalog
- **THEN** the create dialog SHALL show only provider presets compatible with the selected Agent and matching the query or category
- **AND** SHALL retain a custom-provider entry

#### Scenario: Create a profile in a dialog
- **WHEN** the user selects a preset or custom provider in the add-profile flow
- **THEN** the create dialog SHALL populate an editable Agent-specific form below the preset selector
- **AND** SHALL keep cancel and save actions visible while the form scrolls
- **AND** SHALL neither save nor apply merely because a preset was selected

#### Scenario: Edit a profile in a dialog
- **WHEN** the user selects a profile edit action
- **THEN** the page SHALL open an accessible form-oriented edit dialog without requiring the source preset to be selected again
- **AND** SHALL never repopulate an existing credential value or apply the profile merely by saving it
- **AND** SHALL restore focus after the dialog closes and prevent duplicate submissions while saving

#### Scenario: Confirm a consequential profile action
- **WHEN** the user applies or deletes a profile, or performs a manual import requiring confirmation
- **THEN** the page SHALL use an application-owned confirmation dialog that identifies the profile and relevant effects
- **AND** SHALL not use a browser prompt or browser confirmation dialog

#### Scenario: Apply profile from the configuration page
- **WHEN** the user confirms a global profile application
- **THEN** the page SHALL show observable progress and the final restart or rollback guidance
- **AND** SHALL refresh profile status without changing the selected Session or runtime workflow

#### Scenario: Apply profile in Web mode
- **WHEN** a user applies a profile in the Web/mock runtime
- **THEN** the page SHALL show a deterministic simulated result without fabricating local files, credentials, or native runtime state

#### Scenario: Use the configuration page on a narrow viewport
- **WHEN** the page is rendered at a narrow supported viewport
- **THEN** the Agent switcher, configuration controls, status strip, toolbar, profile metadata, and card actions SHALL remain usable without horizontal page overflow
- **AND** create/edit dialogs SHALL keep their preset selector, form fields, and sticky primary actions keyboard-operable within the viewport

### Requirement: Web runtime profile parity
The Web/mock runtime SHALL implement the same profile lifecycle, validation, switch-away backfill semantics, and application contracts deterministically without reading or writing local CLI configuration files or claiming native global changes occurred.

#### Scenario: Apply profile in Web mode
- **WHEN** a user applies a supported profile in Web/mock mode
- **THEN** the adapter SHALL update simulated applied state and return a result marked as simulated
- **AND** SHALL NOT claim that a local filesystem path was changed

