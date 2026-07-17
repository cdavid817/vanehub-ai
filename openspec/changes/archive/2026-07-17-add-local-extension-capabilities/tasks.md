## 1. Frontend Contracts and Mock Runtime

- [x] 1.1 Add strict TypeScript extension capability, framework, status, preview, test, and operation request types.
- [x] 1.2 Add the `ExtensionService` interface plus runtime-selected Tauri and deterministic Web/mock adapters.
- [x] 1.3 Add adapter and mock lifecycle tests covering stable catalog ids, desktop-only behavior, and state transitions.

## 2. Extension Capabilities Settings Experience

- [x] 2.1 Register the Extension Capabilities page after SDK Dependencies and add synchronized zh-CN/en navigation and search keys.
- [x] 2.2 Implement the service-backed page with summary, platform notice, search, capability cards, compatibility and requirement details, and lifecycle controls.
- [x] 2.3 Add installation/uninstall confirmations, operation polling, card-local logs, errors, and refresh behavior using shared settings primitives.
- [x] 2.4 Add component tests for filtering, Web/mock limitations, operation feedback, both locales, and semantic theme compatibility.

## 3. Native Extension Domain

- [x] 3.1 Add Rust extension models and the built-in PaddleOCR, faster-whisper, and sherpa-onnx allowlisted catalog.
- [x] 3.2 Add SQLite schema initialization and repository operations for selected framework, install, enablement, port, path, version, health, error, and timestamps.
- [x] 3.3 Implement Windows x64 and Python prerequisite detection plus non-mutating install previews and unsupported reasons.
- [x] 3.4 Implement guarded application-owned paths and backend-owned install/uninstall command plans with proxy propagation and unified command auditing.
- [x] 3.5 Implement async install, uninstall, enable, start, stop, health refresh, and self-test operations with per-framework mutation locking and existing task-registry integration.
- [x] 3.6 Add loopback port checks, owned child-process tracking, health transitions, and foreign-process protection.
- [x] 3.7 Register Tauri commands and application state without exposing executable or filesystem paths from the frontend.

## 4. Native Verification and Logging

- [x] 4.1 Add Rust tests for catalog stability, compatibility detection, repository persistence, path containment, lifecycle transitions, concurrency rejection, and foreign-port handling.
- [x] 4.2 Verify extension task output remains page-displayable while redacted diagnostic events use the unified native logger and no feature-local log file is created.
- [x] 4.3 Require package-version and framework-import verification before writing the installed marker or SQLite installed state, and test that verification failure leaves the framework uninstalled.

## 5. End-to-End Verification and Documentation

- [x] 5.1 Add or update Playwright coverage for settings navigation, localized extension rendering, search, install preview, operation feedback, and both registered themes using the Web/mock adapter.
- [x] 5.2 Run `npm run test`, `npm run build`, `cargo test`, `cargo check`, `cargo clippy`, and strict OpenSpec validation; fix regressions.
- [x] 5.3 Perform or document a Windows x64 manual smoke test for Python detection and framework install/self-test without making multi-gigabyte model downloads part of CI.
- [x] 5.4 Update `design.md` delivery progress, verified behavior, known limitations, and prioritized follow-up optimizations to match the implemented result.
- [x] 5.5 Reconcile the proposal, design, and local-extension specification around the first-version loopback management sidecar and deferred inference endpoints.
