## ADDED Requirements

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
