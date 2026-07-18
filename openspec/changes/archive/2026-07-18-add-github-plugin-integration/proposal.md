## Why

VaneHub has separate settings surfaces for Skills, MCP servers, and local extension capabilities, but it does not yet provide a unified place for product-level plugin integrations. Adding a first built-in GitHub plugin gives users a clear settings entry for GitHub readiness without introducing a broad third-party plugin runtime before its safety model is designed.

## What Changes

- Add a Plugin Integrations settings page to the settings center.
- Add a built-in GitHub plugin entry that exposes metadata, setup guidance, configuration status, and connection testing.
- Use `gh auth status` as the first desktop readiness check instead of storing a GitHub PAT in the first version.
- Add a plugin integration frontend service interface with matching Tauri desktop and Web/mock adapters.
- Add native commands for listing plugin integrations and testing the GitHub plugin in the Tauri runtime.
- Keep Web/mock behavior honest by showing deterministic plugin data and a localized desktop-only limitation for live checks.
- Localize all Plugin Integrations UI text in Simplified Chinese and English.
- Defer third-party plugin package installation, manifest parsing, plugin resource activation, and plugin marketplace behavior.

## Capabilities

### New Capabilities
- `plugin-integration-management`: Defines built-in plugin integration catalog, GitHub readiness checks, runtime adapter behavior, and logging/redaction expectations.
- `settings-plugin-integration-ui`: Defines the settings page, navigation entry, GitHub plugin card, visual consistency, search, status, and i18n behavior.

### Modified Capabilities
- `settings-center-ui`: Add Plugin Integrations to the settings page set and navigation ordering.
- `frontend-runtime-architecture`: Add plugin integration service boundary and desktop/Web adapter parity expectations.
- `unified-log-management`: Add persistent diagnostics expectations for plugin integration checks that execute native commands.

## Impact

- Frontend: new settings page, page registration, i18n resources, plugin integration types, service interface, Tauri adapter, Web/mock adapter, and focused React tests.
- Desktop runtime: new Rust command module for plugin integration catalog and GitHub readiness checks using a backend-owned command plan.
- Web runtime: deterministic mock adapter that does not claim to inspect host GitHub authentication.
- Native logging: GitHub readiness diagnostics and command failures must be persisted through the unified logging service with token/path redaction before disk writes.
- No new npm package, UI component library, state-management library, or frontend direct Tauri invocation is introduced.
