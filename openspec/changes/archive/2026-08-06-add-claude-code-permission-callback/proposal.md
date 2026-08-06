## Why

Phase 1 (`add-permissions-core`) built a full permission decision point (`evaluate()` / `ApprovalBroker`) but wired it only to native API agents. Claude Code CLI — and the other three CLI agents — run entirely outside it today: every tool call happens invisibly inside a PTY, governed only by Claude Code's own native (or absent) prompting. Claude Code is the only one of the four CLI agents that exposes a mechanism (`PreToolUse` hooks) capable of acting as a genuine synchronous permission gate in interactive mode, confirmed this session against official docs and corroborated independently by a sibling project's (clowder-ai) prior art. This change bridges that hook into VaneHub's existing decision pipeline instead of building a second, parallel permission engine.

## What Changes

- New loopback HTTP server (infrastructure adapter in `permissions`, bound to `127.0.0.1` with a random port and bearer token regenerated every launch) that receives `PreToolUse` requests from a new hook wrapper process, translates them into `Action`/`Resource`, and resolves them through the existing `evaluate()` / `ApprovalBroker.create_pending()` pipeline. Reuses the existing `ApprovalCard` UI and `permission:request` event unmodified — no new approval UI.
- New minimal Rust binary target: the hook wrapper Claude Code actually spawns per tool call. Reads the `PreToolUse` stdin payload, calls the loopback server with a bounded client-side timeout, and translates the response into Claude Code's `permissionDecision`/exit-code contract. When the server is unreachable (not running, connection refused, timeout, or the local discovery file is missing), applies a small hardcoded, conservative allowlist: only known read-only tools fail open, everything else fails closed.
- New stable principal `claude-code` — one global identity for this machine, not scoped per project or session, consistent with how every other principal in the system is already identified by agent id alone. The Agent Policies settings surface gains a row for it using the existing template-picker/confirmation pattern.
- Enabling permission management for the `claude-code` principal writes a VaneHub-owned entry into the user's global `~/.claude/settings.json`. This also affects Claude Code CLI usage outside VaneHub (a plain terminal session), so the first time a template is assigned to this principal, the system requires a distinct one-time confirmation naming that side effect — separate from, and in addition to, the existing trusted/yolo confirmation.
- `cli_config` gains a new, independent operation to install or remove just the VaneHub-owned `hooks.PreToolUse` entry in Claude Code's `settings.json` — atomic write, preserves every other hook entry, fingerprint-drift detected — decoupled from provider/profile application (profile switching is unaffected).
- Tool-to-`Action` mapping is deliberately partial: only `Bash`→`shell.exec`, `Edit`/`Write`→`file.write`, `Read`/`Glob`/`Grep`→`file.read`, and MCP tools (`mcp__*`)→`mcp.tool` are matched by the hook. Other tools (e.g. `WebFetch`) are not matched and fall through to Claude Code's native behavior unchanged — no new `Action` variants are introduced by this change.

## Capabilities

### New Capabilities
- `claude-code-permission-hook`: the loopback HTTP bridge, the hook wrapper binary, the tool-to-`Action` mapping, and the risk-tiered offline fallback that connects Claude Code's `PreToolUse` hook to VaneHub's existing permission decision pipeline.

### Modified Capabilities
- `permissions-core`: "Unified permission decision model" currently scopes evaluation to "a native API agent" only; broadened so a CLI-agent-originated evaluation reaching the same decision point through the new hook bridge is evaluated identically.
- `permissions-approval`: "Agent policy list surfaces every eligible agent's current template" currently lists only agents with `agentOrigin === "user"` plus OnePiece; broadened to include the new stable `claude-code` principal. New requirement for the distinct first-use hook-installation consent step described above.
- `cli-agent-config-management`: new requirement for an independent Claude Code permission-hook projection operation (install/remove the VaneHub-owned `hooks.PreToolUse` entry while preserving unrelated hook entries), decoupled from profile application.

## Impact

- Desktop runtime only. The Web/mock runtime simulates the new principal and settings row deterministically per existing parity requirements; it performs no real hook installation or file access.
- New Rust binary target changes build/packaging: a second compiled artifact ships alongside the main application binary.
- Touches `permissions` (new infrastructure adapter, no PDP logic changes), `cli_config` (new independent operation), and the Agent Policies settings UI (new row + consent dialog, reusing existing components).
- No change to `agent_runtime` or the interactive PTY launch mechanism — Claude Code continues to launch exactly as it does today; only its `settings.json` gains a managed hook entry.
- Requires new fault-injection test coverage (server crash, timeout, malformed/protocol-drifted response) as a prerequisite gate before implementation, per this phase's inherited design constraint.
