## ADDED Requirements

### Requirement: Per-seat managed MCP availability
Every routed seat in a multi-Agent CLI session SHALL receive the active VaneHub-managed MCP servers visible to the shared session workspace through that seat's provider-specific invocation mechanism.

#### Scenario: Heterogeneous seats use the same managed MCP server
- **WHEN** a multi-Agent session containing `claude-code`, `codex-cli`, and `opencode` seats routes a turn to each seat
- **THEN** each seat SHALL be able to initialize and invoke the same active workspace-visible VaneHub-managed MCP server during its own turn

#### Scenario: Agent-to-Agent handoff preserves MCP availability
- **WHEN** one seat completes a reply that routes the next turn to another seat
- **THEN** the routed seat's provider invocation SHALL receive the session workspace's currently visible MCP projection
- **AND** provider-thread resume state SHALL NOT remove or stale that projection
