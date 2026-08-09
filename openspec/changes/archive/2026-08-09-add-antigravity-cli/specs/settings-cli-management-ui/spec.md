## ADDED Requirements

### Requirement: Script-only CLI installation eligibility
The system SHALL support managed CLIs that are distributed exclusively by installer script, with no package-manager package. For such a CLI the backend SHALL derive installer-script lifecycle eligibility rather than npm eligibility, SHALL NOT emit an npm command as a fallback or as guidance, and SHALL select the installer appropriate to the host platform.

#### Scenario: Install a script-only CLI on a POSIX host
- **WHEN** a script-only CLI is not installed on macOS or Linux and the backend marks installer-script installation eligible
- **THEN** the page SHALL present an install action that runs that CLI's declared shell installer
- **AND** neither the action nor its guidance SHALL reference an npm package

#### Scenario: Install a script-only CLI on Windows
- **WHEN** a script-only CLI is not installed on Windows and declares a PowerShell installer
- **THEN** the backend SHALL mark it installer-script eligible using that PowerShell installer rather than reporting it as manual-only

#### Scenario: Script-only CLI without a platform installer falls back to guidance
- **WHEN** a script-only CLI is not installed on a platform for which it declares no installer
- **THEN** the page SHALL present localized manual guidance
- **AND** it SHALL NOT present an install action

#### Scenario: Version probe failure remains diagnosable
- **WHEN** a script-only CLI is installed but its version command fails
- **THEN** the page SHALL show the failed version-check state with its per-CLI error
- **AND** it SHALL NOT present reinstalling the same version as a guaranteed repair

### Requirement: Antigravity CLI appears in CLI management
The CLI management page SHALL present `antigravity-cli` as a managed CLI alongside the other managed CLIs, reporting its installation state, active path, environment and source, runnable state, and version information through the same service-backed contract.

#### Scenario: Antigravity card renders with detection results
- **WHEN** the CLI management page loads on a host where `agy` is installed
- **THEN** the `antigravity-cli` card SHALL show its detected path, environment, source, and current version

#### Scenario: Antigravity counts toward page summaries
- **WHEN** the CLI management page computes installed and attention summary counts
- **THEN** `antigravity-cli` SHALL be included in those counts
