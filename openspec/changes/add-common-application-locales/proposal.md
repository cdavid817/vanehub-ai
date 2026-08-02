## Why

VaneHub AI currently limits its application UI and native desktop copy to Simplified Chinese and English, even though its developer audience includes other common East Asian language communities. The existing binary language checks also make every additional locale error-prone, so the localization contract should become extensible before more translations are added.

## What Changes

- Add Traditional Chinese (`zh-TW`), Japanese (`ja`), and Korean (`ko`) as fully supported application locales in both Tauri desktop and Web/mock runtimes.
- Introduce one supported-locale registry contract for locale metadata, frontend resource loading, selector presentation, validation, fallback behavior, and locale-sensitive formatting.
- Replace binary Chinese/English UI and native-runtime branches with explicit locale resolution and a consistent fallback policy.
- Localize system-tray actions, the close-to-tray notice, and communications overload copy for every supported application locale, including updates after a language change where native copy remains visible.
- Generalize translation validation across all supported locale resources, including key parity, duplicate keys, interpolation variables, and plural forms.
- Establish plural-ready resource conventions so later Spanish, French, German, and Brazilian Portuguese additions do not require another localization architecture change.
- Keep the existing default application language and persisted setting compatibility; automatic system-locale detection and additional user-guide translations are outside this change.

## Capabilities

### New Capabilities

- `application-localization`: Defines supported application locales, complete frontend resource coverage, locale loading and fallback, native desktop copy, pluralization, formatting, and localization regression checks.

### Modified Capabilities

- `app-settings`: Expands the valid application-language setting beyond Simplified Chinese and English and requires every runtime to restore and apply any supported locale before showing the application surface.
- `settings-basic-configuration-ui`: Replaces the binary language selector behavior with a complete, accessible selector for every supported application locale.
- `desktop-background-lifecycle`: Requires system-tray actions and the close-to-tray notice to use the active supported locale and to refresh when the application language changes.

## Impact

- Frontend locale resources, i18next initialization/loading, language types and metadata, settings normalization, the Basic Configuration language selector, locale-sensitive formatting, and localization tests under `src/`.
- Rust desktop language validation and native localization used by tray lifecycle and communications infrastructure under `src-tauri/`.
- Existing settings service and Tauri/Web adapter contracts retain the same `applicationLanguage` string field; no React component gains direct Tauri access.
- Existing SQLite string persistence remains compatible and requires no schema migration.
- Optional locale resources may become separate Vite chunks; no replacement state-management, UI, or localization framework is introduced.
- README and user-guide language coverage remain unchanged by this application-runtime localization change.
