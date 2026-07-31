## Context

VaneHub already stores the selected stable Agent id and has a nullable `sessions.runtime_session_id` column. The native Agent Terminal also knows how to build provider-specific resume arguments. The missing link is initial interactive TUI startup: the current runtime looks for structured session-id events in PTY output, while normal Claude Code, Codex CLI, Gemini CLI, and OpenCode TUIs emit ANSI-rendered screens rather than JSON lines.

Provider capabilities differ:

- Claude Code and Gemini CLI accept a caller-supplied UUID for a fresh session and resume that UUID later.
- Codex CLI allocates its own thread id and writes it in the first `session_meta` record of a new rollout under the Codex session store.
- OpenCode allocates its own `ses_...` id and writes a session row containing the working directory and creation time in its data-store SQLite database.

The React service interface, Tauri adapter, and Web/mock adapter already expose sufficient session metadata. This change is native-only except for preserving existing Web/mock parity.

## Goals / Non-Goals

**Goals:**

- Persist the exact provider session id belonging to each newly opened CLI-backed VaneHub session.
- Resume that exact provider session after the terminal or desktop process restarts.
- Avoid associating a VaneHub session with another concurrently created provider session.
- Keep provider argument and provider-store knowledge in the native Agent runtime infrastructure.
- Preserve unified redacted diagnostics and the existing frontend service boundary.

**Non-Goals:**

- Persisting or reconstructing the terminal transcript as VaneHub chat messages.
- Importing provider conversation contents into VaneHub-owned storage.
- Recovering historical VaneHub sessions that already have a NULL runtime session id by guessing a "most recent" provider conversation.
- Changing the CLI selection UI, session schema, Tauri command DTOs, or Web/mock service interface.

## Decisions

### 1. Use provider-aware acquisition, preferring caller-assigned ids

For a fresh Claude Code or Gemini CLI process, `build_interactive_invocation` generates a UUID and returns it with the invocation specification. It adds the provider's fresh-session argument and the Agent Terminal returns that id only after the PTY child has spawned successfully. The application service then persists it through the existing session gateway before reporting the session as running.

For resume, the builder never generates a replacement id. It emits only the provider-specific resume arguments for the stored id.

Alternative considered: parse all TUI output. Rejected because ANSI screen updates are not a stable provider API and do not consistently expose a session id.

### 2. Discover provider-allocated ids from a launch baseline

For a fresh Codex CLI or OpenCode terminal, native infrastructure captures a provider-store baseline before spawning the process:

- Codex baseline: known rollout paths/ids in the provider session directory. New rollout candidates are validated by parsing only their first `session_meta` JSON line and comparing the recorded `cwd` with the terminal working directory.
- OpenCode baseline: known ids from the provider's SQLite session table. New candidates are filtered by creation time and normalized working directory.

During PTY monitoring, discovery is retried at a bounded cadence after terminal activity. A single new matching id is persisted and published through the existing runtime-session-id event path. No provider conversation content is copied.

Alternative considered: invoke `resume --last` or `--continue`. Rejected because "latest" is global/provider-directory state and can restore a different VaneHub session.

Alternative considered: query the provider CLI through a second long-running process. Rejected because it adds startup latency and shell/shim complexity. Read-only access to provider-owned metadata is smaller and deterministic.

### 3. Fail closed on ambiguity or unavailable metadata

Discovery persists an id only when exactly one new candidate is absent from the launch baseline and its provider metadata matches the terminal working directory. Zero candidates remain pending. Multiple candidates, malformed metadata, or an unavailable provider store produce redacted diagnostics and do not update `runtime_session_id`.

The runtime keeps the structured PTY session-id parser as a valid additional source. Whichever exact source persists first ends discovery; later empty or unrelated values do not replace the association.

Alternative considered: choose the newest candidate. Rejected because timestamps do not prove ownership when multiple CLI sessions start together.

### 4. Keep persistence and runtime boundaries unchanged

The Agent Terminal infrastructure discovers or assigns the id, while the Agent runtime application persists it through `AgentSessionGateway`. SQLite ownership stays in the sessions context. React components continue to call `openAgentTerminal` through `agent-service.ts`; Tauri `invoke()` remains in `tauri-agent-client.ts`. The Web/mock adapter retains its deterministic `web-runtime-<session-id>` behavior and performs no provider-store access.

No new logging file is introduced. Capture success, ambiguity, and read failures use the existing unified Agent terminal logging port with session and stable Agent correlation.

## Risks / Trade-offs

- [Provider storage formats or locations change] → Isolate readers behind provider-specific capture code, validate metadata before use, cover supported shapes with fixtures/tests, and fail closed with redacted diagnostics.
- [Provider creates its durable session only after first user input] → Keep discovery active during terminal output rather than requiring the id to exist immediately at process spawn.
- [Two external sessions start concurrently in the same working directory] → Require a unique candidate relative to the captured baseline; never select merely by newest timestamp.
- [Assigned id is persisted but the provider later fails during initialization] → Persist only after successful child spawn; later provider failure remains visible through existing lifecycle and diagnostic handling.
- [Provider session record is deleted before reopen] → Preserve the exact stored id and surface the provider's resume failure instead of silently starting unrelated history.

## Migration Plan

No schema migration is required. New sessions begin capturing ids after deployment. Existing rows with a valid `runtime_session_id` resume unchanged; existing NULL rows start fresh and acquire an id only when a new provider session can be correlated exactly.

Rollback is code-only. Persisted ids remain compatible with the pre-change resume path.

## Open Questions

None.
