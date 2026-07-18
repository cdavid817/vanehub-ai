## Why

The unified logging implementation can persist secrets formatted as JSON or separated key/value text, emits unredacted native diagnostics to stderr, and performs synchronous directory scans and full-file queries while holding the shared registry lock. Main-window chat mutations also discard some service failures without user feedback or durable diagnostics, and two end-to-end tests cannot pass because their locators are ambiguous.

## What Changes

- Strengthen native log redaction for structured and whitespace-separated sensitive values, and ensure native diagnostics use the configured active log directory without writing raw diagnostic messages to stderr.
- Rotate the active log file and run retention archival outside the per-entry write path so operation output does not repeatedly scan the log directory.
- Read session log pages and exports without holding the registry mutex during filesystem work, and bound the data read for paginated views.
- Register the existing native session-workspace and shell command modules so the Tauri adapters can reach the repaired log viewer and related workspace capabilities.
- Terminate managed session shells before archive or deletion, without holding the registry mutex while the PTY cleanup may block.
- Surface main-window chat send, stop, and configuration-persistence failures with localized feedback and durable client log events through the existing service boundary.
- Make CLI-management E2E navigation locators target the settings navigation buttons unambiguously.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `unified-log-management`: Strengthen redaction, configured-directory handling, active-log rotation, and retention behavior for native diagnostics and operation logs.
- `session-log-viewer`: Require bounded session-log retrieval that does not block the shared native registry during filesystem scanning or export preparation.
- `session-workspace-tabs`: Require declared desktop commands for the session-workspace service contract.
- `chat-experience`: Require visible and durably reported main-window chat operation failures while retaining the frontend service boundary.

## Impact

- Desktop runtime: `src-tauri/src/logging.rs`, `src-tauri/src/lib.rs`, and `src-tauri/src/session_tabs.rs`.
- Frontend: main layout chat model, chat configuration hook, localized resources, and focused tests.
- Browser/Web mock: preserves the existing no-op durable client log behavior and matching service contracts.
- Validation: Rust/unit tests, frontend tests, affected Playwright coverage, build, Clippy, and strict OpenSpec validation.
