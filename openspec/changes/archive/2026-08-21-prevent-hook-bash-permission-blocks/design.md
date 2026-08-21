## Context

See `proposal.md` for motivation. Claude Code is the only managed CLI whose VaneHub permission integration is installed in a global user settings file. The sidecar currently turns every matched invocation into an explicit `allow` or `deny`, while Claude Code defines successful empty hook output as no decision and continuation through its native permission flow.

The Agent Runtime already carries a per-launch environment map for both chat and interactive terminal processes. Hook subprocesses inherit the Claude Code process environment, so ownership can be declared at the existing native launch boundary without adding frontend APIs or persistent state.

## Goals / Non-Goals

**Goals:**

- Distinguish VaneHub-managed Claude Code processes from independently launched processes before any discovery or network work occurs.
- Preserve fail-closed behavior for managed sessions when the VaneHub permission server is unavailable.
- Preserve Claude Code's native permission prompts and settings for unmanaged sessions.
- Keep the other managed CLI launch mappings byte-for-byte equivalent except for tests that prove they do not receive the Claude marker.

**Non-Goals:**

- Parsing Bash command strings to infer file paths or command safety.
- Replacing the global hook projection with a generated `--settings` document.
- Adding a hook enable/disable UI or changing stored policy-template semantics.
- Changing Web/mock behavior.

## Decisions

### D1. Use a child-process environment marker

The Agent Runtime will add `VANEHUB_PERMISSION_HOOK_SCOPE=managed` only when projecting a `claude-code` launch. This reuses the environment channel already consumed by chat and terminal process adapters and naturally propagates to the hook subprocess.

A static ownership marker is sufficient because it selects which permission system should answer; it is not an authentication credential. The existing per-launch discovery bearer token remains the authentication boundary for requests that reach the loopback server.

Alternatives considered:

- Injecting a temporary Claude `--settings` value risks replacing or reassembling the user's existing `hooks` key and creates additional lifecycle and quoting complexity.
- Parsing session ids or working directories cannot reliably prove that VaneHub owns a process.

### D2. Represent unmanaged execution as no decision

The wrapper decision model will add a pass-through outcome. When the marker is absent or has any value other than `managed`, the wrapper exits successfully without writing stdout. It must not return `allow`: Claude Code documents `allow` as skipping its permission prompt, while empty successful output preserves the normal permission flow.

The scope check occurs immediately after stdin parsing and before discovery-file access. Malformed stdin remains fail-closed because the wrapper cannot safely determine the request contract, while a valid unscoped request remains passive.

### D3. Preserve managed-session policy behavior

Marked sessions continue through the existing wrapper request, authenticated loopback server, policy evaluation, approval broker, audit, and offline fallback. No policy template or grant matching changes are required.

Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI retain their current provider-native flags and OpenCode environment projection. Tests will make the absence of the Claude-specific marker explicit.

## Risks / Trade-offs

- [A user manually sets the marker in an external shell] → That user explicitly opts into VaneHub evaluation; loopback authentication and current offline behavior still apply.
- [A future launch path omits the shared environment map] → Cover both chat and interactive profile projection with native unit tests.
- [The global hook still starts a small sidecar for unmanaged calls] → Return before filesystem or network access; removing global projection is a larger lifecycle change outside this fix.
- [A malformed unmanaged payload still denies] → Preserve the existing fail-closed contract because ownership and event validity cannot be established safely.

## Migration Plan

No data migration is required. Existing global hook entries continue pointing at the updated sidecar. After upgrade, unmanaged sessions become passive automatically, while newly launched VaneHub Claude Code processes receive the marker. Rolling back restores the previous global enforcement behavior without changing stored data.
