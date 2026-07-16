## Context

VaneHub already has several log-like paths: SDK operation logs, CLI package/detection logs, task registry logs, and native diagnostic logging requirements. These logs are currently tied to feature-specific surfaces and do not share a durable file layout, redaction policy, retention behavior, or development contract.

The application must keep the existing runtime boundary: React components depend on service interfaces, Tauri adapters own `invoke()` calls, Web/mock adapters provide browser behavior, and Rust owns local filesystem, SQLite, process, and shell-open operations.

## Goals / Non-Goals

**Goals:**
- Define one native logging service for diagnostics and operation logs.
- Persist SDK and CLI operation logs to the same configured log directory while preserving existing UI-visible operation logs.
- Add Basic Settings controls for displaying the log directory, changing it, and opening the directory in the desktop runtime.
- Persist frontend/client log events for error boundaries and critical user operation failures through the existing service boundary.
- Apply built-in redaction before any line is written to disk.
- Automatically archive logs older than the fixed 30-day retention window.
- Establish a project-wide logging development contract for future features.
- Keep Web/mock behavior explicit: mock path is visible and native open-directory actions are disabled.

**Non-Goals:**
- No user-facing log level configuration in the first version.
- No automatic migration of existing log files after log directory changes.
- No user-defined redaction rules or regular expressions in the first version.
- No direct log file viewer in the UI; the desktop action opens the log directory.
- No requirement to compress archives unless implementation can do so without adding unnecessary complexity.

## Decisions

### Decision: Introduce a Rust-owned unified log service

The native runtime will own a `LogService` or equivalent module that resolves the active log directory, writes redacted log entries, applies retention, and archives expired logs.

Alternatives considered:
- Keep SDK/CLI logs feature-local: rejected because it does not create a reusable standard for future features.
- Write logs from frontend adapters: rejected because filesystem access and sensitive redaction must stay in the Rust/native layer.

### Decision: Store only user-selected log directory in settings

The persisted setting will represent the configured log directory. Fixed first-version policies such as 30-day retention, built-in redaction enabled, automatic archival, and supported levels can be returned as log metadata rather than persisted as editable settings.

Alternatives considered:
- Persist every policy value now: rejected because the user explicitly does not need retention or level configuration in the first version.
- Hard-code the directory only: rejected because users need to modify the save path.

### Decision: Path changes affect only new logs

When the user changes the log directory, the service will validate/create the new directory and write future logs there. Existing files remain where they were.

Alternatives considered:
- Migrate old logs automatically: rejected to avoid long-running file operations, accidental data movement, and unclear rollback behavior.
- Delete old logs after path change: rejected because it would be destructive and surprising.

### Decision: Built-in redaction before persistence

The logging service will redact common sensitive material before writing logs: password-like key/value pairs, token/secret/api-key fields, bearer tokens, and common provider key formats where practical. Redaction applies to diagnostics and operation logs written through the unified service.

Alternatives considered:
- User-configurable redaction rules: deferred because it adds validation, testing, and misuse risk.
- UI-only redaction: rejected because raw secrets would still exist on disk.

### Decision: Automatic archival for expired logs

The service will automatically process logs older than 30 days into an archive location under the active log directory, such as `archive/`. The exact file organization can be chosen during implementation, but archived logs must remain under the configured log directory and must not block startup or UI rendering.

Alternatives considered:
- Manual archive button: rejected because the requirement is automatic archival.
- Immediate deletion without archive: rejected because the requirement includes archival capability.

### Decision: Web/mock remains non-native

The Web/mock settings adapter will return a mock log path and policy metadata, and the UI will disable native open-directory actions with explanatory disabled state. It will not pretend to access the local filesystem.

Alternatives considered:
- Simulate success for open-directory: rejected because it hides runtime differences users need to understand.

### Decision: Frontend log events use service-mediated native persistence

Frontend code that detects a persistent-worthy client event, such as an error boundary failure or critical user operation failure, will report that event through a frontend service interface. In desktop runtime, the Tauri adapter will call a declared native command and Rust will redact and write the event through unified logging. In Web/mock runtime, the adapter will use no-op or mock behavior and will not write local files.

Alternatives considered:
- Write frontend logs directly to files: rejected because React must not bypass runtime adapters or native filesystem controls.
- Rely only on browser console output: rejected because desktop support diagnostics need durable logs.

## Risks / Trade-offs

- [Risk] Redaction misses an unknown secret format -> Mitigation: centralize redaction, cover common tokens, and add focused tests so new patterns can be extended safely.
- [Risk] Log writes or archival block native operations -> Mitigation: keep file IO bounded, run archival opportunistically, and avoid blocking Tauri command responses on large cleanup work.
- [Risk] Existing feature logs continue using old paths -> Mitigation: include an implementation audit task for SDK/CLI/task-registry log calls and require future features to use the unified logging service.
- [Risk] Changing log directory to an invalid or restricted path fails -> Mitigation: validate and create the directory before saving it, then return a user-displayable error without changing the active setting.
- [Risk] Log directory opening is platform-specific -> Mitigation: keep it behind a Rust command and expose only service-level behavior to React.
- [Risk] Frontend log events become noisy or include sensitive fields -> Mitigation: restrict persistent client events to error boundaries and critical operation failures, then apply Rust-side redaction before writing.

## Migration Plan

1. Add the log directory setting with a default under the VaneHub app-owned user data directory.
2. Implement the native logging service and wire diagnostics plus SDK/CLI operation logs through it.
3. Add settings service methods and Web/mock behavior.
4. Add frontend client log event reporting through service adapters.
5. Add the Basic Settings log management UI.
6. Audit existing SDK/CLI log paths and replace nonconforming persistence with unified logging.
7. Validate with OpenSpec, frontend tests/build, and Rust tests/checks.

Rollback is additive: if needed, disable new UI controls and stop writing operation logs through the unified service while keeping existing in-memory/UI operation logs.

## Open Questions

- Should archived logs be plain rotated files under `archive/`, or compressed if a suitable dependency already exists?
- Should diagnostic logs and operation logs share one file with typed entries, or use separate files under one directory such as `diagnostic.log` and `operations.log`?
