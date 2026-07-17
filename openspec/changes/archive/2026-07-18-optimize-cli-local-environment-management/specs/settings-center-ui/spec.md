## MODIFIED Requirements

### Requirement: CLI detection refresh interaction
The CLI management page SHALL refresh CLI detection and remote version metadata through asynchronous backend-managed all-tool or single-tool operations.

#### Scenario: Start all-tool refresh detection
- **WHEN** the user activates the page refresh action
- **THEN** the page SHALL start one asynchronous refresh operation for all supported CLIs through the frontend service boundary
- **AND** the settings shell SHALL remain interactive while the operation runs

#### Scenario: Start single-tool refresh detection
- **WHEN** the user activates refresh on one CLI card
- **THEN** the page SHALL request a targeted asynchronous refresh for that stable agent id without marking unrelated cards as refreshing

#### Scenario: Display refreshed CLI metadata
- **WHEN** refresh detection completes for a supported CLI
- **THEN** the corresponding card SHALL display installation state, active path, environment and source, runnable state, current version, latest version, discovered installation count, conflict state, last checked time, or a user-displayable per-CLI error

#### Scenario: One CLI refresh fails
- **WHEN** refresh detection fails for one supported CLI but succeeds for another
- **THEN** the page SHALL preserve and display the successful CLI result and show the failed CLI's error without failing the whole page

### Requirement: CLI version actions
The CLI management page SHALL present install, upgrade, downgrade, current, confirmation, or manual-guidance actions using backend-derived lifecycle eligibility.

#### Scenario: Stable version selection
- **WHEN** available versions are displayed for an eligible CLI
- **THEN** the page SHALL show at most the latest 20 stable versions by default
- **AND** it SHALL exclude prerelease versions

#### Scenario: Install missing CLI
- **WHEN** a CLI is not installed and the backend marks npm installation eligible
- **THEN** the page SHALL present an install action for the selected stable version

#### Scenario: Upgrade or downgrade npm-managed CLI
- **WHEN** an active npm-managed CLI has a selected stable version different from its current version
- **THEN** the page SHALL present an upgrade or downgrade action matching version order

#### Scenario: Multiple installations require confirmation
- **WHEN** the backend reports multiple distinct installations before an otherwise eligible package mutation
- **THEN** the page SHALL show the active target and installation distribution and require explicit user confirmation

#### Scenario: Active source cannot be updated safely
- **WHEN** the backend reports a non-npm, unknown, or broken active installation
- **THEN** the page SHALL show localized manual or source-native guidance and SHALL NOT present npm mutation as updating that active installation

#### Scenario: Current CLI version selected
- **WHEN** an eligible CLI has the selected target version equal to its current version
- **THEN** the page SHALL present the current-version state and prevent a redundant package operation

### Requirement: CLI operation feedback
The CLI management page SHALL show recent operation state and expandable logs while preventing concurrent global package mutations.

#### Scenario: Operation state in CLI card
- **WHEN** a refresh, install, upgrade, or downgrade operation is associated with a CLI
- **THEN** that CLI card SHALL show the latest operation status without requiring a global log panel

#### Scenario: Expand operation logs
- **WHEN** the user expands operation details for a CLI card
- **THEN** the page SHALL display the logs associated with that CLI's most recent operation

#### Scenario: Detection operation is card-local
- **WHEN** a targeted CLI detection operation is running
- **THEN** the page SHALL disable affected refresh controls and SHALL keep unrelated CLI cards and settings navigation interactive

#### Scenario: Package mutation is globally serialized
- **WHEN** any CLI install, upgrade, or downgrade operation is queued or running
- **THEN** the page SHALL disable package mutation controls across CLI cards while keeping navigation, cached information, card expansion, and logs interactive

## ADDED Requirements

### Requirement: Compact CLI local environment cards
The CLI management page SHALL present supported tools as compact operational cards using shared settings primitives and semantic tokens.

#### Scenario: Render diagnostic card
- **WHEN** a CLI card renders cached or refreshed data
- **THEN** it SHALL provide scannable identity, environment/source badge, current and latest version, active path, health state, refresh action, eligible lifecycle action, and expandable diagnostics without oversized marketing layout

#### Scenario: Render both registered visual styles
- **WHEN** either `futuristic` or `minimal` is active
- **THEN** CLI cards, conflict warnings, dialogs, controls, focus states, and logs SHALL remain readable using equivalent semantic token roles without theme-name branches in the component

### Requirement: About CLI environment summary
The About page SHALL show a compact service-backed summary of the supported CLI environment without duplicating lifecycle controls.

#### Scenario: Render desktop environment summary
- **WHEN** cached desktop CLI status is available
- **THEN** About SHALL show installed and attention counts and provide navigation to CLI Management

#### Scenario: Render Web environment summary
- **WHEN** About runs through the Web/mock adapter
- **THEN** it SHALL show a localized native-detection-unavailable summary and SHALL NOT imply that host CLIs were inspected

### Requirement: Localized CLI environment management text
All new CLI environment and About summary user-visible text SHALL use synchronized Simplified Chinese and English resources.

#### Scenario: Switch application language
- **WHEN** the active language changes between zh-CN and en
- **THEN** environment, source, health, conflict, confirmation, manual-guidance, refresh, summary, and error text SHALL render in the active locale

#### Scenario: Maintain translation parity
- **WHEN** a CLI environment translation key is added, changed, or removed
- **THEN** the existing locale parity and visible-text guardrail tests SHALL continue to pass

