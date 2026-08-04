## Why

The Basic Configuration page mixes everyday preferences with diagnostic and operational tools, producing a long page whose visual hierarchy does not match how frequently users need each setting. The page also omits the existing `defaultFolderPath` setting required by the confirmed specification.

## What Changes

- Reorganize Basic Configuration into four user-oriented groups: common preferences, startup and window behavior, workspace defaults, and advanced configuration.
- Use compact setting rows with left-aligned labels and descriptions and right-aligned controls for high-frequency preferences.
- Keep network proxy, logs, data management, and runtime information on the page but place them inside a collapsed advanced region for progressive disclosure.
- Expose and persist the existing default project directory setting through the shared settings provider.
- Present folder-opener discovery and ordering as an expandable management control while keeping the default opener immediately accessible.
- Present floating-assistant behavior alongside other startup and window behavior instead of as a separate bottom-only section.
- Move the broad reset action to a low-frequency footer area and require explicit confirmation before resetting settings.
- Preserve responsive behavior, localization parity, desktop/Web adapter boundaries, and existing native capability limitations.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-basic-configuration-ui`: Define the new grouped information architecture, progressive disclosure of advanced sections, default project directory control, and safer reset placement.
- `settings-floating-assistant-ui`: Allow the floating-assistant control to be grouped with startup and window behavior instead of requiring bottom-only placement.

## Impact

- Frontend settings components under `src/settings/pages/` and their localized resources and tests.
- Existing common settings persistence through `SettingsProvider`; no direct Tauri calls or service contract changes are introduced.
- Both desktop and Web/mock runtimes receive the same layout, while native-only controls retain their current availability semantics.
- No Rust, database, dependency, or adapter-boundary changes are expected.
