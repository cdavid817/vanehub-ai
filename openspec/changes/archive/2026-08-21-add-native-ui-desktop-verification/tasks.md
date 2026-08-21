## 1. Shared configuration

- [x] 1.1 Extend `createDesktopConfig` to accept an explicit ordered spec list alongside the directory glob
- [x] 1.2 Confirm every layer still owns a disjoint spec set
  - Guarded in `desktop-orchestrator.node-test.mjs`: each mode's configuration must name `specs-<mode>`.

## 2. Session workspace layer

- [x] 2.1 Add `tests/desktop/specs-session-workspace/` and its wdio configuration, reusing the CLI fixture so the Agent tab has a deterministic Agent
- [x] 2.2 Create a real session through the desktop UI against a real workspace folder
- [x] 2.3 Visit every workspace tab; assert selection moves and the visible panel is that tab's panel
  - Asserted through `aria-controls` rather than "some panel is visible": a workspace that moves the selected tab without moving the panel looks correct in a screenshot.
- [x] 2.4 Assert each panel renders its own content rather than an empty shell
  - The first pass passed vacuously. Eight of nine panels are lazy chunks and were sitting at `正在加载功能...`, which satisfied a `text.length > 0` check. With the placeholder excluded, all nine resolve and carry real content: the workspace tab renders the fixture Agent's banner inside the live terminal, `变更` a real diff of the fixture repository, `文档` the file's real contents, `日志` 1,172 characters of unified log.
- [x] 2.5 Assert no fatal frontend error after the traversal

## 3. Dialog layer

- [x] 3.1 Add `tests/desktop/specs-dialogs/` and its wdio configuration
- [x] 3.2 Assert a main-path dialog exposes `role="dialog"`, takes focus, and closes on Escape with focus restored
  - `aria-modal` is asserted too. A dialog that traps focus is a desktop-only failure: there is no browser chrome to escape to.
- [x] 3.3 Assert a dialog submit path completes against the real service boundary
  - The submitted session must exist natively afterwards with the expected Agent, and cancelling must leave the native session count unchanged.
  - Application shutdown moved to an `after` hook: one application instance serves a whole spec file, so exiting inside the first test left the second talking to a dead runtime.

## 4. Settings persistence layer

- [x] 4.1 Add `tests/desktop/specs-settings-persistence/` with two ordered specs sharing one run context
- [x] 4.2 Spec one: change a setting through the rendered UI and assert the settings service reports it
  - WebDriver's select interaction does not take under WebKitGTK, and a native select popup is not reliably automatable across the three webviews this must pass on, so the control is driven by the `change` event it listens for. Handler, service boundary, IPC, storage, and relaunch all stay real.
  - Reading the element back after dispatch returns the pre-save value — the control is controlled and React re-renders synchronously inside the discrete event. Native storage is the only honest signal, and it is what the layer is about.
- [x] 4.3 Spec two: after relaunch, assert the settings service still reports it and the UI presents it
- [x] 4.4 Confirm the assertion reads native settings, never browser storage
  - Guarded: the relaunch spec must invoke `get_settings` and must not mention `localStorage`. `tests/e2e/personalization-settings.spec.ts` asserts persistence by reading `localStorage`, which cannot fail for a defect in the path the desktop client uses.
  - Also guarded: the relaunch spec must not import its sibling. It did at first, which registered that spec's `describe` a second time in the worker that loaded it.

## 5. Wiring, guards, and validation

- [x] 5.1 Add the three modes to `scripts/test-desktop.mjs` and include them in `all`
- [x] 5.2 Add the three npm scripts
- [x] 5.3 Extend `scripts/desktop-orchestrator.node-test.mjs` for the added layers
- [x] 5.4 Negative-check each layer: confirm it fails when the behavior it claims to verify is absent
  - Session workspace: inverting the panel/tab correspondence fails the layer. Dialogs: removing the Escape keystroke fails both tests. Persistence: suppressing the write fails the relaunch assertion.
- [x] 5.5 Name the layers in AGENTS.md
- [x] 5.6 `openspec validate add-native-ui-desktop-verification --strict`
- [x] 5.7 Run the mandatory command set; record the per-platform result
  - Linux: all five layers `PASSED`. Windows and macOS `NOT RUN`; CI's matrix runs `npm run test:desktop`, so all five reach all three platforms without a workflow change.
