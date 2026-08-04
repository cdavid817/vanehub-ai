## 1. Native Recovery Policy

- [x] 1.1 Add a platform-neutral WebView failure-kind and recovery-policy state machine with a bounded renderer-unresponsive threshold.
- [x] 1.2 Add unit tests for renderer exit, browser exit, auto-recoverable failures, repeated unresponsiveness, expiry, and recovery reset.

## 2. Windows WebView2 Integration

- [x] 2.1 Pin the Windows `webview2-com` dependency to the version used by the current Tauri runtime.
- [x] 2.2 Register a main-WebView `ProcessFailed` observer during Tauri setup and translate WebView2 failure kinds into the recovery policy.
- [x] 2.3 Write redacted unified diagnostics for every failure and execute reload, reload-fallback restart, or browser-process restart actions.

## 3. Frontend Regression Coverage

- [x] 3.1 Add regression coverage proving retained session-page switching keeps the active agent terminal mounted and refits it on return.
- [x] 3.2 Confirm Web/mock runtime requires no desktop API or adapter change.
- [x] 3.3 Handle rejected main/floating surface bootstrap promises and render a visible retry surface without calling Tauri APIs directly.
- [x] 3.4 Add localized bootstrap-recovery copy and regression tests for visible fallback, retry, and best-effort diagnostics.

## 4. Verification

- [x] 4.1 Run `npm run lint`, `npm run test`, and `npm run build` after the frontend bootstrap guard is added.
- [x] 4.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 4.3 Run `openspec validate recover-webview-white-screen --strict` and `openspec validate --specs --strict`.
- [x] 4.4 Re-exercise the Tauri dev WebView through repeated session-page and application minimize/restore cycles after both recovery layers are present and record the result. Result (2026-08-02): 27 cycles across 9 retained tabs completed with a minimum root height of 820 px and no empty DOM, tab mismatch, console error, page error, or request failure. An injected `App` module-load failure displayed the localized retry surface, and retry restored the workspace.
