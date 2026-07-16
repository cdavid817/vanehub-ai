## ADDED Requirements

### Requirement: Supported CLI tool management catalog
The system SHALL maintain backend-owned management metadata for the supported AI coding CLI tools using stable agent identifiers.

#### Scenario: List managed CLI tools
- **WHEN** CLI management status is requested
- **THEN** the system SHALL return Claude Code, Codex CLI, Gemini CLI, and OpenCode in the fixed management order using their stable agent ids

#### Scenario: Preserve CLI package metadata
- **WHEN** the system manages a supported CLI tool
- **THEN** it SHALL associate the stable agent id with its executable name and npm package name from backend-owned metadata

### Requirement: Persisted CLI detection status
The system SHALL store the last known CLI detection status for supported CLI tools.

#### Scenario: Read last known detection status
- **WHEN** the frontend requests CLI tool status for initial rendering
- **THEN** the system SHALL return persisted installed state, current version, latest version, available versions, detected path, last checked time, and last error when available

#### Scenario: Represent never detected CLI
- **WHEN** a supported CLI has no persisted detection result
- **THEN** the system SHALL return a status that distinguishes never detected from installed and not installed states

### Requirement: CLI version metadata
The system SHALL track current local version, latest remote npm version, and selectable stable npm versions for each supported CLI.

#### Scenario: Store stable available versions
- **WHEN** remote npm versions are refreshed for a CLI
- **THEN** the system SHALL store at most the latest 20 stable versions by default and exclude prerelease versions

#### Scenario: Store detected executable path
- **WHEN** a CLI executable is found locally
- **THEN** the system SHALL store the resolved local executable path for display

### Requirement: Web runtime CLI detection honesty
The Web runtime SHALL NOT fake local CLI installation status.

#### Scenario: Web runtime cannot inspect native CLI tools
- **WHEN** the CLI management page is rendered outside the Tauri desktop runtime without a native detection backend
- **THEN** the Web adapter SHALL return unsupported or undetected CLI status rather than reporting mock installed tools
