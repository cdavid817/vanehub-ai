# settings-cli-management-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Service-backed CLI parameter settings page
The settings center SHALL render CLI Parameter Management as a service-backed page separate from CLI installation and version management.

#### Scenario: Open CLI parameter page
- **WHEN** a user opens CLI Parameter Management
- **THEN** the page SHALL load typed profiles through the frontend agent service
- **AND** it SHALL preserve the settings shell, independent content scrolling, search behavior, and mounted draft state

#### Scenario: Keep installation management separate
- **WHEN** the CLI parameter page renders
- **THEN** it SHALL NOT install, upgrade, downgrade, detect, or remove a CLI package
- **AND** CLI package operations SHALL remain on the existing CLI Management page

### Requirement: CLI management page
The settings center SHALL replace the provider management page with a `CLI 管理` page for supported local AI coding CLI tools.

#### Scenario: Open CLI management page
- **WHEN** a user opens the CLI management settings page
- **THEN** the page SHALL display Anthropic Claude Code CLI, OpenAI Codex CLI, Google Gemini CLI, and OpenCode CLI in that fixed order
- **AND** the page SHALL use service-backed CLI status data rather than frontend-local provider demo data

#### Scenario: Render CLI summary
- **WHEN** the CLI management page renders
- **THEN** it SHALL show only CLI installed and CLI not installed summary counts
- **AND** it SHALL NOT show active provider count, add provider actions, or provider configuration empty states

#### Scenario: Remove provider configuration controls
- **WHEN** the CLI management page renders any CLI card
- **THEN** it SHALL NOT show API Key, URL, preset, enable, edit, or delete controls

### Requirement: Cached CLI status initial rendering
The CLI management page SHALL synchronously read the last persisted CLI detection result for initial rendering without starting expensive detection work.

#### Scenario: Initial page load reads cached result
- **WHEN** a user opens the CLI management page
- **THEN** the page SHALL request the last known CLI status through the frontend service boundary
- **AND** the request SHALL NOT trigger local executable checks, CLI version commands, npm registry queries, install, upgrade, or downgrade commands

#### Scenario: No previous detection
- **WHEN** no persisted detection result exists for a supported CLI
- **THEN** the CLI card SHALL display an undetected state and allow the user to start refresh detection

#### Scenario: First startup auto refresh
- **WHEN** the application starts and no persisted detection result exists for any supported CLI
- **THEN** the system SHALL start one asynchronous CLI detection refresh after reading cached status
- **AND** the startup and settings shell rendering SHALL NOT block on local executable checks, CLI version commands, npm registry queries, install, upgrade, or downgrade commands

### Requirement: CLI detection refresh interaction
The CLI management page SHALL refresh CLI detection and remote version metadata through asynchronous backend-managed operations.

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
- **THEN** the page SHALL present an install action for that version

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
The CLI management page SHALL show the most recent operation state and expandable logs inside each affected CLI card.

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

### Requirement: Bulk CLI upgrade
The CLI management page SHALL provide a service-backed bulk upgrade action for supported managed CLIs whose current version is older than the latest known stable version and whose lifecycle eligibility allows a backend-managed mutation method.

#### Scenario: Start bulk upgrade
- **WHEN** at least one supported CLI has a backend-managed lifecycle method and has a latest known stable version newer than its current version
- **THEN** the CLI management page SHALL enable an Upgrade All action through the frontend agent service boundary
- **AND** the action SHALL start one asynchronous backend-managed operation rather than invoking package mutation directly from React components

#### Scenario: No eligible bulk upgrades
- **WHEN** no supported CLI is eligible for bulk upgrade
- **THEN** the CLI management page SHALL disable the Upgrade All action
- **AND** it SHALL keep per-CLI refresh, diagnostics, and manual guidance available

#### Scenario: Bulk upgrade skips ambiguous targets
- **WHEN** a supported CLI has multiple installations, an unsupported active source, unknown current version, unknown latest version, or no version delta
- **THEN** the bulk upgrade operation SHALL skip that CLI
- **AND** it SHALL record user-visible operation logs explaining why the CLI was not mutated

