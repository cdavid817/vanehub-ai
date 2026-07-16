## Context

The settings center already exists as the shared desktop/Web configuration surface, but Basic Configuration must become the authoritative entry point for common app preferences rather than a prototype page. The change crosses React context state, i18next synchronization, Tailwind token behavior, runtime adapters, SQLite persistence, and Rust command registration.

The project constraints require React components to depend on service interfaces only. Desktop-specific persistence and Node.js inspection must live behind the Tauri adapter and Rust commands, while browser preview behavior must remain usable through a Web adapter.

## Goals / Non-Goals

**Goals:**

- Provide a SettingsProvider-managed common settings model for language, font size, theme, and default folder path.
- Persist common settings in SQLite for desktop runtime and localStorage for Web runtime.
- Synchronize app language with i18next and keep zh-CN/en resources aligned.
- Apply font-size changes through root `html` font size so Tailwind rem values scale consistently.
- Apply theme changes through `data-theme` and CSS variable groups, with Basic Configuration as the only theme switch entry.
- Surface Node.js path/version as read-only environment information.

**Non-Goals:**

- Adding a new UI component library or state manager.
- Changing the app's top-level routing model.
- Implementing agent-specific launch or session behavior.
- Supporting arbitrary custom font sizes, custom themes, or folder browsing beyond the default folder path setting.

## Decisions

1. Use a single SettingsProvider as the frontend orchestration point.

   SettingsProvider owns loaded settings, mutation helpers, and side effects for i18next, `html` font size, and `data-theme`. This keeps UI controls simple and avoids duplicating preference-application logic across pages.

   Alternative considered: each Basic Configuration control directly calls services and applies DOM side effects. That would make persistence and rendering order harder to test and would spread runtime concerns into UI components.

2. Extend the existing frontend service boundary with common settings methods.

   The service contract exposes `getSettings`, `saveSetting`, and `getNodeInfo`-style operations through the same adapter pattern used by other runtime data. The Tauri adapter calls Rust commands; the Web adapter uses localStorage and mock Node.js information.

   Alternative considered: calling Tauri `invoke()` from SettingsProvider. That violates the project boundary and would break browser preview.

3. Persist desktop settings as SQLite key-value rows.

   A `settings` table with stable string keys keeps the schema flexible for small preference values and avoids a wide table migration for each new setting. Values are normalized and validated at the Rust/frontend boundary before application.

   Alternative considered: frontend-only localStorage for desktop. That would not meet the local-storage-through-Rust architecture and would diverge from other app-owned desktop persistence.

4. Use root font size instead of CSS `zoom`.

   Tailwind sizing is rem-oriented when configured and can scale consistently from `html { font-size: ... }`. CSS `zoom` changes visual scaling without layout semantics and caused bottom whitespace, so it is not used.

   Alternative considered: applying font size to a content wrapper. Existing `px` or explicit component text classes can override inheritance, so wrapper font sizing is not reliable for global scaling.

5. Make Basic Configuration the only theme switching entry.

   Theme changes are settings, not workspace session actions. Removing the sidebar cycle button avoids conflicting state updates and keeps the sidebar utility row focused on navigation/help.

   Alternative considered: keeping both entries synchronized. That increases UI ambiguity without adding capability.

## Risks / Trade-offs

- Existing hard-coded `px` values may not scale with root font-size -> audit common Tailwind text/spacing usage and convert UI sizing that should scale to rem-based classes or CSS variables where needed.
- i18n extraction can miss dynamic or split strings -> compare zh-CN/en key sets and search for remaining hard-coded Chinese in TS/TSX files.
- SQLite migration can affect existing desktop users -> use additive migration and default values when keys are absent.
- Web and desktop settings can drift in behavior -> keep identical TypeScript interfaces and shared validation/defaults before delegating persistence.
- Node.js command discovery can be slow or unavailable -> return nullable path/version fields and display a read-only unavailable state rather than blocking settings rendering.

## Migration Plan

1. Add the SQLite `settings` table through a versioned migration.
2. Register Rust commands for loading settings, saving one setting, and reading Node.js environment information.
3. Extend frontend service interfaces and implement both Tauri and Web adapters.
4. Add SettingsProvider side effects for i18next, root font size, and `data-theme`.
5. Update Basic Configuration UI and remove sidebar theme switching.
6. Extract remaining Chinese strings into zh-CN/en resources and verify key parity.
7. Run OpenSpec, frontend, and Rust validation commands.

Rollback is additive: keep migration harmless, fall back to default settings if persisted values are missing or invalid, and restore prior UI behavior by reverting the frontend provider/page changes if needed.
