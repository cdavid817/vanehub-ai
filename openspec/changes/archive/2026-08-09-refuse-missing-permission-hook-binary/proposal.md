## Why

The permission-hook wrapper is a second binary (`src-tauri/src/bin/vanehub-permission-hook.rs`). `tauri.conf.json` declares no `bundle.resources` and no `bundle.externalBin`, so no packaged build ships it — `bootstrap/permissions.rs` records this as "deliberately out of scope for this pass".

Nothing checks for it before use. Enabling Claude Code permission management writes `PreToolUse` entries into Claude Code's **global** `settings.json` naming that path. In a packaged build the path does not exist, so Claude Code invokes a command that cannot start on every matching tool call — in a file VaneHub does not own, which keeps that state after VaneHub exits.

Confirmed on a packaged Windows build: `vanehub-permission-hook.exe` is absent, and `wrapper_binary_path` resolves through the Tauri resource directory to a path with nothing at it.

## What Changes

- Installing the Claude Code hook fails with an actionable error when the wrapper binary is not on disk, instead of writing entries that point at it.
- The user's global Claude Code settings are left untouched in that case.
- Removing the hook keeps working without the binary, so anyone who enabled it against an earlier build can still clean up.
- Package the wrapper as a Tauri external binary using a per-target build-and-rename step, and resolve the installed wrapper beside the main executable before considering the resource directory.
- Keep the missing-file guard as defense in depth for incomplete or damaged installations.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `claude-code-permission-hook`: hook installation gains a precondition — the wrapper binary must exist before any entry naming it is written to Claude Code's global settings.

## Impact

**Runtime scope: desktop only.** The Web runtime has no permission-hook surface. No React component, frontend service interface, or Tauri command signature changes; the new failure travels the existing `PermissionsApplicationError::Infrastructure` path that `assign_template` already surfaces.

Affected files:

- `src-tauri/src/contexts/permissions/infrastructure/claude_code_hook_adapter.rs`
- `src-tauri/src/bootstrap/permissions.rs` — resolves the packaged sidecar beside the application
- `src-tauri/tauri.conf.json`, `package.json`, and `scripts/prepare-permission-hook-sidecar.mjs` — prepare and declare the target-qualified external binary
- `.github/PREVIEW_RELEASE_NOTES.md` — describes failure behavior for incomplete installations

Behaviour change for users: supported packages carry the hook wrapper, while an incomplete or damaged installation reports an error instead of leaving Claude Code unable to run a missing hook.