#### Scenario: Bulk upgrade feedback
- **WHEN** a bulk upgrade operation is queued or running
- **THEN** the CLI management page SHALL disable package mutation controls only for CLIs participating in a conflicting package operation
- **AND** it SHALL keep cached CLI status, card expansion, operation logs, page refresh controls, and settings navigation interactive where they do not conflict with package mutation

#### Scenario: Bulk upgrade isolates CLI workers
- **WHEN** a bulk upgrade includes more than one eligible CLI
- **THEN** each eligible CLI SHALL run in an independent backend worker
- **AND** a failure, timeout, or skip for one CLI SHALL NOT stop unrelated eligible CLIs from upgrading
- **AND** the final operation result SHALL report upgraded, skipped, and failed CLI ids independently

### Requirement: Local environment check toolbar
The CLI management page SHALL present local environment check actions in a compact toolbar that includes conflict diagnostics, refresh, and bulk upgrade while preserving existing per-CLI operational cards.

#### Scenario: Render local environment check toolbar
- **WHEN** the CLI management page renders
- **THEN** it SHALL show localized actions for diagnosing installation conflicts, refreshing CLI status, and upgrading all eligible CLIs
- **AND** the page SHALL continue to show current version, latest version, active path, health state, conflict state, and diagnostics for Claude Code, Codex CLI, Gemini CLI, and OpenCode

#### Scenario: Refresh detects CLIs independently
- **WHEN** CLI Management refreshes more than one CLI
- **THEN** each CLI detection SHALL run in an independent backend worker
- **AND** a slow or failed detection for one CLI SHALL NOT prevent other CLI statuses from being detected and persisted

#### Scenario: Keep About page navigational
- **WHEN** the About page shows the CLI environment summary
- **THEN** it SHALL provide installed and attention counts with navigation to CLI Management
- **AND** it SHALL NOT duplicate refresh, diagnose, install, upgrade, or downgrade lifecycle controls

### Requirement: Cause-specific CLI lifecycle guidance
The CLI management page SHALL explain unavailable package mutation with guidance derived from the detected installation state instead of a single generic manual-action warning.

#### Scenario: Active CLI cannot run
- **WHEN** a supported CLI has an active installation that fails its version command
- **THEN** the CLI card SHALL show localized environment-check guidance
- **AND** it SHALL NOT present an install or upgrade action as if reinstalling the same version would necessarily repair the active CLI

#### Scenario: Multiple installations detected
- **WHEN** a supported CLI has multiple detected installations or a conflict state
- **THEN** the CLI card SHALL direct the user to installation diagnostics before package mutation
- **AND** diagnostics SHALL identify the active path and each discovered installation

#### Scenario: Active source is not backend-managed
- **WHEN** a supported CLI's active installation source is not supported by a backend-managed npm, wget-script, or WinGet mutation plan
- **THEN** the CLI card SHALL show localized source-native update guidance using the detected source label
- **AND** it SHALL avoid suggesting that a generic npm install will update the active non-npm CLI

#### Scenario: Non-npm source has newer known version
- **WHEN** a supported CLI's active installation source is not npm-managed and the latest known stable version is newer than the current version
- **THEN** the CLI card SHALL still surface the localized upgrade state
- **AND** the upgrade control SHALL only be enabled when the backend can preserve the detected source-specific install method
- **AND** source-native guidance SHALL explain why the user must update through the detected installation source

### Requirement: Source-aware CLI lifecycle planning
CLI install and upgrade operations SHALL choose a backend-managed method from the detected installation source instead of always using npm.

#### Scenario: First install prefers wget script
- **WHEN** a supported CLI is not installed and has an official script installer
- **THEN** the backend lifecycle plan SHALL use the wget-based script installer before npm
- **AND** the user-visible install command SHALL reflect the same preferred method

