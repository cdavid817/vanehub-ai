## Context

The Tauri main window is declared in `src-tauri/tauri.conf.json` with a restored size of 1280×820 and minimum dimensions of 1100×700. No runtime code currently overrides its initial window state. The requested behavior applies only to the normal desktop main window; Web mode has no native window and the floating assistant has its own compact placement rules.

## Goals / Non-Goals

**Goals:**

- Start a newly launched desktop main window maximized to the operating system's available work area.
- Preserve the taskbar, native window controls, restored dimensions, minimum dimensions, close-to-tray behavior, and floating assistant behavior.

**Non-Goals:**

- Enter borderless or exclusive fullscreen mode.
- Force the window back to maximized after the user restores or resizes it.
- Add a user preference or persist a custom window placement.
- Change browser/Web layout scaling.

## Decisions

### Declare the initial state in Tauri configuration

Set `maximized: true` on the existing main window configuration. This uses Tauri's native startup-window contract and avoids React-side window calls, new capabilities, timing flicker, or platform-specific Rust code. Calling `maximize()` during setup was rejected because it can briefly display the restored window and duplicates declarative configuration.

### Preserve restored dimensions

Keep the existing width, height, minimum dimensions, and centering values. They remain the native restored-window geometry when the user selects Restore Down. The maximized state is not fullscreen, so normal operating-system chrome and taskbar behavior remain intact.

### Keep runtime boundaries unchanged

No React component, service interface, Tauri adapter, Web/mock adapter, SQLite code, or Rust command changes are required. The Web runtime remains responsive to its browser viewport, and the floating assistant continues to use its independent runtime-created window.

## Risks / Trade-offs

- [A user may prefer a smaller initial window] → This change intentionally establishes maximized startup as the product default while preserving immediate native restore/resize controls.
- [Displays provide different work areas and scale factors] → Delegate sizing to the operating system's native maximize behavior instead of calculating pixels.
- [Future window-state persistence may conflict with the default] → Treat this declarative value as the fallback initial state; a future explicit preference can supersede it through a separate specification.

## Migration Plan

No migration is required. Existing installations receive the new initial window state on the next process launch. Reverting the single Tauri configuration field restores the previous fixed-size launch behavior.

## Open Questions

None.
