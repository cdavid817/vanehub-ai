## Why

The desktop main window currently opens at a fixed 1280×820 size, leaving much of the available workspace unused on larger displays. Opening maximized gives the session workspace its full usable area immediately without requiring a manual maximize action.

## What Changes

- Configure the Tauri main window to start maximized on a fresh desktop application launch.
- Preserve normal maximized-window semantics: the operating-system taskbar remains available and the user can restore, resize, minimize, or close the window normally.
- Keep the existing fallback dimensions and minimum size for restored-window mode.
- Leave the Web/mock runtime and floating-assistant window behavior unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-background-lifecycle`: Define the main desktop window's initial maximized presentation while preserving existing tray restore and background lifecycle behavior.

## Impact

- Tauri desktop window configuration only.
- No React service, Web adapter, Rust command, database, or frontend/backend boundary changes.
