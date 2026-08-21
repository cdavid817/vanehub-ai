## Why

VaneHub installs its Claude Code permission hook in the user's global settings, so the hook currently intercepts Bash and file mutations even when Claude Code was launched independently from a normal terminal. Those unmanaged sessions can be denied when VaneHub is unavailable or forced to wait for approval in a different application, which breaks Claude Code's native permission experience outside VaneHub.

## What Changes

- Mark Claude Code processes launched by VaneHub as managed permission-hook sessions.
- Make the global hook wrapper produce no decision for unmarked Claude Code sessions so Claude Code's native permission flow remains authoritative.
- Preserve the existing authenticated VaneHub evaluation, approval, audit, timeout, and offline fail-closed behavior for marked sessions.
- Keep Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI scoped to their existing launch-time permission projections without adding global hooks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `claude-code-permission-hook`: Restrict VaneHub permission decisions to VaneHub-managed Claude Code sessions and pass unmanaged sessions through without approving or denying them.
- `cli-agent-permission-launch-flags`: Project an explicit permission-hook ownership marker only into VaneHub-managed Claude Code launches.

## Impact

- Desktop runtime only; the Web/mock runtime remains unchanged.
- Affects the Claude Code sidecar wrapper and the native Agent Runtime CLI profile projection used by chat and interactive terminal launches.
- Does not add frontend service APIs or cross the existing frontend/backend boundary.
- Does not change policy-template meanings or the native launch controls used by the other managed CLIs.
