## 1. Native CLI Diagnostics

- [x] 1.1 Audit CLI refresh, executable detection, version probing, npm registry lookup, and package operation failure paths in `src-tauri/src/lib.rs`.
- [x] 1.2 Add unified log writes for CLI refresh start, per-CLI detection failures, npm view failures, timeout failures, partial refresh completion, and refresh operation failure.
- [x] 1.3 Add unified log writes for CLI package operation failures with operation id, CLI id, package name, target version, npm executable, explicit arguments, stdout, stderr, exit status or timeout reason, and sanitized environment context.
- [x] 1.4 Ensure all new durable logs use the existing unified logging service and redaction path instead of feature-local files or direct unredacted file writes.

## 2. Native CLI Package Operations

- [x] 2.1 Verify the backend CLI catalog entries for Claude Code, OpenCode, Codex CLI, and Gemini CLI use the correct agent ids, executable names, and npm package names.
- [x] 2.2 Ensure `install_cli_version` returns an operation id before npm work starts for all valid managed CLI ids and stable target versions.
- [x] 2.3 Ensure the package operation constructs npm with explicit arguments equivalent to `npm install -g <package>@<targetVersion>` for all four managed CLIs.
- [x] 2.4 Persist successful package-operation results and refresh the affected CLI status with the operation id after install, upgrade, or downgrade.
- [x] 2.5 Improve failed package-operation status and operation logs so users see a concise error while durable logs retain detailed diagnostics.

## 3. Frontend Refresh and Error State

- [x] 3.1 Update the CLI management page refresh button to show a dynamic loading indicator and refreshing label while the refresh mutation is pending or the returned refresh operation is queued/running.
- [x] 3.2 Keep CLI cards visible during refresh failures and refresh listed CLI statuses after refresh operation terminal states.
- [x] 3.3 Report refresh/package start failures through the frontend logging service boundary in Tauri runtime when no backend operation id is returned.
- [x] 3.4 Keep Web/mock adapter behavior interface-compatible by simulating refresh/package operations and loading states without local file writes.

## 4. Tests

- [x] 4.1 Add or update Rust tests covering CLI catalog package resolution, stable target-version validation, unsupported CLI rejection, and explicit npm package argument construction.
- [x] 4.2 Add or update Rust tests covering unified log persistence/redaction for CLI refresh and package operation failures.
- [x] 4.3 Add or update frontend tests for refresh button loading/disabled state and operation-status-driven refresh completion.
- [x] 4.4 Add or update frontend tests for install, upgrade, and downgrade action derivation across Claude Code, OpenCode, Codex CLI, and Gemini CLI status examples.

## 5. Verification

- [x] 5.1 Run `npm run test`.
- [x] 5.2 Run `npm run build`.
- [x] 5.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.4 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.5 Run `openspec validate --specs --strict`.
- [x] 5.6 Run `openspec validate fix-sdk-page-cli-errors --strict`.
