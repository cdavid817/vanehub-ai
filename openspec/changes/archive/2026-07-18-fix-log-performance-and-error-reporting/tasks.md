## 1. Unified Native Logging

- [x] 1.1 Strengthen message redaction for assignment, JSON, bearer, and provider-token formats and cover positive and negative cases with Rust tests.
- [x] 1.2 Add configured active-directory state for native diagnostics, remove raw stderr output, and update the state after a successful log-directory save.
- [x] 1.3 Rotate the active JSONL log and schedule retention maintenance outside the per-entry append path with concurrency-safe writes.

## 2. Session Log Performance

- [x] 2.1 Resolve session access and the log directory under `RegistryStore` before releasing it for session-log page queries and export preparation.
- [x] 2.2 Read active and rotated session log files newest-first within a fixed interactive retrieval bound and retain pagination/truncation behavior.
- [x] 2.3 Add Rust tests for rotation, retention selection, redaction persistence, bounded lookup, and lock-independent query setup.
- [x] 2.4 Register the existing session-workspace and shell command wrappers plus `ShellManager` in the desktop runtime.
- [x] 2.5 Terminate managed session shells before archive or deletion without holding the registry mutex during PTY cleanup.

## 3. Frontend Error Reporting

- [x] 3.1 Add localized main-chat failure notifications and durable client-log reporting for send and stop mutation failures without bypassing the service boundary.
- [x] 3.2 Report session chat-configuration persistence failures, preserve the Web/mock no-op log behavior, and add focused frontend tests.

## 4. Regression Coverage

- [x] 4.1 Replace ambiguous CLI-management E2E text locators with semantic settings-navigation button locators.
- [x] 4.2 Run `npm run lint`, `npm run test`, `npm run build`, `npx playwright test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml`, and `openspec validate fix-log-performance-and-error-reporting --strict`.
