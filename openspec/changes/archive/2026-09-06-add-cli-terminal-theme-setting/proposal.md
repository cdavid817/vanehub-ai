## Why

The embedded terminal that hosts single-Agent CLI sessions renders on a fixed dark palette regardless of the application style. Users who run the application in the light `minimal` style, or who find dark terminals hard to read, have no way to get a light terminal, and the palette is coupled to the application theme by living on `:root`, so any change would also repaint ordinary Shell and remote terminals that must keep their own behavior.

## What Changes

- Add an application-level `cliTerminalTheme` setting (`"light" | "dark"`, default `"dark"`) carried through the shared settings model, both runtime adapters, the native settings domain, DTO, and the existing key/value persistence.
- Add a "CLI 会话主题" select under the application theme in Basic Configuration → common preferences, with a short compatibility note.
- Add a complete light palette beside the existing dark one, scoped to the Agent terminal container through a `data-cli-terminal-theme` attribute that the settings provider applies, so Shell and remote terminals keep the shared default.
- Make `createTerminalTheme()` read variables from a given element, falling back to the document root for existing callers, and let Agent terminals hot-swap `terminal.options.theme` on the attribute change without recreating, reconnecting, or clearing anything.
- Keep the application theme and the CLI terminal theme independent in both directions.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-settings`: Add the `cliTerminalTheme` key with strict validation, default, and Web/desktop parity.
- `settings-basic-configuration-ui`: Add the CLI terminal theme control to common preferences.
- `agent-terminal-runtime`: Define the light/dark palette contract, scoping, hot update, and the CLI-owned-color boundary.

## Impact

- `src/types/settings.ts`, `src/services/settings-service.ts`, both settings adapters, `src/settings/settings-provider.tsx`, `src/settings/pages/basic-settings-page.tsx`, `src/session-workspace/terminal-theme.ts`, `src/session-workspace/agent-terminal-tab.tsx`, `src/styles.css`, five locale files.
- `src-tauri/src/contexts/desktop/domain/settings.rs`, `src-tauri/src/commands/desktop/{dto,mapper}.rs`. No schema change: the value is one more row in the existing `settings` key/value table.
- No new dependency. No CLI flags, environment variables, or CLI restarts.
