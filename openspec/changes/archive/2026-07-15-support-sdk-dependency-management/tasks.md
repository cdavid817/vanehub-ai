## 1. Frontend Service Model

- [x] 1.1 Add SDK dependency TypeScript types for SDK ids, definitions, statuses, version info, environment status, operation logs, and operation results.
- [x] 1.2 Add `SdkService` with methods for listing status, checking environment, loading versions, checking updates, installing, updating, rolling back, uninstalling, and reading operation logs.
- [x] 1.3 Add a Web/mock SDK adapter with deterministic mock SDK status, version options, simulated install/update/rollback/uninstall behavior, and simulated logs.
- [x] 1.4 Add a runtime SDK service factory that selects the Tauri adapter in desktop runtime and the Web/mock adapter otherwise.

## 2. Tauri SDK Backend

- [x] 2.1 Add a Rust `sdk` module with catalog definitions for `claude-sdk` and `codex-sdk`, including package names, companion packages, fallback versions, and descriptions.
- [x] 2.2 Implement dependency path resolution for `~/.vanehub/dependencies/<sdk-id>/` with normalized path safety helpers.
- [x] 2.3 Implement SDK status detection by checking managed package directories and reading installed package versions from package metadata.
- [x] 2.4 Implement Node.js and npm detection with user-displayable environment status.
- [x] 2.5 Implement npm registry version lookup, fallback version responses, latest-version detection, and version comparison.
- [x] 2.6 Implement backend semver normalization and validation for requested versions.
- [x] 2.7 Implement install/update/rollback using backend-owned package definitions and npm argument vectors with `--ignore-scripts`.
- [x] 2.8 Implement uninstall by deleting only the normalized managed SDK directory and updating dependency metadata.
- [x] 2.9 Register SDK Tauri commands in the app invoke handler and expose a TypeScript Tauri adapter for the SDK service.

## 3. SDK Settings Page

- [x] 3.1 Replace `SdkPage` demo data usage with SDK service-backed loading, refresh, environment, status, and version state.
- [x] 3.2 Render managed SDK cards with status, current version, latest version, install path, related provider hints, and error state.
- [x] 3.3 Add compact version selection controls that derive install, update, rollback, and current-version actions from installed and selected versions.
- [x] 3.4 Add install, update, rollback, uninstall, refresh, and check-update actions with one-operation-at-a-time disabling.
- [x] 3.5 Add an operation log panel that displays logs for the active SDK operation and preserves state across settings navigation.
- [x] 3.6 Keep the page aligned with existing settings-center styling, shared controls, semantic tokens, and responsive layout.

## 4. Agent Readiness Integration

- [x] 4.1 Extend agent registry/readiness metadata so SDK-backed agents can reference a managed SDK dependency by stable SDK id.
- [x] 4.2 Include missing SDK dependency reasons in agent availability results without launching interactive sessions.
- [x] 4.3 Ensure existing command-based availability checks still work for CLI-backed agents that do not declare a managed SDK dependency.

## 5. Tests

- [x] 5.1 Add frontend unit tests for version normalization/action derivation and SDK settings page behavior.
- [x] 5.2 Add Web/mock adapter tests for status, version, and simulated operation flows.
- [x] 5.3 Add Rust tests for SDK id validation, semver validation, package spec construction, path safety, version parsing, and uninstall path blocking.
- [x] 5.4 Add or update agent readiness tests for installed and missing SDK dependency states.

## 6. Verification

- [x] 6.1 Run `openspec validate "support-sdk-dependency-management" --strict`.
- [x] 6.2 Run `npm run build`.
- [x] 6.3 Run `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; $env:CARGO_NET_OFFLINE="true"; cargo check --manifest-path src-tauri\Cargo.toml`.
- [x] 6.4 Manually verify the SDK settings page in Web/mock runtime.
- [x] 6.5 Manually verify desktop SDK status detection and one non-destructive operation path before testing real install/uninstall flows.
