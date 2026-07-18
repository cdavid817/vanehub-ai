## Why

The CLI Management and About pages already expose the core local CLI environment data, but the management workflow is still missing the compact "local environment check" affordances users expect from cc-switch: diagnose conflicts, refresh, and upgrade every eligible tool from one place.

## What Changes

- Add a service-backed "upgrade all" action for supported npm-managed CLI tools that are behind their latest stable version.
- Support managed CLI install and upgrade plans across source-specific methods such as wget-based official scripts and npm, preferring the existing installation method during upgrades and wget-based official scripts for first installs when available.
- Keep local environment checking compact and operational: diagnostics, refresh, current version, latest version, upgrade state, active path, conflict information, and logs remain visible without a marketing-style layout.
- Adjust About page environment summary presentation to stay aligned with the optimized CLI Management page while avoiding lifecycle controls there.
- Preserve both desktop and Web/mock runtime behavior: desktop performs native operations; Web/mock reports localized unavailable states.
- Keep all new visible text in Simplified Chinese and English i18n resources.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-cli-management-ui`: Add bulk upgrade behavior and tighten the CLI Management/About page presentation contract around local environment checks.

## Impact

- Frontend service boundary: extend `src/services/agent-service.ts` plus matching Tauri and Web adapters for bulk CLI package mutation.
- Frontend UI: update CLI Management page/cards, About page summary copy/layout, i18n resources, and focused tests.
- Desktop runtime: add Tauri commands backed by asynchronous native operations that serialize per-tool CLI lifecycle mutations, choose a safe source-specific install method, and refresh status afterward.
- Web runtime: return a localized unsupported operation task without inspecting or mutating host CLIs.
