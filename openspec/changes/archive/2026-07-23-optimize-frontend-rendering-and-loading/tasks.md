## 1. Virtualization Foundation

- [x] 1.1 Add `@tanstack/react-virtual` with npm and commit the resulting `package.json` and `package-lock.json` changes.
- [x] 1.2 Add reusable measured virtual-list utilities that preserve stable item keys, bounded overscan, semantic position metadata, and Tailwind-owned static styling.
- [x] 1.3 Add reusable localized Suspense and retryable module-load boundary components using the existing error-boundary dependency.

## 2. Prompt Hook Windowing

- [x] 2.1 Refactor the Prompt Hook card renderer so ordinary and virtual collections share the same card operations and stable hook-id keys.
- [x] 2.2 Implement the more-than-500 threshold, responsive one/two-column virtual rows, measured variable heights, and four-row overscan.
- [x] 2.3 Reset and remeasure Prompt Hook virtualization after filter, search, sort, grouping, or responsive-column changes without losing hook operations.
- [x] 2.4 Add component coverage for the 500/501 boundary, bounded mounted rows, responsive regrouping, offscreen operations, and accessible collection metadata.

## 3. Session Log Virtualization and Seek

- [x] 3.1 Replace direct log article mapping with measured variable-height virtualization, stable log-id keys, ten-entry overscan, and a terminal load-more item.
- [x] 3.2 Preserve newest-first pagination, id de-duplication, filter reset, redacted context rendering, and export behavior through the existing `agentService`.
- [x] 3.3 Add the localized timestamp input, validation, loaded-range seek, virtual scroll, and programmatic focus behavior.
- [x] 3.4 Implement bounded deep seek that reads at most ten cursor pages per action, preserves filters, supports continuation, and reports out-of-range timestamps.
- [x] 3.5 Add component coverage for variable heights, mounted-row bounds, pagination de-duplication, invalid timestamps, found targets, ten-page limits, continuation, and no-match results.

## 4. Lazy Feature Modules

- [x] 4.1 Convert designated heavy settings page registrations, including Agents and Prompt Hooks, to named-export-compatible `React.lazy()` imports.
- [x] 4.2 Update the settings shell so unvisited modules stay unloaded, visited pages remain mounted and hidden, and loading/failure states do not reset page state.
- [x] 4.3 Dynamically import Loop Center on first Loops activation while preserving mounted session workspace and loaded task-board state.
- [x] 4.4 Keep Chat eager and convert non-default session tab panels to lazy imports integrated with the existing per-session mounted-tab set.
- [x] 4.5 Add tests for initial unloaded state, first-load fallbacks, retryable failures, settings and Loop keep-alive, tab keep-alive, and session-switch resets.
- [x] 4.6 Add a deterministic Vite manifest or build-output assertion that designated feature modules are emitted outside the initial entry chunk.

## 5. Localization and End-to-End Coverage

- [x] 5.1 Add synchronized zh-CN and en strings for module loading, module-load retry, timestamp input, validation, seek progress, continuation, and no-match states.
- [x] 5.2 Add Playwright coverage for scrolling to offscreen Prompt Hooks and log entries, timestamp focus, lazy feature activation, and state preservation.
- [x] 5.3 Verify virtualized and lazy-loaded layouts at desktop and narrow browser viewports with no overlap, clipping, inaccessible controls, or unstable loading dimensions.
- [x] 5.4 Verify both Tauri-selected and Web/mock runtime paths continue to use the unchanged frontend service boundary and deterministic Web fixtures.

## 6. Validation

- [x] 6.1 Run `npm run lint` and resolve all frontend lint failures.
- [x] 6.2 Run `npm run test` and `npm run build`, including virtualization, lazy-loading, and bundle-splitting assertions.
- [x] 6.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 6.4 Run `openspec validate optimize-frontend-rendering-and-loading --strict` and `openspec validate --specs --strict`.
