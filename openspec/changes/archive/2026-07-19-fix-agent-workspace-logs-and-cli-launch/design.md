## Context

The previous change replaced the session chat panel with a real Agent terminal for single-Agent CLI sessions. Follow-up usage exposed two product issues and one runtime reliability issue: visible labels still describe the center tab as Agent Terminal instead of Workspace, the right information panel lacks a quick launch-log view, and Claude Code/Codex CLI failures on Windows are hard to diagnose from the UI.

The existing session log capability already exposes logs through `agentService.listSessionLogs`, and the native terminal runtime already writes redacted start/exit diagnostics under `session.agent_terminal`. This change should reuse those contracts instead of adding a second log model.

## Goals / Non-Goals

**Goals:**

- Make user-facing labels match the new workspace concept.
- Add a compact Logs tab to the right information panel using the existing session log service.
- Add a bottom multiline composer to the Agent CLI workspace that sends submitted text to the live terminal through the existing Agent terminal input service.
- Record startup failures before returning native process errors.
- Normalize Windows npm-style shim executable paths for managed CLI tools when a concrete package binary can be found.

**Non-Goals:**

- Do not convert terminal output into chat messages.
- Do not add full log export/search controls to the compact info panel; the main Logs tab remains the detailed viewer.
- Do not change saved CLI parameter semantics.
- Do not implement Multi Agent session creation.

## Decisions

- Reuse `agentService.listSessionLogs` in `SessionInfoPanel`. This preserves the frontend service boundary and keeps the right panel a lightweight overview of the same log data used by the detailed Logs tab.
- Submit workspace input through `agentService.sendAgentTerminalInput`. The xterm remains fully interactive, while the composer submits with Enter, supports Shift+Enter for new lines, and appends a carriage return so submitted commands behave like pressing Enter in the terminal.
- Keep log persistence in the native unified logging path. The terminal launcher records startup, failure, exit, stop, idle cleanup, and shutdown diagnostics without adding feature-local files.
- Add a generic Windows shim resolver with known package binary candidates for `claude-code`, `codex-cli`, and `opencode`. If no concrete binary exists, the configured executable is left unchanged so custom installations still work.
- Keep internal component names and service method names stable where possible. The rename is user-facing text, not a domain rewrite.

## Risks / Trade-offs

- Windows CLI packages may change their internal binary layout. The resolver only uses paths that exist on disk and falls back to the configured executable.
- The compact Logs tab may show more than startup logs for the same session. Filtering to the `session.agent_terminal` category keeps it focused on terminal diagnostics while the full Logs tab remains available for broader logs.
- The composer does not inspect shell prompts or command completion state. It only forwards user-entered text to the current terminal session and disables itself when no terminal id is attached.
- Some launch failures can happen before wrapper creation. The native launcher records each failure point with the session and agent id so the UI can surface the latest available diagnostic.
