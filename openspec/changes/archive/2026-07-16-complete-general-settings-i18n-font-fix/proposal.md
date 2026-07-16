## Why

The Basic Configuration page needs to become the single reliable place for common application preferences across desktop and Web runtimes. Current hard-coded Chinese UI text and ineffective font-size switching make language and accessibility preferences inconsistent, while sidebar theme switching creates a second competing entry point.

## What Changes

- Add service-backed common settings management for language, font size, visual theme, default folder path, and read-only Node.js environment details.
- Persist settings through a Tauri SQLite key-value table in desktop runtime and localStorage in Web runtime.
- Synchronize language changes with i18next and keep zh-CN/en translation resources fully aligned.
- Fix font-size switching by applying root-level `html` font sizing so Tailwind rem-based sizing scales consistently without `zoom` side effects.
- Move visual style switching exclusively to Basic Configuration and remove the workspace sidebar's visual-style cycle control.
- Keep React components behind service interfaces and update both Tauri and Web adapters for new settings operations.

## Capabilities

### New Capabilities

- `app-settings`: Common user settings persistence, application of language/font/theme preferences, and Node.js environment display behavior.

### Modified Capabilities

- `settings-center-ui`: Basic Configuration page requirements change from prototype/local controls to service-backed common settings controls.
- `frontend-runtime-architecture`: Frontend service boundary requirements expand to include common settings adapters for both Tauri and Web runtimes.
- `native-runtime-architecture`: Native runtime requirements expand to include SQLite-backed app settings storage and Node.js environment inspection commands.
- `main-layout-ui`: Sidebar utility requirements change so Settings remains available but visual style switching is no longer exposed in the workspace sidebar.

## Impact

- Frontend: SettingsProvider, settings service interfaces, Tauri/Web adapters, Basic Configuration UI, i18n resources, theme/font application, and removal of sidebar theme switching.
- Backend: SQLite migration for `settings`, Rust settings commands, Node.js environment inspection, and command registration.
- Runtime scope: affects both Tauri desktop and browser Web runtimes; desktop persists through SQLite, Web persists through localStorage.
- Adapter boundary: React components continue to call frontend service interfaces only; direct Tauri `invoke()` remains isolated to the Tauri adapter.
- Verification: OpenSpec validation plus frontend tests/build and Rust checks/tests for settings persistence and command behavior.
