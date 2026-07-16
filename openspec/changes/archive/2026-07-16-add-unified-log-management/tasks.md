## 1. Settings And Contracts

- [x] 1.1 Extend frontend settings types with log directory and read-only logging policy metadata.
- [x] 1.2 Extend `SettingsService` with log directory save/open operations and keep Tauri `invoke()` calls inside the Tauri settings adapter.
- [x] 1.3 Add a frontend client log event service contract for persistent-worthy UI errors and critical user operation failures.
- [x] 1.4 Extend the Web/mock settings and client log adapters to return a mock log path, expose fixed policies, report open-directory as unavailable, and use no-op or mock behavior for client log events.
- [x] 1.5 Extend Rust settings models and validation for the persisted log directory setting.
- [x] 1.6 Add or update settings normalization and client log service tests for log directory, logging policy defaults, and Web/mock client event behavior.

## 2. Native Unified Logging Service

- [x] 2.1 Create a Rust unified logging module/service that resolves the active log directory with an app-owned default.
- [x] 2.2 Implement log entry writing with `error`, `warn`, `info`, and `debug` levels plus feature or operation context.
- [x] 2.3 Implement built-in redaction before persistence for password, token, secret, API key, bearer token, and common provider key patterns.
- [x] 2.4 Implement log directory validation/creation and ensure path changes affect only newly written logs.
- [x] 2.5 Implement automatic archival for log files older than 30 days under an archive location in the active log directory.
- [x] 2.6 Expose native commands for reading logging metadata, saving the log directory, opening the active log directory, and accepting frontend client log events.
- [x] 2.7 Add Rust tests for redaction, invalid directory rejection, no migration on path change, retention/archive behavior, and persisted frontend client events.

## 3. Existing Log Integration And Audit

- [x] 3.1 Audit current SDK, CLI, task registry, and native diagnostic logging paths for nonconforming log persistence.
- [x] 3.2 Route SDK install, update, rollback, and uninstall operation output through the unified logging service while preserving SDK page logs.
- [x] 3.3 Route CLI detection, version refresh, install, upgrade, and downgrade output through the unified logging service while preserving CLI card logs.
- [x] 3.4 Route native command failures and backend-managed task diagnostics through the unified logging service where diagnostic logging is currently missing or ad hoc.
- [x] 3.5 Route React error boundary failures and selected critical user operation failures through the client log event service.
- [x] 3.6 Add regression tests or focused unit tests proving SDK/CLI operation logs and frontend client events are persisted through unified logging while existing UI flows still work.

## 4. Basic Settings UI

- [x] 4.1 Add a Basic Settings log management section showing the active log directory and fixed logging policies.
- [x] 4.2 Add desktop log directory change behavior through `SettingsService` without direct Tauri calls from React components.
- [x] 4.3 Add an open log directory action that is enabled in desktop runtime and disabled in Web/mock runtime.
- [x] 4.4 Add user-facing error handling for invalid log directory saves and unavailable Web/mock open-directory behavior.
- [x] 4.5 Add frontend tests for desktop-capable display behavior, Web/mock disabled behavior, and settings service calls.

## 5. Documentation And Verification

- [x] 5.1 Document the unified logging development contract in the relevant OpenSpec capability so future native features use the logging service.
- [x] 5.2 Run `openspec validate "add-unified-log-management" --strict`.
- [x] 5.3 Run `npm run test`.
- [x] 5.4 Run `npm run build`.
- [x] 5.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.6 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
