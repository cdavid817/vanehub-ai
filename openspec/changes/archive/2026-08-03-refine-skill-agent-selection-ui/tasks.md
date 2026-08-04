## 1. Selection Board Presentation

- [x] 1.1 Refactor selected-Agent Skill rendering into responsive Assigned and Available panels with deterministic counts, bounded empty states, and Assigned-first narrow-layout order.
- [x] 1.2 Replace immediate-action assignment checkboxes with explicit Assign/Remove buttons, row-scoped pending feedback, and stable-Agent accessible names.
- [x] 1.3 Keep selected-Agent rows focused on preview and relationship status while retaining enable/edit/delete lifecycle controls only in All Skills.

## 2. Localization and Regression Coverage

- [x] 2.1 Add synchronized zh-CN, zh-TW, en, ja, and ko strings for selection-board descriptions, actions, pending states, and focused empty states.
- [x] 2.2 Extend presentation and interaction tests for CLI/API boards, explicit actions, omitted lifecycle controls, pending isolation, and row-owned failures.
- [x] 2.3 Extend Skill Playwright coverage for wide parallel panels, narrow stacked order, long labels, keyboard activation, and absence of assignment checkboxes.
- [x] 2.4 Add equivalent English and Simplified Chinese user-guide chapters that explain All Skills, Unassigned, enablement, per-Agent assignment, outcomes, and runtime differences.
- [x] 2.5 Add direct presentation assertions for Assigned/Available counts and both panel empty states.
- [x] 2.6 Add direct state-derivation assertions for mounted CLI bindings and paused CLI/API bindings.

## 3. Verification

- [x] 3.1 Run `npm run lint`, `npm run test`, `npm run build`, and targeted Skill Playwright tests.
- [x] 3.2 Run `cargo test`, `cargo check`, and `cargo clippy` for `src-tauri/Cargo.toml` to preserve shared desktop integration confidence.
- [x] 3.3 Run `openspec validate refine-skill-agent-selection-ui --strict` and `openspec validate --specs --strict`, then record the verification result.

## Verification Result

- 2026-08-03: ESLint passed; Vitest passed 434/434; production build and frontend chunk checks passed.
- 2026-08-03: targeted Skill Playwright passed 3/3 on Chromium.
- 2026-08-03: Cargo tests passed 1049 executed tests across library and architecture suites (3 ignored); `cargo check` and warning-denying Clippy passed.
- 2026-08-03: the change and all 84 main specs passed strict OpenSpec validation.
- 2026-08-03: documentation unit, README parity, link, mdBook test, and complete documentation build checks passed with the new English and Simplified Chinese Skill chapters.
- 2026-08-03: verification follow-up added direct panel count/empty-state and mounted/paused relationship-state coverage; full Vitest passed 436/436, with lint, build, Cargo test/check/Clippy, and strict OpenSpec validation rerun.
