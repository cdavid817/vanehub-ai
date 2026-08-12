## 1. Contract and visual foundations

- [x] 1.1 Add regression coverage for the workspace bottom divider and dedicated OnePiece Agent icon.
- [x] 1.2 Implement the semantic bottom divider and reusable OnePiece vector icon across shared Agent identity surfaces.
- [x] 1.3 Add native IM serialization/deserialization tests for every stable connector id and fix DingTalk/WeCom wire names.

## 2. Scheduled Tasks and CLI management

- [x] 2.1 Add frontend and native tests proving OnePiece is selectable, persisted, and executed in API mode while unsupported API Agents remain rejected.
- [x] 2.2 Implement OnePiece Scheduled Task eligibility and execution-mode routing in desktop and Web/mock flows.
- [x] 2.3 Replace the duplicate installed/missing summary cards with one installed-coverage card and update component/E2E coverage.

## 3. CLI parameter audit

- [x] 3.1 Record audited managed parameter expectations for all five CLIs using current official documentation and installed CLI help output.
- [x] 3.2 Correct frontend/native parameter metadata, known values, scopes, previews, and localized descriptions while excluding policy-governed controls.
- [x] 3.3 Add parity tests that fail when the frontend and native editable catalogs diverge.

## 4. Gemini CLI Agent Configuration

- [x] 4.1 Extend TypeScript and Rust profile payload contracts, validation, capability predicates, presets, and serialization for `gemini-cli`.
- [x] 4.2 Implement native Gemini configuration inspection, import/discovery, managed projection, credential isolation, drift handling, atomic apply, and tests.
- [x] 4.3 Implement Web/mock Gemini profile lifecycle parity and adapter contract tests.
- [x] 4.4 Add Gemini CLI navigation, profile fields, summaries, localization in all supported locales, and component/E2E tests.

## 5. Settings navigation

- [x] 5.1 Reorder existing Settings destinations to the specified workflow-oriented sequence without changing ids or lazy-loading behavior.
- [x] 5.2 Update navigation tests and E2E expectations for the new order and deep-link stability.
- [x] 5.3 Revise the Settings order so recurring Agent behavior, MCP, Skills, and personalization precede one-time CLI installation and external integrations.
- [x] 5.4 Update unit and E2E order assertions for the revised frequency-oriented sequence.

## 6. Verification

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 6.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.3 Run `npx playwright test`, `openspec validate polish-settings-and-agent-integrations --strict`, and `openspec validate --specs --strict`.
