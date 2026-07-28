## 1. Hydration Regression Coverage

- [x] 1.1 Add a focused SettingsProvider test that defers the settings response and verifies children first render only after configured font size, theme, and language are applied.
- [x] 1.2 Add failure-path coverage proving shared defaults and the load error are available when initial settings hydration fails.
- [x] 1.3 Update the Basic Configuration rendering test to exercise the client-side hydrated provider lifecycle.

## 2. Provider Hydration Boundary

- [x] 2.1 Make initial settings application awaitable and complete it before clearing the provider loading state.
- [x] 2.2 Gate settings-dependent children during initial hydration while preserving default fallback, event subscription, optimistic mutation, and desktop/Web adapter behavior.

## 3. Verification

- [x] 3.1 Run focused SettingsProvider and Basic Configuration tests.
- [x] 3.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 3.3 Run `cargo test`, `cargo check`, and `cargo clippy` against `src-tauri/Cargo.toml`.
- [x] 3.4 Run strict validation for the change and all main specifications.

## 4. Verification Warning Remediation

- [x] 4.1 Add direct Web adapter coverage for a localStorage-backed save-and-read round trip.
- [x] 4.2 Remove the unintended pnpm lockfile and clear no-content generated line-ending status.
- [x] 4.3 Re-run focused and strict validation, then confirm only intended change files remain.
