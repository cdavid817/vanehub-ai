## 1. Contracts And Service Boundary

- [x] 1.1 Define shared TypeScript contracts for CLI tool status, CLI refresh result, and CLI package operation inputs.
- [x] 1.2 Extend the frontend service boundary with methods to list cached CLI statuses, refresh CLI detections asynchronously, install a selected CLI version, and read related operation status/logs.
- [x] 1.3 Update the Tauri adapter to call declared Tauri commands only from the runtime adapter layer.
- [x] 1.4 Update the Web adapter to expose the fixed CLI catalog with unsupported native detection behavior and no fake installed states.

## 2. Native CLI Catalog And Persistence

- [x] 2.1 Add backend-owned CLI catalog metadata for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode` in the fixed display order.
- [x] 2.2 Add SQLite persistence for last-known CLI detection status, versions, path, errors, timestamps, and last operation id.
- [x] 2.3 Implement read-only cached CLI status loading without executing local commands or npm network calls.
- [x] 2.4 Preserve agent stable ids and keep CLI metadata compatible with the existing agent registry contract workflow.

## 3. Async Detection And Version Refresh

- [x] 3.1 Implement an asynchronous refresh operation that resolves executable path and current local version for each CLI.
- [x] 3.2 Implement asynchronous npm registry version checks for latest version and available versions.
- [x] 3.3 Filter available versions to the latest 20 stable versions and exclude prerelease versions.
- [x] 3.4 Persist partial per-CLI results and errors so one failed CLI check does not discard other results.
- [x] 3.5 Ensure detection and npm version checks do not block the Tauri main thread or frontend rendering.
- [x] 3.6 Automatically start one asynchronous CLI refresh on first startup when no persisted detection result exists.

## 4. Async Install, Upgrade, And Downgrade

- [x] 4.1 Implement a backend-managed operation that accepts only `agentId` and `targetVersion`.
- [x] 4.2 Resolve npm packages from the backend whitelist and execute `npm install -g <package>@<targetVersion>` with explicit process arguments.
- [x] 4.3 Derive install, upgrade, downgrade, current-version, and disabled states from current and selected versions.
- [x] 4.4 Record stdout, stderr, success, failure, and timestamps in operation logs.
- [x] 4.5 Refresh and persist the affected CLI status after a successful package operation.

## 5. CLI Management UI

- [x] 5.1 Rename the settings navigation entry and page title from Provider Management to `CLI 管理`.
- [x] 5.2 Replace the provider list with four fixed CLI cards in the agreed order.
- [x] 5.3 Render only `CLI 已安装` and `CLI 未安装` summary cards.
- [x] 5.4 Remove API Key, URL, presets, enable, edit, delete, active provider count, add provider, and empty provider configuration UI.
- [x] 5.5 Show installed state, current version, latest version, local install path, last checked time, and detection errors.
- [x] 5.6 Show install command and copy action for missing or undetected CLIs.
- [x] 5.7 Add version selection and action buttons for install, upgrade, downgrade, and current-version states.
- [x] 5.8 Show each CLI card's most recent operation state with expandable logs.
- [x] 5.9 Keep the page interactive while refresh or package operations run, disabling only affected controls.
- [x] 5.10 Localize all user-visible text in synchronized zh-CN and en resources.

## 6. Verification

- [x] 6.1 Add unit tests for version filtering, action derivation, and Web adapter unsupported behavior.
- [x] 6.2 Add Rust tests for CLI catalog lookup, command argument construction, cached status reads, and stable-version filtering.
- [x] 6.3 Add or update frontend tests for initial cached rendering, refresh operation state, and card-local logs.
- [x] 6.4 Add or update tests for first-startup automatic CLI refresh behavior.
- [x] 6.5 Run `npm run test`.
- [x] 6.6 Run `npm run build`.
- [x] 6.7 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.8 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.9 Run `openspec validate "replace-provider-management-with-cli-management" --strict`.
