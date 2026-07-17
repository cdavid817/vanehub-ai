## Why

Users need a dedicated place to confirm product identity, runtime positioning, supported agent ecosystem, and project metadata without mixing that information into editable basic settings.

## What Changes

- Add an About page to the settings center navigation.
- Display localized product overview, runtime support, supported AI coding agents, version/build metadata, GitHub repository, changelog, and update status.
- Place About as the final settings navigation tab.
- Provide a check-for-updates action backed by the GitHub Releases API with graceful failure handling.
- Keep the page frontend-only and compatible with both Tauri desktop and Web/mock runtimes.
- Reuse existing settings page layout primitives, semantic visual tokens, lucide icons, and i18n resources.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `settings-center-ui`: Extend the settings page set with a localized About page.

## Impact

- Frontend: `src/settings/settings-pages.ts`, a new settings page component, an About update-check service, locale resources, and focused tests.
- Desktop runtime: Tauri CSP allows GitHub Releases API requests; no native API or Rust command changes.
- Web runtime: no adapter changes; the page renders from frontend-local metadata.
- Dependencies: no new packages.
