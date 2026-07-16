## 1. Native Settings Persistence

- [x] 1.1 Add a versioned SQLite migration for a key-value `settings` table in app-owned storage.
- [x] 1.2 Implement Rust data types and validation for supported setting keys and values.
- [x] 1.3 Implement `get_settings` to return persisted settings merged with defaults.
- [x] 1.4 Implement `save_setting` to validate and upsert a single setting.
- [x] 1.5 Implement `get_node_info` to return read-only Node.js path/version or an unavailable result.
- [x] 1.6 Register the new Tauri commands and add focused Rust tests for settings persistence and validation.

## 2. Frontend Service Boundary

- [x] 2.1 Extend the frontend service interface with common settings and Node.js environment operations.
- [x] 2.2 Implement the Tauri adapter using the new Rust commands without exposing `invoke()` to React components.
- [x] 2.3 Implement the Web adapter using localStorage-backed settings and Web-safe Node.js mock/unavailable data.
- [x] 2.4 Share defaults and value validation between SettingsProvider and adapters where practical.

## 3. Settings Provider and Global Application

- [x] 3.1 Add or update SettingsProvider to load settings once, expose mutation helpers, and preserve loading/error state.
- [x] 3.2 Synchronize language setting changes with i18next.
- [x] 3.3 Apply font size by setting the root `html` font size and remove any global `zoom` usage.
- [x] 3.4 Apply visual theme by setting the document theme attribute used by CSS variable groups.
- [x] 3.5 Ensure invalid persisted values fall back to defaults without breaking app rendering.

## 4. Basic Configuration and Layout UI

- [x] 4.1 Build the Basic Configuration page with four setting controls: language, font size, visual theme, and default folder path.
- [x] 4.2 Display Node.js executable path and version as a read-only environment section with unavailable handling.
- [x] 4.3 Route all Basic Configuration mutations through SettingsProvider.
- [x] 4.4 Remove the workspace sidebar visual-style cycle control while preserving Settings and Help utility actions.
- [x] 4.5 Confirm settings center scrolling and layout remain stable at 12px, 14px, 16px, and 18px root font sizes.

## 5. i18n Extraction

- [x] 5.1 Replace hard-coded Chinese user-facing strings in the targeted React/TypeScript files with translation keys.
- [x] 5.2 Add matching zh-CN and en translation entries for settings, layout, chat, MCP, SDK, provider, Agent, and Skills surfaces.
- [x] 5.3 Verify zh-CN and en translation resources have identical key sets.
- [x] 5.4 Search TS/TSX sources for remaining hard-coded Chinese UI text and resolve intentional exceptions.

## 6. Verification

- [x] 6.1 Run `openspec validate "complete-general-settings-i18n-font-fix" --strict`.
- [x] 6.2 Run `npm run test`.
- [x] 6.3 Run `npm run build`.
- [x] 6.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
