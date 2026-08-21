## 1. Presentation Model and Localization

- [x] 1.1 Add typed Prompt Hook management-view state and pure helpers for primary/additional filters, compact summary values, stable category grouping, and flattened virtual rows.
- [x] 1.2 Add unit tests for combined filters, category counts and order, group expansion resets, and stable flattened-row keys.
- [x] 1.3 Add synchronized locale keys for management/runtime navigation, compact summaries, additional-filter state, category controls, unified detail sections, and accessible action labels in every supported locale.

## 2. Compact Management Inventory

- [x] 2.1 Refactor the Prompt Hooks page into Hook-management and runtime-records views while preserving service-boundary queries, mutations, refresh-with-previous-data behavior, and desktop/Web parity.
- [x] 2.2 Replace the metric-card grid and count panel with one compact inventory summary and keep creation and refresh actions clearly prioritized.
- [x] 2.3 Replace the five-control toolbar with directly accessible search, enabled-state, and stable CLI filters plus an additional-filter surface for source, stage, and category, including active-state and clear-all controls.
- [x] 2.4 Replace large two-column cards with accessible category headings and compact Hook rows showing identity, source, enabled state, publication state, binding count, toggle, detail entry, and overflow actions.
- [x] 2.5 Adapt measured virtualization above 500 Hooks to flattened category-heading and compact-Hook rows with stable order, responsive remeasurement, bounded overscan, correct result resets, and accessible collection metadata.

## 3. Unified Hook Detail Workflow

- [x] 3.1 Build one responsive, focus-managed Hook detail surface with overview, content/publication, and version-history sections that remains usable at desktop and narrow widths.
- [x] 3.2 Move enablement, stable CLI binding, category, stage, order, governance, and identity presentation into the overview section with source-appropriate edit restrictions.
- [x] 3.3 Consolidate user-Hook metadata and template editing, service-backed variable insertion, explicit preview, save-draft, live-version state, and publish actions into the content/publication section.
- [x] 3.4 Move immutable version summaries, operational evaluations, attribution guidance, confirmed rollback, and preserved-draft state into the version-history section.
- [x] 3.5 Reuse the unified field vocabulary for creation and retain localized validation, bounded content preview, and confirmed deletion without separate edit and advanced inventory entry points.

## 4. Runtime Records

- [x] 4.1 Move assembled-prompt preview and recent safe Hook trace summaries into the runtime-records view and remove the trace table from the management document flow.
- [x] 4.2 Make trace loading view-aware while retaining previous trace data during refresh, explicit full-content preview rules, empty states, failure states, and stable agent-id inputs.

## 5. Automated UI Coverage and Documentation

- [x] 5.1 Update Prompt Hook component tests for compact summaries, task-view navigation, progressive filters, category expansion, locked toggles, binding mutations, and refresh state.
- [x] 5.2 Update Prompt Hook interaction tests for create, unified edit-to-draft-to-publish flow, preview, rollback, deletion confirmation, keyboard focus, and built-in governance restrictions.
- [x] 5.3 Extend virtualization tests past 500 Hooks for grouped row flattening, overscan, filtering, responsive measurement, offscreen operations, and accessible positions.
- [x] 5.4 Update Playwright coverage for the management and runtime-records journeys in Web/mock mode and regenerate the localized Prompt Hook user-guide screenshots and surrounding guidance.

## 6. Required Verification

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 6.2 Run `npm run test:coverage` and confirm the project coverage thresholds remain satisfied.
- [x] 6.3 Run `npx playwright test` for the UI behavior change and record the result.
- [x] 6.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 6.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 6.6 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.7 Run `openspec validate simplify-prompt-hooks-settings-experience --strict` and `openspec validate --specs --strict`.
