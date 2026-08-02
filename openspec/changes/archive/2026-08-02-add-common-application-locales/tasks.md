## 1. Localization Contract and Standards

- [x] 1.1 Update `openspec/project.md` so every registered application locale, rather than only zh-CN and en, must have complete semantically aligned user-visible resources and active-locale formatting.
- [x] 1.2 Add a typed frontend supported-locale registry for `zh-CN`, `en`, `zh-TW`, `ja`, and `ko` with deterministic order, label keys, text direction, and local resource loaders; derive `AppLanguage` validation from it without introducing `any`.
- [x] 1.3 Refactor i18next initialization and SettingsProvider hydration to load bundled optional locale chunks, await the selected resource before first render, preserve `zh-CN` default/fallback behavior, and retain a localized recoverable error on load failure.
- [x] 1.4 Update settings normalization plus Tauri and Web/mock adapter tests to accept and preserve every supported locale through the unchanged `applicationLanguage` string contract.

## 2. Language Selector and Frontend Resources

- [x] 2.1 Replace the Basic Configuration Chinese-versus-English selector branch with registry-driven options and add recognizable localized names for every supported locale.
- [x] 2.2 Add a complete, reviewed `zh-TW` resource matching the canonical English key and interpolation contract, including independent review of regional terminology and destructive-action copy.
- [x] 2.3 Add a complete, reviewed `ja` resource matching the canonical English key and interpolation contract.
- [x] 2.4 Add a complete, reviewed `ko` resource matching the canonical English key and interpolation contract.
- [x] 2.5 Review high-risk settings, confirmations, errors, accessibility labels, notifications, chat controls, CLI/SDK/MCP terminology, and stable literal exceptions across all five resources.

## 3. Pluralization and Locale Formatting

- [x] 3.1 Inventory every count-sensitive translation and update call sites to pass numeric `count` values instead of preformatted strings where plural selection is required.
- [x] 3.2 Convert count-sensitive resources to explicit i18next v4 `_one`/`_other` pairs in every registered locale, allowing identical localized values where the language has no grammatical-number distinction.
- [x] 3.3 Add focused tests for singular/plural selection, interpolation variables, and Intl/i18next date, time, number, and percentage formatting in representative supported locales.

## 4. Native Desktop Localization

- [x] 4.1 Extend the Rust `ApplicationLanguage` domain type, parsing, serialization, defaults, persistence fixtures, mapper tests, and setting-service tests for `zh-TW`, `ja`, and `ko`.
- [x] 4.2 Introduce one Rust native localization catalog covering tray show/hide/quit labels, close-to-tray title/body, and communications overload copy for all supported locales with deterministic `zh-CN` fallback.
- [x] 4.3 Add a desktop settings locale side-effect port and Tauri lifecycle implementation that retains native menu handles and refreshes existing tray labels after a saved language change without exposing native calls to React.
- [x] 4.4 Route communications overload copy through the shared native catalog so new messages observe the current saved locale and remove raw Chinese-versus-non-Chinese branching.
- [x] 4.5 Add Rust tests for the exact supported locale set, native copy coverage, fallback behavior, live tray-label update behavior where testable, and redacted warning behavior when a native refresh fails.

## 5. Localization Regression Coverage

- [x] 5.1 Generalize locale-resource tests to discover the registered resources and verify key parity, raw duplicate keys, interpolation-token parity, valid plural suffixes, and registry/resource alignment for all five locales.
- [x] 5.2 Add frontend tests for initial hydration, delayed optional-resource loading, failed-resource fallback, immediate switching, persisted restoration, and registry-driven selector rendering in Tauri-compatible and Web/mock flows.
- [x] 5.3 Add representative render assertions for each supported locale across navigation, Basic Configuration, chat, notifications, dialogs, and error/empty states without relying on fallback text.
- [x] 5.4 Add Playwright desktop-width and narrow-width locale smoke scenarios that verify switching/persistence and detect clipping or overlap in representative long-text surfaces.

## 6. Verification

- [x] 6.1 Run `npm run lint`, `npm run test`, and `npm run build` and resolve all frontend, resource, and localization failures.
- [x] 6.2 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` and resolve all native localization failures and warnings.
- [x] 6.3 Run the relevant Playwright locale scenarios and manually review representative zh-CN, en, zh-TW, ja, and ko desktop/narrow screenshots for semantic and layout defects.
- [x] 6.4 Run `openspec validate "add-common-application-locales" --strict` and `openspec validate --specs --strict`, then record the implementation verification results before archive.

## Verification Results

Verified on 2026-08-02:

- `npm run lint`: passed.
- `npm run test`: 102 files and 378 tests passed.
- `npm run build`: passed, including the frontend lazy-chunk policy check.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 975 passed, 3 intentionally ignored, plus 11 architecture tests passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml`: passed without warnings.
- `npx playwright test tests/e2e/application-locales.spec.ts`: 2 scenarios passed; all five locales switched and restored at 1440×900 and 390×844.
- Manual screenshot review: all ten locale/viewport captures had readable copy, contained selectors, and no horizontal clipping or overlapping controls.
- `openspec validate add-common-application-locales --strict`: passed.
- `openspec validate --specs --strict`: all 81 main specifications passed.

Warning remediation re-verified on 2026-08-02:

- Focused SettingsProvider, active-locale formatting, usage panel, memory panel, and execution timeline tests: 5 files and 18 tests passed.
- Real chat-message, notification-center, and CLI-conflict-dialog localization renders: all 5 registered locales passed.
- `npm run lint`: passed.
- `npm run test`: 103 files and 380 tests passed.
- `npm run build`: passed, including the frontend lazy-chunk policy check.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 975 passed, 3 intentionally ignored, plus 11 architecture tests passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed without warnings.
- `npx playwright test tests/e2e/application-locales.spec.ts`: 4 scenarios passed, including non-settings Japanese workspace, notification, and create-session dialog bounds at desktop and narrow widths.
- `openspec validate add-common-application-locales --strict`: passed.
- `openspec validate --specs --strict`: all 81 main specifications passed.

## 7. Verification Warning Remediation

- [x] 7.1 Replace raw initial settings-load errors with localized user-displayable copy while retaining the original cause for diagnostics, and cover the fallback behavior in SettingsProvider tests.
- [x] 7.2 Route remaining frontend-owned date, time, and number formatting through the active application locale and add focused regression assertions.
- [x] 7.3 Render real chat, notification, and dialog components in supported locales and extend Playwright long-text coverage beyond the Basic Configuration surface.
- [x] 7.4 Reconcile the design and task wording with the implemented i18next v4 `_one`/`_other` resource convention used consistently across registered locales.
- [x] 7.5 Re-run focused and full frontend validation, relevant native tests, Playwright locale scenarios, and strict OpenSpec verification after remediation.
