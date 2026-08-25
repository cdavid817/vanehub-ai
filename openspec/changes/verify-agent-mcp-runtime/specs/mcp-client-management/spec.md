## ADDED Requirements

### Requirement: Managed MCP availability is independent of observation
The desktop runtime SHALL expose every active MCP server visible to the current session workspace to supported Agent invocations independently of whether MCP relay observation is enabled, while protocol telemetry MUST remain disabled unless the user explicitly enables it.

#### Scenario: Claude Code uses a managed MCP server
- **WHEN** a `claude-code` CLI generation runs with an active workspace-visible VaneHub-managed MCP server
- **THEN** the generation SHALL receive an invocation-scoped MCP configuration that can initialize and invoke that server
- **AND** the runtime SHALL NOT modify Claude Code's global configuration

#### Scenario: Codex uses a managed MCP server
- **WHEN** a `codex-cli` CLI generation runs with an active workspace-visible VaneHub-managed MCP server
- **THEN** the generation SHALL receive invocation-scoped MCP configuration overrides that can initialize and invoke that server
- **AND** the runtime SHALL NOT modify Codex's global configuration

#### Scenario: OpenCode uses a managed MCP server
- **WHEN** an `opencode` CLI generation runs with an active workspace-visible VaneHub-managed MCP server
- **THEN** the generation SHALL receive an invocation-scoped MCP configuration that can initialize and invoke that server
- **AND** unrelated existing invocation configuration and OpenCode's global configuration SHALL remain effective

#### Scenario: OnePiece uses a managed MCP server
- **WHEN** a `onepiece` native API generation runs with an active workspace-visible VaneHub-managed MCP server whose bounded tool catalog was cached by a successful connection test
- **THEN** the generation SHALL expose and invoke the server through the native MCP tool gateway subject to the existing approval policy

#### Scenario: MCP observation is disabled
- **WHEN** any supported Agent uses a VaneHub-managed MCP server while MCP relay observation is disabled
- **THEN** the MCP server SHALL remain available to that Agent
- **AND** the runtime MUST NOT persist proxied MCP request lifecycle telemetry

#### Scenario: MCP server is not visible
- **WHEN** an MCP server is inactive or project-scoped to a workspace other than the Agent session workspace
- **THEN** the runtime SHALL NOT expose that server to the Agent invocation
