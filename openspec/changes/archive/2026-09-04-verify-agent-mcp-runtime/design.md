## Context

See `proposal.md` for motivation. The MCP repository already owns persistence, scope filtering, connection testing, and the relay transport. Native OnePiece generations consume cached MCP catalogs through `AgentMcpToolPort`. CLI generations only call `ManagedMcpRelayPort::prepare` when the observability setting is enabled, and that adapter currently returns provider arguments only for `claude-code` and `codex-cli`.

OpenCode 1.18 supports invocation-only inline configuration through `OPENCODE_CONFIG_CONTENT`; its MCP schema accepts local commands under `mcp`, but its `run` command has no MCP argv flag. The process runner already supports a bounded per-launch environment map, although the prepared MCP projection currently carries arguments and a cleanup guard only.

## Goals / Non-Goals

**Goals:**

- Make active, workspace-visible VaneHub MCP servers usable by all four requested Agent runtimes without global provider writes.
- Keep MCP protocol observation explicitly opt-in while making MCP availability unconditional.
- Preserve one private cleanup guard across provider configuration and per-server relay artifacts.
- Produce deterministic native desktop evidence for single-Agent and multi-Agent use without real model calls or credentials.

**Non-Goals:**

- Synchronizing VaneHub MCP definitions into provider-global configuration files.
- Adding MCP support to Gemini CLI, Antigravity, browser interaction modes, or unmanaged external terminals in this change.
- Changing MCP approval policy, connection limits, transport semantics, or the Web/mock service contract.

## Decisions

### Separate provider projection from protocol observation

The Agent process adapter will prepare the workspace-visible MCP projection for every CLI generation. The existing `mcp_relay_enabled` execution-context value will control whether relay observation metadata is attached, not whether the server is available. This retains one transport-normalizing path for stdio, SSE, and Streamable HTTP while preventing telemetry when observation is off.

Alternative: generate direct provider configurations when observation is off. Rejected because it duplicates three transport mappings per provider, exposes more secret-bearing provider syntax, and bypasses the relay's existing bounded cleanup behavior.

### Extend prepared MCP data with launch environment

`PreparedMcpRelay` will carry both invocation arguments and a bounded environment map. The process adapter will merge that map into the existing runner environment before launch. Claude Code and Codex continue using arguments; OpenCode receives one `OPENCODE_CONFIG_CONTENT` value.

The local Runner environment allowlist will admit that explicit OpenCode projection key. Remote Runners continue to reject it because the projected relay executable and its private configuration are owned by the local runtime.

Alternative: add OpenCode-specific environment mutation in the process adapter. Rejected because provider configuration belongs behind the MCP/provider adapter boundary and would create another hard-coded launch branch.

### Merge OpenCode inline configuration structurally

The adapter will parse an existing inherited `OPENCODE_CONFIG_CONTENT` JSON object when present, preserve unrelated keys and MCP entries, and overwrite only names managed by the current VaneHub invocation. Each projected OpenCode MCP entry will be a local command that launches the VaneHub relay helper with its private per-server configuration. Invalid inherited inline JSON will make projection fail safely and the generation will continue without the managed projection, following the existing warning path.

Alternative: replace the variable with a fresh MCP-only object. Rejected because it silently removes unrelated caller configuration. A temporary global/project file is also rejected because OpenCode project precedence could override it and cleanup would be harder to prove.

### Exercise provider behavior with deterministic process fixtures

The new WebdriverIO layer will prepend run-scoped fixture executables for Claude Code, Codex, and OpenCode. Each fixture will understand only the production invocation shape, discover the projected MCP entry, complete an initialize/list-tools/tool-call exchange, emit the provider's expected structured completion format, and record bounded evidence. A local deterministic OnePiece-compatible HTTP fixture will request the MCP tool and complete after the desktop approval command; it will not use a real API credential.

The multi-Agent case will seat the three CLI providers, route one turn to each stable Agent id, and require a separate successful MCP protocol marker per seat. This proves that seat routing and provider-thread lifecycle retain current workspace projection.

### Keep the desktop layer independent

The layer receives its own WebdriverIO config, npm entry point, isolated application/config directories, result summary, screenshots, logs, and fixture `PATH`. It can be run independently during development and will be added to the composed desktop run without altering other layers' environments.

## Risks / Trade-offs

- [OpenCode inline schema changes across releases] → Pin the tested fixture contract to the repository's supported OpenCode invocation and cover configuration shape with Rust unit tests plus the desktop fixture.
- [Inherited inline configuration contains secrets] → Never log the value; parse and merge in memory, pass only through the child environment, and keep runner diagnostics metadata-only.
- [Always starting the relay changes process overhead] → Start it only when an Agent actually connects to a projected active server; retain invocation-scoped cleanup and current resource limits.
- [OnePiece approval can make desktop tests race] → Wait for the explicit pending-approval event/state before approving, and assert the MCP fixture call marker before accepting completion.
- [Provider fixtures can accidentally prove their own assumptions] → Validate the same MCP fixture independently and assert native persisted status, provider projection, upstream protocol call, and final Agent message as separate evidence points.

## Migration Plan

No data migration is required. Existing MCP rows and the observability setting retain their schema. Rollback removes the OpenCode environment projection and restores conditional preparation; private invocation directories remain self-cleaning and startup scavenging handles stale artifacts after interrupted runs.
