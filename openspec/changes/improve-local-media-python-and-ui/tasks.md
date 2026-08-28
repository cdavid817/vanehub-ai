## 1. Native Python discovery domain and infrastructure

- [x] 1.1 Add local-media domain types for discovery availability, candidate source, normalized Python version, compatibility state, and stable reason codes, with serialization-independent unit tests.
- [x] 1.2 Define the application discovery port and service use case, including configured-profile paths as bounded seeds and a shared Python version compatibility policy.
- [x] 1.3 Implement direct, shell-free interpreter probing with isolated arguments, timeout, output-size and candidate-count limits, structured identity parsing, and safe partial-failure handling.
- [x] 1.4 Implement platform seed collection for PATH-resolvable Python commands and bounded Windows launcher enumeration without recursive filesystem scanning.
- [x] 1.5 Normalize and deduplicate resolved paths with platform-aware comparison, then order candidates deterministically by compatibility, version, and path.
- [x] 1.6 Add Rust tests for compatible and incompatible versions, broken aliases, malformed output, timeout and output limits, duplicate identities, stable ordering, configured paths, and empty results using process doubles or fixture executables.

## 2. Native API, bootstrap, and safe diagnostics

- [x] 2.1 Wire the discovery infrastructure adapter into local-media bootstrap and expose it through `LocalMediaApi` without changing profile persistence or worker fallback behavior.
- [x] 2.2 Add a thin Tauri discovery command and DTO mapper, register the command, and ensure raw process output and operating-system errors never cross the command boundary.
- [x] 2.3 Emit only unified-log allowlisted discovery outcome, count, source category, duration bucket, and stable reason-code fields, with tests proving paths, environment values, and raw output are excluded.
- [x] 2.4 Add native command and bootstrap integration tests for successful, empty, partially failed, and unavailable discovery results without starting a local-media worker.

## 3. Frontend service contract and adapters

- [x] 3.1 Add strict TypeScript discovery result and candidate types plus `discoverPythonEnvironments()` to `LocalMediaService`.
- [x] 3.2 Implement and test Tauri adapter invocation and DTO normalization for valid and malformed discovery payloads.
- [x] 3.3 Implement and test the production Web adapter's truthful `native_unavailable` result with no fabricated candidates, devices, or readiness.
- [x] 3.4 Update deterministic E2E/test service doubles and adapter conformance tests so every local-media adapter implements the additive discovery contract.

## 4. Settings state and Python environment selection

- [x] 4.1 Extend `useLocalMediaSettings` with an independent discovery query, manual refresh, retryable localized failure state, and mounted-page caching that never persists inventory data.
- [x] 4.2 Add draft-only actions that assign a compatible candidate to one selected engine or explicitly selected engines while preserving every model, device, and tuning field.
- [x] 4.3 Preserve configured paths absent from discovery as editable “not detected” values and retain the existing service-backed custom executable picker.
- [x] 4.4 Add hook tests proving discovery failures do not block profile editing, refresh never overwrites drafts, selection does not save or start workers, and only successful profile Save activates a selected path.

## 5. Guided Local Media UI

- [x] 5.1 Build a compact setup overview for master enablement, detected Python availability, per-engine completeness/readiness, saved state, and truthful next-step guidance.
- [x] 5.2 Build a shared Python environment panel with refresh, version/path/source details, compatible and incompatible states, per-engine assignment, explicit apply-to-all, not-detected values, and custom-path fallback.
- [x] 5.3 Refactor OCR, STT, and TTS cards so required setup remains visible, summaries expose safe readiness/configuration metadata, and optional tuning fields use accessible progressive disclosure.
- [x] 5.4 Automatically open the disclosure containing a blocking validation or readiness issue and associate and focus its accessible error target without discarding draft changes.
- [x] 5.5 Add a responsive sticky Save/Discard action area with dirty, saving, saved, conflict, and failed feedback, reserved content space, wrapping controls, and no narrow-window clipping.
- [x] 5.6 Add matching zh-CN and en-US locale keys for all discovery, compatibility, setup, disclosure, status, and error states.
- [x] 5.7 Keep all new and refactored React production files within the 300-line limit and verify components depend only on the service boundary, Tailwind classes, and existing UI primitives.

## 6. UI and workflow verification

- [x] 6.1 Add component tests for compatible selection, explicit multi-engine application, incompatible candidates, missing saved candidates, custom paths, retry, draft/save semantics, and Web mode.
- [x] 6.2 Add accessibility and responsive tests for disclosure semantics, keyboard order, error focus, non-color status meaning, sticky-action clearance, and narrow desktop widths in both supported locales.
- [x] 6.3 Add Playwright coverage for the Local Media setup path from Python discovery through required fields, Save, and readiness probe using deterministic fixtures without model inference.
- [x] 6.4 Add or extend desktop fixture coverage for the real discovery IPC path and record Windows, macOS, and Linux outcomes independently as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`.

## 7. Required validation and specification sync

- [x] 7.1 Run `npm run lint:ci`, `npm run test`, `npm run build`, and `npm run test:coverage`; fix all failures without lowering coverage or adding lint exemptions.
- [x] 7.2 Run `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, and `npm run architecture:check`; fix all failures.
- [x] 7.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`; fix all failures.
- [x] 7.4 Run `npx playwright test`, `npm run desktop:unit:test`, and the applicable `npm run test:desktop` layers; retain platform-specific evidence and report any environment-limited layer accurately.
- [x] 7.5 Run `openspec validate improve-local-media-python-and-ui --strict` and `openspec validate --specs --strict`, then update task checkboxes and implementation verification evidence for review.

## Implementation verification evidence

- Frontend validation: `lint:ci`, unit tests, production build, coverage, coverage policy,
  version checks, contracts, and architecture checks passed. Vitest completed 1,610 tests across
  315 files. The deterministic local-media Playwright suite passed 26/26, and the general
  Playwright suite passed 166/166 after scoping an ambiguous `--config` locator to its accessible
  token-list label.
- Native validation: formatting, workspace check, Clippy with warnings denied, panic-policy scan,
  and workspace tests passed. The main native library completed 4,083 tests with 13 ignored;
  the architecture, MCP fixture/relay, and permission-hook suites also passed.
- Linux desktop discovery IPC: `PASSED` in desktop smoke; all 33 smoke spec files passed. The
  discovery command returned a bounded typed inventory and left the local-media profile unchanged.
  Evidence: `test-results/desktop/2026-08-25T17-57-12-792Z-077f9dfc`.
- Linux deterministic inference fixture: `PASSED`; all 10 scenarios passed after limiting the
  measured sherpa-onnx ASCII-path preflight to Windows, so POSIX Unicode paths reach the real
  canary instead of being rejected categorically. Python bridge tests passed 91/91. Evidence:
  `test-results/desktop/2026-08-26T02-12-14-063Z-bdca3cb0`.
- Windows desktop discovery IPC: `NOT RUN` locally. macOS desktop discovery IPC: `NOT RUN`
  locally. Their CI runner outcomes must be recorded independently before release sign-off.
