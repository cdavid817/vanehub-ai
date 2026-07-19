## Why

Developers need a fast, predictable way to open the active session's effective local workspace in the editor, file manager, or terminal they already use. VaneHub currently exposes session files and shells inside the application but does not discover common Windows development tools, remember user opener preferences, or safely launch the current worktree or project in an external program.

## What Changes

- Add a bounded Windows folder-opener catalog for Visual Studio Code, File Explorer, Windows Terminal, Git Bash, IntelliJ IDEA, and WebStorm, including installation detection and stable availability results.
- Add global preferences for one configured default opener and a multi-select enabled opener list, with File Explorer as the required fallback and atomic SQLite persistence.
- Add a split folder-opener control to the right of the session Logs and Report tabs. Its main action uses the effective default, while its menu lists enabled available openers and links to opener settings.
- Resolve launch targets on the native side using `worktreePath`, then `folder`, then `projectPath`; reject missing, deleted, and remote workspace targets in the first version.
- Launch only allowlisted programs through explicit executable paths and arguments without shell command concatenation, and record redacted detection and launch diagnostics through unified logging.
- Keep the Web/mock runtime contract-compatible with deterministic availability data and an explicit native-action-unavailable result.
- Add a Basic Settings section for default selection, supported-opener multi-select, detected status/path display, and manual refresh.

## Capabilities

### New Capabilities

- `workspace-folder-openers`: Defines the supported opener catalog, bounded Windows discovery, preference invariants, effective-default fallback, safe native session-directory resolution and detached launch, diagnostics, and Web/mock behavior.

### Modified Capabilities

- `session-workspace-tabs`: Adds the fixed right-side split opener control, accessibility behavior, narrow-layout behavior, and local/remote/unavailable session states.
- `app-settings`: Adds atomically persisted default and enabled folder-opener preferences and runtime restoration/event behavior.
- `settings-basic-configuration-ui`: Adds installation status, default opener, enabled opener multi-select, and refresh controls through the service boundary.

## Impact

- Frontend contracts and adapters in `src/services/agent-service.ts`, the Tauri adapter, and Web/mock adapter gain opener discovery, preference, refresh, and open-session-folder operations; React components continue to avoid direct Tauri invocation.
- The session tab toolbar and Basic Settings UI gain localized, icon-bearing opener controls and loading/error/fallback states.
- The Rust desktop and workspaces contexts coordinate through published APIs/ports: desktop owns opener discovery, preferences, and allowlisted launch planning, while workspaces owns session authorization and effective-root resolution.
- SQLite settings gain stable opener preference keys; detected executable paths and availability are not persisted.
- The shared process platform gains a bounded detached-spawn path for external GUI/terminal programs, with unified redacted diagnostics.
- Desktop runtime receives full native behavior; Web runtime preserves deterministic preview behavior without claiming local process side effects.
- No breaking public behavior or new third-party runtime dependency is expected.
