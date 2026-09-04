## Why

MCP servers created in VaneHub are not consistently available to every supported Agent runtime: Claude Code and Codex receive invocation-scoped relay configuration, OnePiece uses the native MCP tool gateway, but OpenCode currently receives no VaneHub-managed MCP configuration. The native desktop suite also stops at MCP CRUD/connection checks and does not prove that a created server is usable from single-Agent and multi-Agent conversations.

## What Changes

- Require active, visible VaneHub-managed MCP servers to be exposed through the supported runtime-specific path for Claude Code, Codex CLI, OpenCode, and OnePiece without mutating provider-global configuration.
- Add invocation-scoped OpenCode MCP projection through its supported inline configuration environment while preserving unrelated caller configuration.
- Require every routed seat in a multi-Agent CLI conversation to receive the same workspace-visible MCP projection for its own provider invocation.
- Add an isolated WebdriverIO desktop verification layer that creates and tests a real MCP fixture, proves single-Agent availability for Claude Code, Codex, OpenCode, and OnePiece, and proves availability during a multi-Agent conversation.
- Preserve the existing MCP relay security, cleanup, redaction, and opt-in observability behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mcp-client-management`: Define runtime-specific exposure of VaneHub-managed MCP servers for Claude Code, Codex CLI, OpenCode, and OnePiece.
- `multi-agent-group-chat`: Require workspace-visible MCP configurations to remain effective for each routed Agent seat.
- `desktop-runtime-verification`: Add an isolated native WebdriverIO MCP runtime layer with actual protocol-use evidence.

## Impact

- Desktop runtime only; the Web/mock adapter contract is unchanged.
- Affects the Agent runtime MCP relay adapter, local process environment projection, OpenCode provider integration, and multi-seat generation dispatch.
- Adds deterministic CLI/provider/MCP fixtures and a dedicated WebdriverIO configuration and npm entry point.
- Does not add dependencies, write provider-global configuration, or weaken the frontend/native service boundary.
