## 1. Test-Build Boundary

- [x] 1.1 Add pinned WebdriverIO runner, Tauri service, assertion, and reporting development dependencies with npm and commit the updated lockfile.
- [x] 1.2 Add optional Rust automation plugin dependencies behind a dedicated `desktop-e2e` Cargo feature.
- [x] 1.3 Add a dedicated Tauri desktop-test configuration and least-privilege capabilities that enable automation only when the `desktop-e2e` feature is selected.
- [x] 1.4 Conditionally register the automation plugins in native bootstrap without changing normal production bootstrap behavior.
- [x] 1.5 Add automated configuration guards proving normal production/package commands omit the test feature, automation plugins, global API exposure, and automation capabilities.

## 2. Orchestrator Foundations

- [x] 2.1 Add a unit-tested host detector that maps supported operating systems and architectures to the repository's native target metadata and returns `BLOCKED` for unsupported hosts.
- [x] 2.2 Add a unit-tested artifact resolver that reads Tauri/Cargo/build metadata and returns the absolute executable path, platform, architecture, profile, and test-feature state.
- [x] 2.3 Make artifact resolution bind to the current build invocation and fail with inspected-location diagnostics when the result is missing, stale, or ambiguous.
- [x] 2.4 Add a run-context helper that creates a unique temporary root, absolute `VANEHUB_APP_DATA_DIR`, fixture directory, result directory, and `VANEHUB_TEST_RUN_ID`.
- [x] 2.5 Add safety validation that blocks launch when the test data path escapes the run root, is relative, or aliases the normal application-data location.
- [x] 2.6 Add unit tests for supported platform mappings, artifact naming/ambiguity, isolated paths, unsafe paths, and result-state exit codes.

## 3. Desktop Driver and Readiness

- [x] 3.1 Add WebdriverIO configuration for a single native application instance using the embedded Tauri driver provider and bounded startup, command, and shutdown deadlines.
- [x] 3.2 Add a stable root readiness marker after successful React bootstrap without importing Tauri APIs into a React component.
- [x] 3.3 Add or confirm stable accessibility selectors for the minimal smoke navigation and introduce `data-testid` only where no semantic selector is reliable.
- [x] 3.4 Add a desktop harness helper that executes the existing read-only `get_settings` command against the running Tauri application without mocking and validates its response shape.
- [x] 3.5 Add regression tests confirming Web/mock startup and the existing Playwright browser suite remain independent from desktop readiness instrumentation.

## 4. Smoke, Lifecycle, and Evidence

- [x] 4.1 Add the native desktop smoke specification covering process launch, window and React readiness, real `get_settings` IPC, one stable navigation interaction, and fatal-error monitoring.
- [x] 4.2 Add process ownership tracking for the spawned application, driver, and attributable descendants without process-name-wide termination.
- [x] 4.3 Add graceful shutdown followed by bounded owned-process cleanup and assert that no process owned by the run remains.
- [x] 4.4 Add lifecycle tests proving cleanup leaves an unrelated VaneHub AI process untouched and refuses forced cleanup when ownership is uncertain.
- [x] 4.5 Add failure evidence collection for the summary, assertion, screenshot when available, frontend/driver diagnostics, and process state under `test-results/desktop/<run-id>/`.
- [x] 4.6 Copy native diagnostics only from the run's isolated unified log directory after existing redaction and record unavailable evidence without masking the original failure.
- [x] 4.7 Add tests for evidence indexing, bounded diagnostic output, redacted-log collection, partial collection failure, and successful-run temporary-data cleanup.

## 5. Commands and Result Contract

- [x] 5.1 Add `npm run test:desktop:build` to build the current-platform unbundled native artifact with the dedicated desktop test feature/configuration.
- [x] 5.2 Add `npm run test:desktop:smoke` to verify an explicitly resolved desktop test artifact in an isolated run.
- [x] 5.3 Add `npm run test:desktop` to compose desktop build and smoke while preserving the first failing or blocked exit status.
- [x] 5.4 Add `npm run test:verify` as a thin orchestrator over the repository's existing mandatory validation commands and applicable desktop verification without duplicating their implementations.
- [x] 5.5 Emit a machine-readable and human-readable result using only `PASSED`, `FAILED`, `BLOCKED`, `NOT RUN`, or reason-bearing `NOT REQUIRED`, with a non-zero exit code for failed or blocked requested layers.
- [x] 5.6 Document that Playwright Web/mock E2E and native desktop smoke are distinct verification layers and that local results claim only the current platform.

## 6. Cross-Platform CI

- [x] 6.1 Add a fail-fast-disabled Windows, macOS, and Linux desktop smoke matrix that installs pinned Node/Rust dependencies and builds each artifact on its native runner.
- [x] 6.2 Install the required Linux WebView and virtual-display prerequisites and run the same `npm run test:desktop` contract used on developer machines.
- [x] 6.3 Upload platform-labelled run summaries, screenshots, driver diagnostics, process state, and redacted native logs when a matrix job fails or is blocked.
- [x] 6.4 Ensure successful jobs do not upload temporary databases or application-data directories and that one platform failure does not misreport or cancel other platform evidence.
- [x] 6.5 Update CI documentation and permanent agent guidance only after the desktop smoke command and all three matrix variants have demonstrated stable behavior.

## 7. Implementation Verification

- [x] 7.1 Run orchestrator, resolver, isolation, lifecycle, evidence, and production-boundary unit tests.
- [x] 7.2 Run `npm run test:desktop` on the current Windows host, record the actual executable path and evidence directory, and verify real launch, IPC, interaction, and clean shutdown pass.
- [x] 7.3 Run `npx playwright test` to verify the existing browser Web/mock E2E layer still passes independently.
- [x] 7.4 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 7.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.6 Run `openspec validate add-cross-platform-desktop-automation --strict` and `openspec validate --specs --strict`.
- [x] 7.7 Review CI results and report Windows, macOS, and Linux independently as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` without inferring unexecuted outcomes.

  Verification record: GitHub Actions run `31863928804` reported Windows `PASSED`, macOS `PASSED`, and Linux `PASSED` independently.
