## 1. Characterization and Test Fixtures

- [x] 1.1 Extend Skill settings test fixtures with representative immutable System, mutable User override, unsupported Utility, compatibility-defaulted, resource-bearing, and shadowed definitions.
- [x] 1.2 Add failing component tests that assert the default row emphasizes identity, enabled state, effective layer, type, description, and its context-specific primary action without rendering the full runtime metadata badge set.
- [x] 1.3 Add failing component tests for System read-only and unsupported Utility summaries, including icon-or-text semantics, omitted invalid actions, and access to the full explanation through Details.
- [x] 1.4 Add failing interaction tests for opening details, changing the selected Skill, reconciling selection after filtering or refresh, and preserving view, filters, assignments, and enablement.
- [x] 1.5 Add failing tests for deterministic effective-and-shadowed precedence ordering, usage and resource summaries, and the separation between Details and `SKILL.md` Preview.

## 2. Selection and Detail Component Foundation

- [x] 2.1 Add page-local selected Skill identity state based on the existing canonical identity helper and reconcile it against the visible effective inventory.
- [x] 2.2 Create a shared, presentation-only Skill detail body for runtime facts, immutable and unavailable explanations, compatibility state, usage, resources, and precedence.
- [x] 2.3 Create an ordered precedence timeline that labels the effective definition and every shadowed definition with layer, origin, version, availability, and explicit effective or shadowed state.
- [x] 2.4 Create a responsive detail surface that renders the shared body once as a labeled supporting inspector on wide content regions or a focus-managed application panel on narrower regions.
- [x] 2.5 Preserve trigger focus for narrow detail dismissal and expose selected or expanded state, sequential headings, translated accessible names, and visible focus indicators.

## 3. Inventory Row Hierarchy

- [x] 3.1 Extract a compact Skill summary component that keeps production TSX files within 300 physical lines and uses stable Skill ids rather than array indexes.
- [x] 3.2 Refactor lifecycle rows to show bounded identity and state information, concise version or usage text where appropriate, and Details as a secondary action.
- [x] 3.3 Replace the large System warning treatment with a compact lock/read-only summary while retaining Preview and omitting Edit and Delete.
- [x] 3.4 Replace repeated unsupported Utility messaging with one concise row notice, omit Role assignment behavior, and retain the full inspector explanation.
- [x] 3.5 Remove the row-inline effective-details disclosure after the inspector provides equivalent information and tests cover every migrated field.

## 4. Agent Assignment Experience

- [x] 4.1 Refactor selected-Agent rows so Assign or Remove remains the only primary button while Details and Preview use secondary styling and global mutation controls remain absent.
- [x] 4.2 Keep pending and failed assignment feedback attached to the canonical Skill row while allowing unrelated rows, filters, details, and previews to remain operable.
- [x] 4.3 Preserve Assigned and Available panel counts, wide parallel comparison, narrow Assigned-first document order, and stable Agent id mutations for CLI and API Agents.
- [x] 4.4 Verify that paused CLI bindings and API prompt-injection relationships retain their existing semantic labels without provider-specific UI branches.

## 5. Responsive Styling, Localization, and Runtime Parity

- [x] 5.1 Implement the list-detail layout using existing Tailwind breakpoints and semantic tokens, with no horizontal page scrolling at 375px, breakpoint boundaries, desktop widths, or 200 percent zoom.
- [x] 5.2 Add reduced-motion variants, dark-theme contrast-safe states, non-color selected/status indicators, and appropriately sized pointer targets without adding animation or icon dependencies.
- [x] 5.3 Add the complete detail, timeline, selected-state, read-only, Utility, resource, and responsive-panel key set to English, Simplified Chinese, Traditional Chinese, Japanese, and Korean locales.
- [x] 5.4 Confirm equivalent Tauri and Web/mock adapter responses render the same row hierarchy, detail content, precedence semantics, and immutable or unavailable explanations without changing service contracts.

## 6. Automated UI Verification

- [x] 6.1 Run the focused Vitest Skill settings suites and resolve row, inspector, dialog-focus, and mutation-regression failures.
- [x] 6.2 Add Playwright coverage for the wide list-detail layout, keyboard-only opening and dismissal, focus restoration, Agent primary-action hierarchy, and a 375px narrow viewport with no horizontal overflow.
- [x] 6.3 Run `npx playwright test` and record the UI behavior verification result in this task list. Result: 85 passed in Chromium.
- [x] 6.4 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check` and resolve coverage or contract failures. Result: 719 Vitest tests and all policy, version, and contract checks passed.

## 7. Required Repository Validation

- [x] 7.1 Run `npm run lint:ci`.
- [x] 7.2 Run `npm run test`. Result: 719 tests passed.
- [x] 7.3 Run `npm run build`.
- [x] 7.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 7.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 7.6 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 7.7 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.8 Run `openspec validate refine-effective-skill-management-ui --strict` and `openspec validate --specs --strict`. Result: change valid and 95 main specs passed.
