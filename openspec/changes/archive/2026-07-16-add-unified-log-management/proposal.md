## Why

VaneHub currently exposes SDK and CLI operation logs in feature-specific UI surfaces, but does not define one durable logging contract for diagnostics, operation output, retention, redaction, and archival. A unified log management capability is needed so users can locate logs consistently and future features follow the same logging rules instead of adding ad hoc log handling.

## What Changes

- Add a unified log management capability for desktop diagnostics and operation logs.
- Persist SDK and CLI operation logs to the configured log directory, while keeping existing in-page operation log display behavior.
- Add a log management section under Basic Settings / common settings with the current log directory, an open-directory action, and read-only policy information.
- Allow users to change the log directory; the change affects only newly written logs and does not migrate older log files.
- Apply built-in sensitive information redaction before log lines are persisted.
- Apply automatic retention and archival for expired logs with a fixed 30-day retention policy for the first version.
- Support `error`, `warn`, `info`, and `debug` levels in the logging model without exposing user-facing level configuration.
- Route frontend/client events that need persistence, including error boundary failures and critical user operation failures, through the frontend service boundary to the native logging service.
- Define project-wide logging development requirements so future native features and operation flows use the unified logging service.
- Provide Web/mock behavior that displays a mock log path and disables native open-directory actions.
- Identify and update existing nonconforming SDK/CLI logging paths during implementation.

## Capabilities

### New Capabilities
- `unified-log-management`: Defines the shared logging contract, log directory behavior, redaction, retention, archival, log levels, desktop persistence, Web/mock behavior, and development rules for future features.

### Modified Capabilities
- `app-settings`: Extend common settings with the configured log directory and read-only logging policy values needed by the settings service.
- `settings-center-ui`: Add Basic Settings log management controls and Web/mock disabled behavior.
- `native-runtime-architecture`: Require native commands and operation flows to write diagnostics and SDK/CLI operation logs through the unified logging service.
- `sdk-dependency-management`: Persist SDK operation logs through unified log management in addition to exposing operation logs to the SDK settings page.
- `agent-tool-registry`: Persist CLI package and detection operation logs through unified log management in addition to exposing card-level logs.

## Impact

- Affects both desktop runtime and Web/mock runtime.
- Frontend settings types, `SettingsService`, `tauri-settings-client.ts`, and `web-settings-client.ts` need new log management operations while preserving the service boundary.
- Frontend service contracts need a client log event path so desktop adapters can report persistent client-side errors to Rust and Web/mock adapters can use no-op or mock behavior.
- Basic Settings UI needs a log management section; React components must continue to avoid direct Tauri `invoke()` calls.
- Rust/Tauri needs a unified log service, log directory resolution, log directory opening command, redaction, retention, archival, and integration with SDK/CLI operation log writers.
- SQLite settings storage needs to persist the configured log directory while fixed policies can remain code-defined.
- Existing SDK/CLI log writing and task registry paths need review and migration to the unified logging contract.
- Tests should cover settings normalization, Web/mock disabled behavior, Rust redaction, path changes without migration, automatic archival/retention behavior, and SDK/CLI operation log persistence.
