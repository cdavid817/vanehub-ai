## 1. Service Contract

- [x] 1.1 Add usage statistics request/response types for supported time ranges and summary fields.
- [x] 1.2 Add `getUsageStatistics` to the Agent service interface.
- [x] 1.3 Implement Tauri adapter wiring to a declared native command.
- [x] 1.4 Implement Web/mock adapter aggregation from mock session messages.

## 2. Native Aggregation

- [x] 2.1 Add Rust usage statistics structs and supported range validation.
- [x] 2.2 Implement read-only SQLite aggregate query for message usage totals, counted assistant messages, and counted sessions.
- [x] 2.3 Register the native Tauri usage statistics command.
- [x] 2.4 Add focused Rust tests for all-time and range-filtered aggregation.

## 3. Settings UI

- [x] 3.1 Add Usage Statistics to the settings page registry before About.
- [x] 3.2 Build a localized Usage Statistics page using shared settings primitives, semantic tokens, and icon-backed range controls.
- [x] 3.3 Add zh-CN and en locale keys for navigation, search, labels, empty/error states, and accounting limitation notes.
- [x] 3.4 Keep the page compatible with both `futuristic` and `minimal` styles without page-specific theme branches.

## 4. Frontend Tests

- [x] 4.1 Add or update Web adapter/service tests for usage statistics aggregation.
- [x] 4.2 Add page rendering tests for loaded and empty usage statistics states.
- [x] 4.3 Keep i18n parity and visible text guardrail tests passing.

## 5. Verification

- [x] 5.1 Run `openspec validate "add-usage-statistics-settings" --strict`.
- [x] 5.2 Run `npm run test`.
- [x] 5.3 Run `npm run build`.
- [x] 5.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
