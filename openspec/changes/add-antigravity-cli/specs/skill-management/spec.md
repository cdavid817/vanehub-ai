## MODIFIED Requirements

### Requirement: Agent mount path management
The system SHALL use registered CLI-capable Agent ids as Skill mount carriers, SHALL store one editable mount path per CLI-capable Agent, and SHALL reject mount paths that overlap the VaneHub-managed `.vanehub` namespace or any Skill source directory.

#### Scenario: Default Agent mount paths
- **WHEN** the system returns mount paths for registered Agents
- **THEN** it SHALL include defaults for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli` when those CLI Agents are registered
- **AND** SHALL exclude API-only Agents

#### Scenario: Update Agent mount path
- **WHEN** a user changes a CLI Agent mount path to a valid disjoint relative path
- **THEN** the system SHALL persist the new path for that Agent and immediately migrate existing managed Skill links for that Agent
