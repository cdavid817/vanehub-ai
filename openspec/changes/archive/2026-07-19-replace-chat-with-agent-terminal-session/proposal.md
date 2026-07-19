## Why

The current session page treats CLI agents as a custom chat composer and message stream, which hides the real CLI interface and duplicates configuration controls that already exist in CLI Parameter settings. Replacing the chat-first surface with a real session-scoped Agent terminal makes the desktop experience match how developers actually use Claude Code, Codex CLI, Gemini CLI, and OpenCode while keeping launch configuration centralized.

## What Changes

- **BREAKING**: Replace the main session Chat panel with an Agent Terminal panel for single-Agent CLI sessions.
- Automatically start the selected Agent CLI after a single-Agent session is created.
- Launch Agent CLIs through native-owned shell wrappers: PowerShell on Windows, `cmd` fallback when PowerShell is unavailable, and the default shell on macOS/Linux.
- Inject only the selected Agent's saved CLI Parameter profile using the `interactive` launch scope; remove first-version session-page model, permission, reasoning, thinking, and streaming selectors from the Agent terminal experience.
- Persist provider runtime session ids and reuse them to resume a session after the live CLI process has been closed.
- Keep live Agent CLI processes attached to sessions across session switching and page closure, then stop idle terminal processes after 30 minutes or during application shutdown.
- Keep the existing ordinary Shell tab for project shell work; it remains separate from Agent Terminal and does not receive Agent CLI parameters.
- Add a create-session mode choice where Single Agent is supported and Multi Agent is visible as disabled/coming soon and cannot be submitted.
- Preserve Web/mock usability with equivalent metadata and placeholder terminal behavior without claiming local CLI execution.
- Persist runtime session id and redacted run diagnostics only; terminal transcript output is not converted into `messages` rows in the first version.

## Capabilities

### New Capabilities
- `agent-terminal-runtime`: Defines session-scoped real Agent CLI terminals, shell-wrapper launch behavior, process retention, idle cleanup, runtime session id persistence, resume behavior, and frontend service/event contracts.

### Modified Capabilities
- `main-layout-ui`: Replace the chat-first main content requirement with an Agent Terminal main content requirement and add create-session Single Agent / disabled Multi Agent mode selection.
- `session-management`: Clarify selected Agent ownership, runtime session id persistence for terminal resume, and lifecycle coherence with retained Agent terminal processes.
- `cli-parameter-management`: Require Agent Terminal launches to use saved `interactive` scope parameters with no session-page override controls in the first version.
- `native-runtime-architecture`: Require native-owned shell wrappers and terminal process registry behavior to remain behind bounded context, service, command, operation, and logging boundaries.

## Impact

- Frontend: `src/main-layout`, `src/session-workspace`, `src/services/agent-service.ts`, `src/services/tauri-agent-client.ts`, `src/services/web-agent-client.ts`, i18n resources, and focused UI tests.
- Native desktop runtime: `agent_runtime` application/domain/infrastructure, Tauri commands, bootstrap registration, process/PTY adapters, CLI profile integration, session gateway updates, lifecycle/cleanup jobs, and unified logging.
- Persistence: existing `sessions.runtime_session_id` is reused; additional terminal registry state may remain in memory unless implementation requires additive SQLite metadata.
- Runtime boundaries: React components continue to use the frontend service interface only; Tauri `invoke()` stays in runtime adapters; shell wrapper and CLI argument construction stay in Rust/native code.
- Web runtime: mock adapter remains functional by exposing the same Agent terminal service surface with simulated/placeholder behavior and without local CLI process access.
