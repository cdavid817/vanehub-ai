## 1. Tool Event Reconciliation

- [x] 1.1 Add a tested pure helper that upserts tool activities by stable id while preserving defined prior input/output fields.
- [x] 1.2 Use the helper in shared chat event reduction and Web/mock message updates so desktop and browser rendering share status-transition semantics.
- [x] 1.3 Defensively normalize duplicate ids already present in loaded message tool history.

## 2. Compact Tool Activity UI

- [x] 2.1 Replace the flat tool card list with a compact activity summary and ordered actionable/completed groups.
- [x] 2.2 Keep approval-required calls visible with their existing controls and put successful history behind keyboard-accessible disclosure.
- [x] 2.3 Add bounded command/path/action previews, localized known-tool labels, localized statuses, counts, and disclosure text in every supported locale.

## 3. Tests and Verification

- [x] 3.1 Add reducer, Web adapter, component, localization, accessibility, and defensive-normalization tests.
- [x] 3.2 Add or update Playwright coverage for a tool-heavy OnePiece/Web turn, including compact history and actionable activity behavior.
- [x] 3.3 Run frontend lint, unit tests, coverage, build, Playwright, Rust checks required by repository policy, and strict OpenSpec validations.

## 4. Compact Failure History

- [x] 4.1 Add component tests for default-collapsed recoverable failures, initially disclosed blocking failures, and consecutive identical failure aggregation without evidence loss.
- [x] 4.2 Render failed calls in a separate accessible disclosure, default it from the assistant terminal status, and retain the failed total in the activity summary.
- [x] 4.3 Aggregate consecutive identical failure presentation with an occurrence count and add localized failure-history labels in every supported locale.
- [x] 4.4 Update OnePiece Playwright coverage and rerun repository-required frontend, Rust, documentation, and strict OpenSpec validations.

## 5. Collapsible Activity Region

- [x] 5.1 Add component tests for successful-history default collapse, active manual collapse, retained user preference, approval override, blocking-failure default expansion, and accessible toggle state.
- [x] 5.2 Replace the static activity header with an accessible outer toggle that keeps counts and the current active preview visible while collapsed.
- [x] 5.3 Implement message-local disclosure state so terminal success auto-collapses before user interaction, manual choices persist, approvals force expansion, and blocking failures open initially.
- [x] 5.4 Update OnePiece Playwright coverage for outer collapse and rerun applicable frontend, browser, OpenSpec, and Rust validation.
