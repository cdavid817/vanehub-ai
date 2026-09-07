## ADDED Requirements

### Requirement: Capability-driven CLI management presentation

CLI Management SHALL render backend-supplied metadata, installation source, executable health, authentication, compatibility, freshness, legacy status, effective capabilities and allowed actions. It SHALL NOT maintain a second provider execution allowlist or infer readiness from version detection alone.

#### Scenario: Installed CLI needs login

- **WHEN** a CLI is installed but authentication is required or unknown
- **THEN** the card SHALL retain installed state and expose an appropriate explicit authentication or check action

#### Scenario: A mode is unsupported

- **WHEN** a provider cannot run a requested mode
- **THEN** the UI SHALL show a localized reason rather than only an unexplained disabled control

### Requirement: Stable identity and interaction mode selection

Create-session controls SHALL use stable provider ids and capability-driven managed-conversation or native-terminal choices. Transport details SHALL remain separate from provider identity. Existing defaults and selected original providers MUST be preserved.

#### Scenario: User selects managed Qwen conversation

- **WHEN** Qwen is compatible with managed ACP execution
- **THEN** the request SHALL use `qwen-code` with its assessed execution profile and not create a `qwen-acp` agent identity

#### Scenario: Unknown capability needs checking

- **WHEN** a provider compatibility result is unknown
- **THEN** the UI SHALL offer an explicit check or explain the limitation and SHALL NOT start hidden sessions on mount

### Requirement: Visible pending interaction and accurate status

Conversation and task views SHALL distinguish running, waiting for permission, waiting for user input, cancelling, interrupted, failed and finished outcomes using compatible state projection. Pending interactions SHALL show provider/seat, operation, scope and applicable decision options with keyboard support and duplicate-submit protection.

#### Scenario: Multiple seats request approval

- **WHEN** two agents have pending permission requests
- **THEN** the UI SHALL identify their separate owners and route each decision to the correct backend request

#### Scenario: A decision is submitted twice

- **WHEN** a user double-clicks an approval button
- **THEN** the UI SHALL prevent duplicate submission and respect backend single-consumption errors

#### Scenario: A turn ends because of a limit

- **WHEN** the runtime reports a token or request limit stop reason
- **THEN** the UI SHALL not present it as verified task success

### Requirement: Truthful usage configuration and legacy disclosure

The UI SHALL distinguish reported usage, explicitly estimated usage, context occupancy and unavailable usage. It SHALL present only verified model/configuration options and SHALL keep the iFlow shutdown and local-compatibility limitations visible. Missing values MUST NOT be formatted as measured zero.

#### Scenario: No usage is reported

- **WHEN** the CLI provides no token or cost data
- **THEN** the UI SHALL display unavailable values rather than zero tokens or zero cost

#### Scenario: Only context occupancy is reported

- **WHEN** the peer reports current context used and size but no billable usage
- **THEN** the UI SHALL label context occupancy separately and not count it as input/output billing

#### Scenario: iFlow is shown

- **WHEN** a user opens the legacy iFlow details
- **THEN** the UI SHALL show the shutdown date and explicitly limited local terminal compatibility

### Requirement: Runtime-adapter parity and honest simulation

Changes to frontend services SHALL have Tauri and deterministic Web/mock implementations with compatible DTOs. Native work SHALL remain behind service adapters. A declared Web HTTP deployment lacking a compatible service SHALL fail explicitly rather than fall back to mock.

#### Scenario: Browser demonstration is used

- **WHEN** the feature is opened through Web/mock
- **THEN** the UI SHALL identify simulation and use synthetic paths, accounts and interactions without claiming real host discovery

#### Scenario: Web HTTP adapter is missing

- **WHEN** the host explicitly selects Web HTTP but the new service has no implementation
- **THEN** resolution SHALL return an explicit unsupported-runtime error and not simulated success

### Requirement: Localized accessible incremental UI

New UI SHALL use existing components, themes, localization and accessibility conventions, with complete Chinese and English resources, keyboard navigation, meaningful focus management and long-path/error handling. It MUST NOT introduce direct Tauri calls, a replacement state framework, or unrelated UI redesign.

#### Scenario: User operates a permission dialog by keyboard

- **WHEN** a user navigates and submits a decision without a mouse
- **THEN** focus SHALL remain usable and return to the originating context after closure

#### Scenario: Locale or theme changes

- **WHEN** the user switches supported language or theme
- **THEN** new CLI labels, unavailable reasons and terminal presentation SHALL follow the existing application conventions
