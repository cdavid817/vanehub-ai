## ADDED Requirements

### Requirement: Safe observability defaults
Desktop observability settings SHALL default to local metadata timelines enabled, OTLP export disabled, MCP relay disabled, metadata-only capture, and 30-day local trace retention.

#### Scenario: Existing installation upgrades
- **WHEN** an installation without saved observability settings starts after the migration
- **THEN** the runtime SHALL apply the safe defaults without enabling network export or content capture

### Requirement: Observability setting validation
The desktop runtime SHALL validate observability export, sampling, retention, and capture settings before persistence or use.

#### Scenario: Valid settings are saved
- **WHEN** a user saves a supported OTLP endpoint and protocol, sampling ratio from 0 through 1, retention from 1 through 90 days, and supported capture policy
- **THEN** the native settings service SHALL persist the non-secret settings and apply them to newly created execution runs

#### Scenario: Invalid endpoint is submitted
- **WHEN** a user submits a malformed or unsupported OTLP endpoint
- **THEN** the settings service SHALL reject it with a typed validation error
- **AND** it SHALL preserve the previously active configuration

### Requirement: Telemetry credential protection
Optional OTLP authentication material SHALL be stored through the native credential service and SHALL NOT be returned as plaintext through frontend settings contracts.

#### Scenario: Authentication material is saved
- **WHEN** a user configures supported OTLP authentication material
- **THEN** the native runtime SHALL store the secret in the credential store and persist only a safe reference or configured indicator
- **AND** logs and trace settings responses SHALL omit the plaintext value

### Requirement: Runtime-specific observability settings behavior
Observability settings SHALL remain behind the shared frontend settings service with Tauri and Web/mock adapter parity.

#### Scenario: Desktop changes export settings
- **WHEN** React saves observability settings in the desktop runtime
- **THEN** it SHALL call the settings service interface
- **AND** only the Tauri adapter SHALL invoke the native command that updates exporter state

#### Scenario: Web mock changes export settings
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL return deterministic contract-compatible settings behavior
- **AND** it SHALL identify native OTLP export, credential storage, and SQLite retention as simulated or unavailable

### Requirement: Setting changes preserve active run context
Changes to observability settings SHALL apply prospectively and SHALL NOT rewrite the identity, sampling decision, capture policy, or relay state of an already active execution run.

#### Scenario: Settings change during generation
- **WHEN** a user changes telemetry settings while an Agent generation is running
- **THEN** the active run SHALL continue under its captured settings snapshot
- **AND** the new settings SHALL apply to later runs
