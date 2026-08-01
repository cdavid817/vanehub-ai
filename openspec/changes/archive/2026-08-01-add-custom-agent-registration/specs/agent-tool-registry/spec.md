## MODIFIED Requirements

### Requirement: Supported CLI tool management catalog
The system SHALL maintain backend-owned management metadata for the supported AI coding CLI tools (`launch_kind = cli`) using stable agent identifiers. This requirement governs only CLI-managed agents; other `launch_kind` values (for example `api`) are registered agents per "Registered agent catalog" but are not CLI tools and are out of scope for this requirement.

#### Scenario: List managed CLI tools
- **WHEN** CLI management status is requested
- **THEN** the system SHALL return Claude Code, Codex CLI, Gemini CLI, and OpenCode in the fixed management order using their stable agent ids

#### Scenario: Preserve CLI package metadata
- **WHEN** the system manages a supported CLI tool
- **THEN** it SHALL associate the stable agent id with its executable name and npm package name from backend-owned metadata

#### Scenario: Non-CLI agents are excluded from CLI management
- **WHEN** CLI management status is requested
- **THEN** the system SHALL NOT include agents whose `launch_kind` is not `cli` in the CLI management list
