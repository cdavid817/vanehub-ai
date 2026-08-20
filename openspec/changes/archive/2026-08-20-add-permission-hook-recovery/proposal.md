## Why

The permission hook is deliberately installed into Claude Code's *global* `settings.json` so that governance outlives the VaneHub process (fail-closed by design). But when the VaneHub instance dies without the user disabling hook management — a crash, a killed dev build, an e2e run's test client — every Claude Code session on the machine degrades to read-only (`Read`/`Glob`/`Grep` offline allowlist), and the only recovery today is a human hand-editing `~/.claude/settings.json`. Claude Code itself cannot self-recover: editing settings requires the `Edit` tool, which the very hook being removed denies. The deny message names the condition but no way out. A stale wrapper path left behind by an application update (the entry points at a binary location that no longer exists) has the same all-sessions blast radius with even less diagnosability.

This change keeps the fail-closed semantics untouched and fixes the recovery paths around them.

## What Changes

- Add a standalone `--uninstall` escape hatch to the `vanehub-permission-hook` wrapper binary: it removes only VaneHub-owned `PreToolUse` entries from Claude Code's global settings, preserves everything else, and works with no VaneHub instance running — recovery becomes one command instead of hand-edited JSON.
- Make the offline deny reasons actionable and state-differentiated: "discovery data present but the instance is unreachable" and "no discovery data at all" produce distinct messages, and both name the two recovery actions (start VaneHub, or run `vanehub-permission-hook --uninstall`).
- Reconverge the hook registration at desktop startup: when the `claude-code` principal already has an assigned template row, the startup path re-projects the hook entries so the wrapper path is refreshed after an application update; failure is best-effort and never blocks startup.
- The offline fallback allowlist, the fail-closed deny for reachable-but-malformed responses, and the global-settings installation model are explicitly unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `claude-code-permission-hook`: the wrapper gains a standalone uninstall mode, offline denials explain their recovery paths, and hook registration reconverges at desktop startup.

## Impact

- `crates/vanehub-permission-hook`: argument handling for `--uninstall`, settings-file surgery mirroring `cli_config`'s ownership marker, and differentiated deny reasons.
- `src-tauri/src/contexts/permissions`: a startup reconvergence entry point on `PermissionsApi`; `bootstrap/permissions.rs` wires it best-effort.
- No Tauri command signature changes, no SQLite schema changes, no frontend changes, no Web/mock runtime impact.
- Out of scope (recorded for a follow-up): preventing dev/e2e builds from installing the hook into the real user's global settings, and any per-session (`claude --settings`) injection model — both change the governance scope itself rather than its recovery paths.
