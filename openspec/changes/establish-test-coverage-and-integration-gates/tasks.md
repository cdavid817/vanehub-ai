## 1. Coverage Foundation

- [x] 1.1 Add version-compatible Vitest V8 coverage dependencies, scripts, explicit production-source include/exclude rules, and text, JSON/LCOV, and HTML reporters.
- [x] 1.2 Add a repository-owned coverage policy that defines normalized frontend/native baselines and the Agent, MCP, and SQLite critical Rust source groups, rejecting empty path matches.
- [x] 1.3 Add a deterministic policy checker that reads frontend and `cargo-llvm-cov` machine reports, enforces wider non-regression baselines, and enforces 80% line coverage for each critical Rust group on Windows and Linux paths.
- [x] 1.4 Run initial instrumented frontend and native suites, review exclusions, and commit truthful baseline values without excluding supported Web/mock or native production behavior.

## 2. Frontend Interaction Testing

- [x] 2.1 Add React/DOM Testing Library, `user-event`, a Vitest DOM environment, cleanup wiring, and a shared render helper for QueryClient, i18n, theme, and deterministic Agent service doubles.
- [x] 2.2 Add component interaction tests for dragging an eligible Session to a valid category and for invalid/no-op drops without direct Tauri access.
- [x] 2.3 Add or extend one Playwright browser scenario for real Session category drag behavior and its accessible non-drag alternative.
- [x] 2.4 Add Prompt Hook interaction tests for editing, validation, preview, successful save/query refresh, service failure with retained input, and immutable built-in hooks.
- [x] 2.5 Confirm the frontend coverage report includes unimported `src/**/*.{ts,tsx}` production files and keeps Web/mock adapter code in scope.

## 3. Critical Rust Behavioral Coverage

- [x] 3.1 Add missing Agent startup and terminal-control tests for launch preparation, open/attach, stop, startup failure compensation, idle cleanup, resource release, and lifecycle diagnostics.
- [x] 3.2 Add missing MCP routing tests for supported routing, byte-transparent forwarding, bounded timeout, child-process failure, protocol failure, and redacted bounded diagnostics.
- [x] 3.3 Add SQLite transaction tests for successful atomic commits and deterministic later-write failures that prove rollback and preservation of pre-existing data.
- [x] 3.4 Add or extend database pool and migration tests for contention/back-pressure, connection recovery, empty schema, and supported legacy upgrade behavior.
- [x] 3.5 Generate native coverage and close uncovered critical-path gaps until every configured Agent, MCP, and SQLite group reaches at least 80% line coverage.

## 4. Native Lifecycle Integration

- [x] 4.1 Add a crate-internal `#[cfg(test)]` integration harness that assembles real temporary `NativeDatabase` persistence and published sessions/agent-runtime APIs with deterministic process, terminal, clock, operation, event, and logging doubles.
- [x] 4.2 Add the successful lifecycle integration scenario: create Session, open Agent Terminal, assert running and persisted operation state, stop Terminal, delete Session, and assert database plus terminal-registry cleanup.
- [x] 4.3 Add startup-failure integration coverage for command-safe errors, failed lifecycle persistence, redacted diagnostic association, and release of reserved resources.
- [x] 4.4 Add repeated stop/cleanup integration coverage that proves documented idempotence and does not recreate live terminal state.
- [x] 4.5 Verify the native integration suite runs without provider CLI installations, network access, credentials, persistent user data, or an interactive Tauri window.

## 5. CI and Code-Quality Gates

- [x] 5.1 Add a frontend coverage step/job that runs Vitest once with coverage, enforces the committed policy, writes a concise workflow summary, and uploads bounded reports even when a threshold fails.
- [x] 5.2 Add a parallel native coverage job that installs a pinned `cargo-llvm-cov` through an immutable supply-chain-compliant reference, enforces the coverage policy, and uploads bounded reports.
- [x] 5.3 Update Rust CI to run Clippy across configured targets with `-D warnings`, resolving warnings without blanket suppressions.
- [x] 5.4 Update frontend lint CI to reject warnings after resolving existing warnings or correcting their rules without weakening intended protections.
- [x] 5.5 Preserve independent frontend, Rust, contract, OpenSpec, native-platform, and Playwright diagnostics, least-privilege permissions, immutable Action references, and current-run cancellation.

## 6. Verification and Handoff

- [x] 6.1 Run `npm run lint`, `npm run test`, the frontend coverage command, `npm run contracts:check`, `npm run build`, and `npx playwright test`.
- [x] 6.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, strict all-target Clippy, `cargo test --manifest-path src-tauri/Cargo.toml`, and the native coverage command.
- [x] 6.3 Verify coverage-policy negative fixtures fail for a sub-80 critical group, a wider baseline regression, an empty path group, and malformed/incomplete reports.
- [x] 6.4 Run `openspec validate establish-test-coverage-and-integration-gates --strict` and `openspec validate --specs --strict`.
- [x] 6.5 Record final frontend/native coverage totals, critical-group results, test counts, CI artifact names, known exclusions, and intentionally deferred follow-ups in this change before archive.
