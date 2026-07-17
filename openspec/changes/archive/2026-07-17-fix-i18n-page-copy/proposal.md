## Why

Several user-facing pages still contain hard-coded English or Chinese text, while existing locale resources only cover part of the settings center and workspace. This causes language switching to produce mixed-language UI and leaves future page changes without a clear i18n contract.

## What Changes

- Audit all React pages and shared UI modules for user-visible hard-coded text, including settings pages, workspace layout, chat controls, dialogs, modal titles, confirmations, notices, placeholders, status labels, and empty states.
- Fix incorrect or mismatched zh-CN and en translation values so equivalent keys convey the same product meaning in both languages.
- Add missing translation keys for pages that currently bypass i18n, especially Agents, SDK Dependencies, MCP Servers, create-session dialog, chat selectors, workspace information tabs, and frontend Web/mock user-facing messages.
- Add tests or scripts that verify zh-CN/en key parity and catch newly introduced hard-coded user-facing literals in page components.
- Update project-level standards so future page or UI changes must support both Simplified Chinese and English through i18n resources.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `frontend-runtime-architecture`: Add an application-wide localization contract for all frontend user-visible text and locale-aware formatting.
- `settings-center-ui`: Expand localized settings center behavior to cover all settings pages, dialogs, forms, operation notices, and empty/error states.
- `main-layout-ui`: Require workspace shell, create-session dialog, information panel, sidebar, status bar, and session actions to use synchronized zh-CN/en translations.
- `chat-experience`: Require chat UI labels, selectors, status text, placeholders, and role labels to use synchronized zh-CN/en translations.

## Impact

- Desktop runtime and Web runtime are both affected because the same React UI runs in both contexts.
- Frontend components must use `react-i18next` translation keys for user-visible copy rather than hard-coded literals.
- Locale resources under `src/i18n/locales/` will grow and must stay key-aligned.
- Frontend tests should be updated where assertions currently assume a single Chinese or English literal.
- Project standards documentation must include the i18n requirement for future UI/page work.
