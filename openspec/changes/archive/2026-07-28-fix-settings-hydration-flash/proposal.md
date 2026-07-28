## Why

The desktop and Web surfaces currently render their full React trees before persisted application settings are loaded and applied. Because the root font size initially falls back to the browser default and changes after the asynchronous settings read, startup visibly resizes the interface and can also flash the wrong theme or language.

## What Changes

- Add an explicit settings-hydration boundary that keeps the formal application surface hidden until normalized settings have been applied.
- Apply persisted font size, theme, and language before rendering settings-dependent children.
- Fall back to the shared default settings when the initial read fails, then render the application with the existing error state available to settings consumers.
- Add focused frontend regression coverage for delayed successful hydration and failed hydration.
- Preserve the existing settings service and desktop/Web runtime adapter boundaries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-settings`: Require startup restoration or fallback settings to be applied before the formal application surface first becomes visible.

## Impact

- Affects both the Tauri desktop and Web/mock surfaces because both use `SettingsProvider`.
- Primarily changes `src/settings/settings-provider.tsx` and its frontend tests.
- Does not change Tauri commands, SQLite schemas, public service interfaces, or runtime adapter parity.
- Introduces no new dependencies and keeps React components isolated from direct Tauri calls.
