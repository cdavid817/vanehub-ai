## Why

Launching VaneHub while it is already running starts a second process. The desktop runtime registers no single-instance guard, so every activation of the desktop icon, shortcut, or start-menu entry reaches `tauri::Builder` and builds an independent application.

Close-to-tray makes this the normal path rather than an edge case. `desktop-background-lifecycle` already requires the main window to hide instead of exiting when the user closes it, so a user who believes VaneHub is closed and clicks the icon again gets a duplicate process instead of the window they wanted.

The duplicate is not a cosmetic second window. Each instance opens the same SQLite database from the same application data directory, installs its own process-wide logging sink, claims its own tray icon, and starts its own retention, retrieval, scheduled-task, and connector jobs. Two writers on one profile is a data-integrity hazard, and the shipped tray affordance for restoring the window is exactly the behavior the duplicate launch bypasses.

`desktop-background-lifecycle` already states the invariant for the tray path — the restore action must surface the existing window "without starting a second application instance" — but no requirement covers a launch requested from the operating system. This change closes that gap in both the specification and the runtime.

## What Changes

- Register a desktop single-instance guard so a launch requested while an instance is already running surfaces the running instance instead of starting a second process.
- Restore the running instance's main window on a duplicate launch by showing, unminimizing, and focusing it, so the guard works when the window is hidden in the tray or minimized.
- Keep helper subprocesses that re-execute the same binary — the MCP relay — outside the guard, so they are never mistaken for duplicate launches.
- Exclude the guard from the `desktop-e2e` test client, whose instance lock would otherwise collide with an installed VaneHub on the same machine.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-background-lifecycle`: A launch requested while an instance is already running surfaces the running instance's main window instead of starting a second application process.

## Impact

- Desktop bootstrap plugin registration in the Tauri builder chain.
- Main-window restoration on duplicate launch, reusing the existing show/unminimize/focus sequence.
- One new Rust dependency, `tauri-plugin-single-instance`; no database migration and no frontend change.
- Native unit tests plus a bootstrap wiring assertion; the `desktop-e2e` client deliberately does not carry the guard, so desktop end-to-end layers do not cover it.
