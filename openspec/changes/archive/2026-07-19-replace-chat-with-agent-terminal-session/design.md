## Context

The workspace currently centers on a chat composer and persisted message stream. Desktop chat execution already runs provider CLIs through `agent_runtime`, and CLI Parameter settings already persist typed launch profiles. The requested behavior changes the primary session experience: a single-Agent session should open a real interactive CLI terminal, launched by the native runtime with the selected Agent's saved interactive parameters.

The existing ordinary project shell should remain available as a separate workspace tab. The new Agent terminal is not a generic shell; it is a session-scoped Agent runtime owned by `agent_runtime`, because it needs Agent registry lookup, CLI executable resolution, CLI parameter projection, provider resume grammar, runtime session id persistence, lifecycle updates, and unified diagnostics.

## Goals / Non-Goals

**Goals:**

- Replace the main chat panel with an Agent Terminal panel for single-Agent sessions.
- Add create-session mode selection with enabled Single Agent and disabled Multi Agent placeholder.
- Automatically start or attach the selected Agent terminal after session creation and when selecting a session.
- Preserve live Agent terminal processes across session switching and page closure.
- Stop retained terminal processes after 30 minutes of inactivity and during application shutdown.
- Resume closed terminals using persisted `runtimeSessionId` when supported by the provider.
- Launch CLIs through native-owned shell wrappers and inject only saved `interactive` CLI profile arguments.
- Keep frontend calls behind service interfaces and keep Tauri `invoke()` in runtime adapters.
- Keep Web/mock mode usable with simulated terminal state and no local process claim.

**Non-Goals:**

- Implement Multi Agent runtime orchestration.
- Persist complete terminal transcripts or convert terminal output into `messages` rows.
- Keep session-page model, permission, reasoning, thinking, or streaming selectors for Agent Terminal.
- Replace the existing ordinary Shell tab.
- Remove the existing headless chat runtime from backend code paths used by other integrations.

## Decisions

### Agent terminal is owned by `agent_runtime`

The new use cases live in `agent_runtime` rather than `workspaces`.

Rationale: Agent terminal startup depends on stable agent ids, availability, provider-specific CLI grammar, runtime session ids, and CLI Parameter profiles. Those are Agent runtime concepts. The `workspaces` context remains responsible for ordinary project shells and bounded file/Git access.

Alternative considered: extend `workspaces::shell` with Agent CLI launch modes. That would reuse more existing PTY code but would make `workspaces` depend on Agent registry and tooling profile semantics. The implementation should instead extract reusable PTY/process utilities at the platform edge if needed while keeping business use cases in `agent_runtime`.

### Frontend gets an Agent terminal service surface

The frontend service boundary is extended with terminal operations such as create/attach, input, resize, kill, and event subscription. React components render xterm and call the service only; `tauri-agent-client` maps those calls to Tauri commands, and `web-agent-client` provides a simulated terminal contract.

Rationale: This preserves the existing runtime adapter pattern and keeps React independent of Tauri commands.

### Native wrappers launch the CLI

Desktop launch constructs a wrapper script owned by Rust/native code:

- Windows prefers PowerShell with `-NoLogo -NoProfile -ExecutionPolicy Bypass -File <wrapper.ps1>`.
- Windows falls back to `cmd.exe /d /s /c <wrapper.cmd>` when PowerShell is unavailable.
- macOS/Linux use the user's default shell or a platform default to run a generated wrapper.

The wrapper changes to the session folder when available and invokes the resolved CLI executable with distinct validated arguments. Logs contain only redacted command diagnostics.

Rationale: wrapper files avoid fragile shell string interpolation in frontend code, keep process setup auditable, and allow platform-specific setup while preserving native ownership.

### CLI profile projection uses `interactive` scope only

Agent Terminal loads the selected Agent's saved profile and projects arguments with `CliParameterLaunchScope::Interactive`. Session-page chat configuration selectors are removed from the Agent Terminal surface and do not override profile values.

Rationale: Settings become the single source of CLI launch configuration for interactive terminals. This avoids the current duplicated configuration model.

### Terminal registry supports attach, idle cleanup, and shutdown cleanup

The native runtime keeps an in-memory registry keyed by session id. Opening a session follows:

```text
open session
  |-- live retained terminal exists -> attach
  |-- no live terminal + runtimeSessionId exists -> start resume
  `-- no live terminal + no runtimeSessionId -> start fresh
```

User input, output activity, resize, and attach update `last_active_at`. A background cleanup job stops terminals idle for more than 30 minutes. Application shutdown stops all retained terminal processes.

Rationale: retaining processes across session switching matches the requested desktop behavior while bounded cleanup prevents unmanaged background process growth.

### Runtime session id is the durable resume anchor

Provider output or terminal metadata is parsed for a provider runtime session id when available. The id is persisted on the existing session record. When a retained process is no longer live, the next start uses provider-specific resume grammar.

Rationale: the existing session model already exposes `runtimeSessionId`, and persisting only this identifier keeps first-version storage small.

## Risks / Trade-offs

- Provider session id extraction can differ across CLI versions -> keep provider-specific parsers covered by fixtures, and degrade to fresh launch with a visible non-resumable state when no id is available.
- Retained terminal processes can consume resources -> enforce one live Agent terminal per session, 30-minute idle cleanup, and shutdown cleanup.
- Shell wrapper quoting is platform-sensitive -> generate wrapper files with literal path handling and test paths containing spaces, quotes, and non-default shells.
- Removing the chat panel is a visible behavior break -> make the change explicit in specs and keep backend chat runtime intact for integrations that still need headless execution.
- Web/mock parity cannot launch local CLIs -> expose simulated terminal state that preserves UI and service behavior without implying real process access.

## Migration Plan

1. Add Agent terminal service, commands, events, and native registry behind `agent_runtime`.
2. Reuse `sessions.runtime_session_id`; add no migration unless implementation discovers a durable metadata gap.
3. Extend frontend service interfaces and both runtime adapters.
4. Replace the workspace main Chat panel with Agent Terminal while keeping the ordinary Shell tab.
5. Add create-session mode selection and disable Multi Agent submission.
6. Keep existing persisted messages readable for search/export, but do not render them as the primary session surface.
7. Rollback by restoring Chat as the main panel and leaving the new terminal commands unused; persisted `runtimeSessionId` remains compatible metadata.

## Open Questions

- Whether future transcript persistence should use unified logs only, a terminal transcript table, or `messages` after a separate specification.
- Whether Multi Agent should eventually create one retained terminal per Agent or use an orchestrator process.