#### Scenario: First install falls back to npm when no script exists
- **WHEN** a supported CLI is not installed and has no official script installer
- **THEN** the backend lifecycle plan SHALL use the npm package installer

#### Scenario: Upgrade preserves existing managed source
- **WHEN** a supported CLI is installed from npm
- **THEN** upgrade SHALL use npm for that CLI
- **WHEN** a supported CLI is installed from a recognized script/vendor path and has an official script installer
- **THEN** upgrade SHALL use the wget-based script installer for that CLI
- **WHEN** a supported CLI is installed from WinGet and has a verified WinGet package id
- **THEN** upgrade SHALL use `winget upgrade --id <package-id> --exact` for that CLI

#### Scenario: Cached WinGet status is reclassified
- **WHEN** a cached CLI status was persisted before WinGet became a backend-managed lifecycle
- **AND** the cached active installation path is still a direct WinGet executable
- **THEN** the cached status SHALL be reclassified as WinGet-managed when it is read
- **AND** CLI Management SHALL NOT continue showing source-native manual guidance for that cached WinGet installation

#### Scenario: Managed install can update without latest metadata
- **WHEN** a supported CLI is installed from a backend-managed source and the latest version lookup is unavailable
- **THEN** CLI Management SHALL still show a one-click upgrade action
- **AND** the backend operation SHALL use the source-preserving `latest` target for that CLI

#### Scenario: Managed install keeps update action when current
- **WHEN** a supported CLI is installed from a backend-managed source and the current version equals the latest known version
- **THEN** CLI Management SHALL still show a one-click upgrade action
- **AND** the backend operation SHALL preserve the detected installation method

#### Scenario: Unsupported source remains manual
- **WHEN** a supported CLI is installed from a source without a verified backend mutation plan
- **THEN** CLI Management SHALL show the upgrade state when versions indicate an update
- **AND** it SHALL prevent starting a backend mutation and show source-native guidance

#### Scenario: Windows script shims are not direct launch targets
- **WHEN** Windows PATH contains PowerShell or shell script shims such as `.ps1` or `.sh` for a managed CLI
- **THEN** backend detection and launch resolution SHALL NOT treat those files as direct executable targets
- **AND** it SHALL prefer direct `.exe`, `.cmd`, `.bat`, or `.com` candidates for the same CLI when available

#### Scenario: Refresh warnings are not persisted as CLI status errors
- **WHEN** CLI refresh encounters transient detection warnings such as version command timeout or remote version lookup timeout
- **THEN** the refresh operation SHALL show the warning in the current operation log
- **AND** the persisted CLI status SHALL NOT store that transient warning in `lastError`

#### Scenario: Slow version probes avoid false broken status
- **WHEN** a locally installed CLI takes several seconds to answer its version command
- **THEN** backend detection SHALL allow enough time for normal Windows CLI shim startup
- **AND** it SHALL NOT mark the CLI as installed-but-unrunnable only because the previous short probe timeout was exceeded

#### Scenario: One-click upgrade replaces copied install commands
- **WHEN** CLI Management renders a backend-managed installed CLI
- **THEN** the CLI card SHALL show an upgrade action instead of a copy-install-command action
- **AND** activating the upgrade action SHALL request a backend package operation targeting the latest known version
- **AND** if latest metadata is unavailable, activating the upgrade action SHALL target `latest`

#### Scenario: Unsupported installed CLI keeps visible upgrade affordance
- **WHEN** CLI Management renders an installed CLI whose lifecycle source cannot be safely mutated by the backend
- **THEN** the CLI card SHALL still show an upgrade action
- **AND** the upgrade action SHALL be disabled with localized source or environment guidance

### Requirement: CLI management uses branded CLI identity

The CLI management settings page SHALL show the branded icon for each managed CLI.

#### Scenario: CLI cards show tool icons

- **WHEN** the CLI management page lists Claude Code, Codex CLI, Gemini CLI, or OpenCode
- **THEN** each tool card SHALL render that tool's branded icon from the stable agent id.

