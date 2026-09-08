## 1. Routing and destination model

- [x] 1.1 Collapse `WorkspaceDestination` to `sessions | inbox | automations`, add the optional `view` segment, and extend `parseWorkspaceLocation` / `formatWorkspaceLocation` with legacy-path redirects
- [x] 1.2 Update `use-main-layout-model.ts` and `workspace-route.test.ts` for the new location shape, persisted-location fallback, and redirect table

## 2. Activity bar

- [x] 2.1 Refactor `WorkspaceActivityBar` to render from `items` / `utilityItems` configuration while keeping the Sessions sidebar-toggle semantics and icon-only accessibility contract
- [x] 2.2 Wire the four entries in `main-layout.tsx`, remove the retired callbacks and `xxxVisited` flags, and register `Mod+1..4` shortcuts with input-focus guards
- [x] 2.3 Add `layout.activityBar.inbox` / `.automations` keys and remove retired keys across every registered locale

## 3. Inbox

- [x] 3.1 Create `src/inbox/` composing Mission Control sections and unread System Activity items into Needs attention / Running / Recently finished, reusing `event-coalescer.ts` and the presentation registry
- [x] 3.2 Move badge ownership to Inbox, de-duplicate counts by run id, and add an Activity log disclosure that renders the existing timeline view
- [x] 3.3 Add the List / Board view toggle rendering `work-board` in board mode, persisted with layout preferences

## 4. Automations

- [x] 4.1 Create `src/automations/` as a lazy tab shell over Loop Center, scheduled tasks, and Goal Center
- [x] 4.2 Render the scheduled-task form and list inline as the Scheduled tab, preserving validation, refresh, mutation, and error-retention behavior
- [x] 4.3 Route `onNavigate` targets emitted by Mission Control and Goal Center to the new destinations and views

## 5. Settings relocations

- [x] 5.1 Add the `evaluation` settings page under the Agents group wrapping `evaluation-center`
- [x] 5.2 Add a System activity section to the `observability` page hosting export, rebuild, and health-panel controls
- [x] 5.3 Move Help into the settings sidebar bottom group and drop the activity-bar Help entry

## 6. Verification

- [x] 6.1 Update component tests for the activity bar, routing, Inbox composition, badge counting, and Automations tabs
- [x] 6.2 Rewrite E2E flows that navigated through retired entries; add coverage for legacy-path redirects and `Mod+1..4`
- [x] 6.3 Visual QA of Inbox and Automations in `futuristic` and `minimal` styles at desktop and narrow widths
- [x] 6.4 Run frontend lint, tests, build, i18n parity, UI E2E, and strict OpenSpec validation

## Verification record (2026-09-08, Linux x86_64, worktree `feat/ui-optimization` on main 7b59c657)

Per-group acceptance was run after each group before the next began; the final sweep below ran after Group 6.

| Command | Result |
| --- | --- |
| `npm run lint:ci` | PASSED |
| `npm run test` | PASSED (498 files, 3058 tests; under host load 20 one lazy-import timeout in `settings-pages.test.ts` appeared once and passed alone) |
| `npm run build` | PASSED (16 lazy chunks verified) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo check --workspace` | PASSED |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED |
| `npm run native:panic:check` | PASSED |
| `cargo test --workspace` | PASSED on rerun with `CARGO_BUILD_JOBS=2` (first attempt SIGKILLed by the 16 GB host's memory watchdog while compiling; no Rust changed) |
| `openspec validate --specs --strict` | PASSED |
| `openspec validate consolidate-activity-bar --strict` | PASSED |
| `npm run architecture:check` | PASSED |
| `npx playwright test` | PASSED (277 tests, 26 min) |
| `npm run test:desktop:build` | PASSED |
| `npm run test:desktop:scheduled-tasks` (Linux) | PASSED |
| `npm run test:desktop:smoke` (Linux) | FAILED — 24/25 specs pass; `domain-skills` fails on a native `storage` error from `set_skill_tool_trust` while the native log shows repeated "Execution timeline retention was deferred after a storage error"; earlier runs failed `domain-prompt-hooks` / `domain-cli-tooling` with "database is locked". These specs drive Tauri commands directly and passed in the first run of the day; the change-relevant specs (`ui-workspace`, `screen-sweep`, `ui-evaluation`, `smoke`) pass in every run. |
| Windows / macOS desktop layers | NOT RUN (only the current platform was exercised) |

Visual QA (6.3): `tests/e2e/inbox.spec.ts` and `tests/e2e/automations.spec.ts` screenshot Inbox (list, activity log, board) and every Automations tab in `futuristic` and `minimal` at 1440 px and 390 px; screenshots inspected, no overlap, truncation, low contrast, or blank panel. The narrow Inbox header wraps its refresh button to a second row, which is acceptable.

Notes:
- Hosted surfaces changed only at their container: `mission-control.tsx` exports `RunCard` (additive) and `SystemActivityView` gained a `maintenanceControls` prop (default `true`) so Inbox can hide export/rebuild/health.
- Goal Center emits no `onNavigate`; 4.3 maps Mission Control's `loop`/`goal`/`evaluation`/`review`/`session` targets only.
- Needs attention lists unread `warning`/`critical` activity as specified; `error`-severity events are counted in the badge and remain in the Activity log.
