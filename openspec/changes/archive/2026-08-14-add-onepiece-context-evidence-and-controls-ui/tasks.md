## 1. Shared setting contract

- [x] 1.1 Add the default-enabled automatic compaction preference to frontend settings types, normalization, validation, reset behavior, and unit tests.
- [x] 1.2 Add the preference to native desktop settings DTOs, domain mappings, persistence defaults, and round-trip tests.
- [x] 1.3 Propagate the captured preference through the native OnePiece personalization/settings gateway without changing active-generation snapshots.

## 2. User control UI

- [x] 2.1 Add a localized, accessible automatic compaction control to the OnePiece parameter panel using the settings provider boundary.
- [x] 2.2 Add component tests for enabled, disabled, saving, failure, and restored setting states.
- [x] 2.3 Add all supported locale strings and verify semantic theme styling without inline styles or direct Tauri invocation.

## 3. Evidence projection

- [x] 3.1 Define an allowlisted native compaction evidence model with before/after character and optional token metrics, qualities, savings, trigger source, path, and policy version.
- [x] 3.2 Centralize successful optimizer and compatibility-path evidence emission into exactly one persisted rich card per compaction.
- [x] 3.3 Combine the persisted user preference with request suppression and add native tests for default, disabled, and generation-snapshot behavior.
- [x] 3.4 Add native tests proving metric correctness, unavailable-token semantics, path provenance, and absence of prompt/tool/secret content.

## 4. Web/mock parity

- [x] 4.1 Apply the persisted automatic compaction preference to Web/mock compaction simulation.
- [x] 4.2 Emit a contract-compatible character-only evidence card with explicit unavailable token evidence.
- [x] 4.3 Add Web/mock tests for enabled/disabled behavior, card contract, persistence shape, and sensitive-content exclusion.

## 5. Verification and completion

- [x] 5.1 Run `openspec validate add-onepiece-context-evidence-and-controls-ui --strict` and `openspec validate --specs --strict`.
- [x] 5.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, `npm run docs:check`, and `npm run build`.
- [x] 5.3 Run `npx playwright test` for the UI behavior change.
- [x] 5.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.5 Run OpenSpec implementation verification, resolve all findings, and record the final validation evidence before archive.

### Final verification evidence

- Completeness: 18/18 tasks, 7/7 requirements, and 17/17 scenarios are implemented.
- Correctness: native, frontend, Web/mock, component, and E2E tests cover the persisted preference, generation snapshot, optimizer and compatibility evidence, unavailable-token semantics, rich-block persistence, and sensitive-content exclusion.
- Coherence: the UI uses the settings service boundary, desktop and Web adapters expose compatible contracts, and evidence projection uses an allowlisted content-free model.
- Validation: OpenSpec strict validation, frontend lint/test/coverage/build checks, full Playwright, and Rust fmt/clippy/test/check all passed.
- Findings: no critical issues, warnings, or archive-blocking suggestions remain.
