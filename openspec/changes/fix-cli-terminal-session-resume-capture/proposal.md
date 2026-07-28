## Why

Interactive CLI-backed sessions persist the selected Agent id, but their provider `runtime_session_id` is usually never recorded because the embedded TUI does not emit the structured JSON events consumed by the current parser. After the process or desktop app restarts, reopening the VaneHub session therefore starts a new provider conversation instead of restoring that session's history.

## What Changes

- Capture an exact provider session id for every newly opened Claude Code, Codex CLI, Gemini CLI, and OpenCode terminal without relying only on structured PTY output.
- Assign and persist a caller-supplied id at successful launch for providers that support it.
- For providers that allocate their own id, correlate the newly created provider record with the launch baseline and working directory, persisting only a unique match.
- Reopen a stopped CLI-backed VaneHub session with its persisted provider session id; never substitute a global "most recent session" when the exact id is missing.
- Fail closed on ambiguous discovery and record a redacted diagnostic through unified logging instead of associating the wrong provider history.
- Preserve the existing frontend Agent service boundary and Web/mock runtime-session-id behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-terminal-runtime`: Make creation-time runtime session id capture provider-aware and exact, and require reopen to use only the owning VaneHub session's persisted provider id.

## Impact

- Desktop runtime: provider invocation construction, Agent Terminal process startup/monitoring, provider session-store discovery, and existing session metadata persistence.
- Web runtime: no contract change; deterministic mock runtime session ids remain supported.
- Data: no SQLite migration; the existing `sessions.runtime_session_id` column remains authoritative.
- Frontend/backend isolation: unchanged. React continues through `src/services/agent-service.ts`; provider-specific behavior remains native-owned behind the Agent runtime adapter.
- Dependencies: no new frontend package or alternative package manager; native implementation reuses existing Rust dependencies and standard filesystem access.
