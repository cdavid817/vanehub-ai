# Terminal and PTY runtime

Single-Agent CLI sessions run inside a session-scoped Agent Terminal: a PTY-backed CLI process owned by the native runtime and exposed to React through the frontend Agent service boundary. React components never call Tauri commands directly for terminal lifecycle.

## Session-scoped, single-Agent

The Agent Terminal is for non-archived single-Agent CLI sessions. A terminal start requested for an archived session is rejected without launching a CLI process and returns a concise user-displayable failure.

## Automatic start and attach

After a single-Agent session is created or selected, the UI automatically requests Agent Terminal startup for that session — no separate launch button. If the selected session already has a live retained Agent Terminal process, the UI attaches to the existing terminal stream instead of spawning a duplicate CLI process for the same session.

## Remote terminals

Remote SSH workspaces expose their own remote terminal runtime path; the local PTY ownership model does not extend to remote sessions unchanged. See the user guide for the remote-workspace workflow and [Native bounded contexts](native-contexts.md) for the `workspaces`/`sessions` ownership split.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/agent-terminal-runtime](../../../openspec/specs/agent-terminal-runtime/spec.md)
- [openspec/specs/remote-terminal-runtime](../../../openspec/specs/remote-terminal-runtime/spec.md)
- [openspec/specs/session-shell](../../../openspec/specs/session-shell/spec.md)

The PTY and shell runtime lives in the `workspaces` and `sessions` bounded contexts; see [Native bounded contexts](native-contexts.md).
