## Context

Native logs are JSONL files in the configured directory. Every append currently validates the directory and scans it for expired files, while the active `vanehub.log` never becomes eligible for archival. The session log viewer reads the whole active file while it holds the application-wide `RegistryStore` mutex. Some native error paths also print raw messages to stderr, and main-window chat mutations do not consistently surface or report service failures.

The desktop implementation must retain its Rust-owned filesystem and SQLite responsibilities. React must report client failures through the existing settings service, the Tauri adapter must remain the only location that invokes the native client-log command, and the Web/mock adapter must remain a no-op for durable diagnostics.

## Goals / Non-Goals

**Goals:**
- Prevent sensitive values in assignment-style and JSON-style diagnostic text from reaching disk or native stderr.
- Persist native diagnostics in the configured active log directory once it is available.
- Eliminate per-entry retention scans and avoid holding the registry mutex during log-file reading, writing, or export preparation.
- Bound interactive session-log reads and provide localized feedback plus durable diagnostics for main-window chat operation failures.
- Restore reliable E2E navigation coverage for CLI management.

**Non-Goals:**
- Replace JSONL logs with SQLite, introduce a remote logging service, or add user-configurable retention and log levels.
- Change the public frontend service contract or make the Web/mock runtime write local files.
- Redesign settings-page routing or split bundles in this defect fix.

## Decisions

### Use serialized, scheduled log maintenance

The logging module will serialize writes with a module-local write guard. Before appending, it will check lightweight maintenance state for the target directory; rotation and retention scans run at most once per maintenance interval, rather than per entry. When the active file crosses its rotation boundary it is renamed to a timestamped `.log` file, leaving a fresh active `vanehub.log`; only rotated files become archival candidates after 30 days.

This keeps the active file eligible for retention and avoids concurrent rename/append races. A background logging queue was considered, but it would require application lifecycle flushing and introduce loss semantics that this repair does not need.

### Redact structured text before every output sink

Redaction will recognize case-insensitive sensitive keys with `=`, `:`, or whitespace separators, quoted JSON values, bearer credentials, and supported provider token prefixes. The same redacted value is used for JSONL persistence and any emergency diagnostic output. Normal native diagnostics will no longer print raw messages to stderr.

Regular expressions were considered for concise parsing, but the implementation will use the smallest dependable parsing helper that covers key/value and JSON forms without adding a new runtime dependency. Sensitive context keys continue to replace their complete values.

### Keep the native active log directory in process state

A Rust-owned active-log-directory resolver will initialize from persisted settings during setup and update after a successful `logDirectory` save. Native diagnostics use it after initialization; the application-data fallback is used only before settings can be read. This preserves a usable failure path for migration/startup failures without bypassing the unified logger.

### Release SQLite state before filesystem work and bound interactive reads

Session-log commands will acquire the session and configured directory under `RegistryStore`, release the mutex, then read or export files. Interactive list queries will inspect newest log data first and enforce a fixed byte/read bound before returning a normal page or a truncated result. Export preparation follows the same lock-release rule.

Adding a log index was considered but deferred: daily rotation plus bounded newest-first reads resolves the current UI-blocking behavior without a schema migration or dual-write consistency burden.

### Register the existing workspace command wrappers

The `session_tabs`, `shell`, and `commands` modules already contain the Rust implementations and Tauri command wrappers, but they are not declared or registered from the crate root. The desktop setup will manage `ShellManager`, declare the modules, and register the existing wrappers in `generate_handler!`. This restores the service contract that the Tauri session-workspace adapter already invokes without moving native behavior into React.

### Report main-chat client failures through existing boundaries

The main layout model will attach `onError` handlers to send and stop mutations and will pass configuration-save failures through a callback. Each failure shows a translated notification and sends a `critical-operation-failure` event through `settingsService`; the Web/mock adapter continues its existing no-op persistence behavior. Failed sends restore the user draft so the prompt is not lost.

## Risks / Trade-offs

- [A capped query may not expose very old entries in a single interaction] → Preserve newest-first pagination semantics, report truncation, and keep full filtered export available.
- [Previously unregistered command wrappers can expose latent implementation errors] → Compile the modules and add focused command-registration coverage before the complete Tauri verification run.
- [The fallback directory is still used before settings initialize] → Restrict it to startup failures and initialize/update the active path before normal commands run.
- [Broader redaction can conceal benign diagnostic text] → Match only the established sensitive-key and credential patterns and add positive/negative test cases.
- [Mutation error notifications can repeat during retries] → Report only terminal mutation failures and retain React Query's existing retry behavior.

## Migration Plan

1. Deploy the new writer against the existing `vanehub.log`; the first maintenance pass rotates it when it crosses the boundary.
2. Continue reading the active file and rotated active-directory `.log` files so historical session logs remain available.
3. If logging initialization fails, use the existing app-data fallback without raw stderr output; no database or API migration is required.
4. Rollback consists of reverting the code. Existing JSONL files remain readable because the entry schema is unchanged.

## Open Questions

None.
