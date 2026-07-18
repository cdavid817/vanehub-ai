## 1. Service Contract

- [x] 1.1 Add shared plugin integration TypeScript models for integration metadata, readiness status, setup steps, test results, and runtime limitations.
- [x] 1.2 Add `PluginIntegrationService` with list, refresh, and test-readiness methods.
- [x] 1.3 Add Tauri and Web/mock plugin integration adapters with matching method signatures and normalized result shapes.
- [x] 1.4 Add adapter contract tests that verify the Web/mock adapter and Tauri adapter expose equivalent TypeScript shapes.

## 2. Desktop Runtime

- [x] 2.1 Add Rust plugin integration domain types and a backend-owned built-in GitHub integration catalog.
- [x] 2.2 Add a native GitHub readiness check that resolves `gh` and runs `gh auth status` without accepting frontend-provided command strings.
- [x] 2.3 Normalize GitHub readiness outcomes into configured, not configured, missing CLI, unavailable, and error statuses.
- [x] 2.4 Persist redacted GitHub readiness diagnostics through the unified logging service for command failures, missing executable, timeout, and unsafe output cases.
- [x] 2.5 Register Tauri commands for listing plugin integrations and testing the GitHub integration.
- [x] 2.6 Add Rust tests for catalog listing, unknown integration rejection, missing CLI handling, unauthenticated handling, and diagnostic redaction behavior where practical.

## 3. Settings UI

- [x] 3.1 Add the Plugin Integrations page and register it after Extension Capabilities and before MCP Servers in `settings-pages`.
- [x] 3.2 Build the GitHub plugin card using shared settings primitives, lucide icons, semantic tokens, and compact operational layout.
- [x] 3.3 Add plugin summary cards, search filtering, setup steps, documentation link, status pills, loading states, error states, and desktop-only Web/mock notice.
- [x] 3.4 Add zh-CN and en locale keys for navigation, search placeholder, page copy, GitHub metadata, setup steps, statuses, actions, notices, and errors.
- [x] 3.5 Add frontend tests for page registration order, search filtering, GitHub status rendering, Web/mock limitation rendering, and i18n resource parity.

## 4. Verification

- [x] 4.1 Run `openspec validate "add-github-plugin-integration" --strict`.
- [x] 4.2 Run `npm run test`.
- [x] 4.3 Run `npm run build`.
- [x] 4.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 4.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 4.6 Record any environment-specific failures or pre-existing unrelated failures in the implementation summary.
