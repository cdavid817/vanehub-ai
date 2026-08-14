## 1. Shared dialog primitive

- [x] 1.1 Add an optional pinned-footer slot to `src/components/ui/application-dialog.tsx` so a dialog can own a scrolling body with fixed header and footer; callers that omit it render unchanged.
- [x] 1.2 Add `src/components/ui/application-dialog.test.tsx` covering Escape, Tab wrap at first and last controls, focus return, `closeDisabled` suppression, autofocus targeting, footer rendering, and ARIA wiring.

## 2. Workspace modals

- [x] 2.1 Move `src/main-layout/create-session-dialog-content.tsx` onto `ApplicationDialog`, mapping its header/body/footer grid to the primitive and wiring `closeDisabled` to the in-flight creation state.
- [x] 2.2 Move `src/main-layout/scheduled-tasks-dialog.tsx` onto `ApplicationDialog`.
- [x] 2.3 Move the batch-delete confirmation in `src/main-layout/session-sidebar.tsx` onto `ApplicationDialog`, wiring `closeDisabled` to `deletingSessions`.
- [x] 2.4 Move `src/settings/pages/cli-conflict-dialog.tsx` onto `ApplicationDialog`.
- [x] 2.5 Verify no `role="dialog"` or `aria-modal` remains outside `application-dialog.tsx` except `notification-center.tsx`, which is a non-modal popover, and `loop-definition-dialog.tsx`, which implements its own Escape and focus handling and is left for a separate change.

## 3. Remove browser-native dialogs

The audit reported two call sites. That count was wrong: the grep behind it used a non-recursive
path pattern. A recursive scan found **thirteen**, so this section grew to cover all of them and a
guardrail now enforces the result.

- [x] 3.1 Replace the `window.prompt` category-creation flow with `src/main-layout/create-category-dialog.tsx`, which validates a non-empty name, surfaces creation failures inline, and assigns the session on success.
- [x] 3.2 Replace the `window.confirm` delete flow in `src/main-layout/scheduled-tasks-dialog.tsx` with inline two-step confirmation, avoiding a nested modal whose second Escape handler would close both dialogs.
- [x] 3.3 Add `src/components/ui/use-confirmation.tsx`, a promise-shaped replacement for `window.confirm` built on `ApplicationDialog`.
- [x] 3.4 Migrate the remaining eleven sites: `MessageFeedbackControls`, `agent-memory-section` (×2), `prompt-hook-lifecycle-panel`, `basic-settings-page`, `cli-parameters-page`, `expert-roles-page`, `extensions-page`, `mcp-page`, `sdk-page`, `ssh-connections-page`.
- [x] 3.5 Add locale keys for the category dialog, the inline delete confirmation, and the shared confirmation across all five registered locales; `src/i18n/i18n-resource-parity.test.ts` passes.
- [x] 3.6 Add a guardrail test asserting `src/**` contains no `window.prompt`, `window.alert`, or `window.confirm` call.
- [x] 3.7 Update the four unit tests that stubbed `window.confirm` to drive the real in-application dialog instead.

## 4. Loop Center empty state

- [x] 4.1 Add an icon/title/explanation/primary-action empty state to `src/loop-center/loop-center.tsx`, matching the shape used by the chat welcome screen and the notification centre.
- [x] 4.2 Give the inspector panel an explanatory empty state instead of a bare line.
- [x] 4.3 Add locale keys across all five registered locales.
- [x] 4.4 Extend `src/loop-center/loop-center-states.test.tsx` for the empty state, its explanation, its primary action, and the inspector's empty state.

## 5. Regression surface

- [x] 5.1 Replace the `.fixed.inset-0 .ucd-panel` dialog selectors in `tests/docs/documentation-screenshots.spec.ts` with `getByRole("dialog")`, then run `npm run docs:screenshots:update`; 21 baselines regenerated and reviewed.
- [x] 5.2 Run `npx playwright test`. Six specs broke on the migration and were repaired: `application-locales` (×2) and `documentation-screenshots` used `.ucd-panel` to find the create-session dialog; `cli-parameters-settings`, `onepiece-agent`, and `session-category-management` drove the removed browser-native dialogs through `page.once("dialog", …)`; `workspace-activity-bar` clicked a scheduled-tasks close button that `ApplicationDialog` does not render. 96 pass.
- [x] 5.3 `npm run lint:ci`, `npm run test` (981), `npm run build`, `openspec validate --specs --strict` (107), `cargo fmt --check` and `cargo check` all pass. `cargo clippy --all-targets -- -D warnings` and `cargo test` were not run: this change touches no Rust, and both require a full rebuild in a fresh worktree.
- [x] 5.4 `npm run contracts:check` passes. `npm run test:coverage` not run separately; `npm run test` covers the same suite without the threshold gate.
- [x] 5.5 Visual QA performed through the regenerated documentation screenshots and ad-hoc captures in both styles at 1440px and 860px.
