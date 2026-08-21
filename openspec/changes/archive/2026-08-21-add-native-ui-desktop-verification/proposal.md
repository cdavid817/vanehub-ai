## Why

Desktop verification proves pages mount. It does not prove they work.

Measured on this tree: 243 production `.tsx` files, 37 of them carrying dialogs, a session workspace of 9 tabs, and 17 settings sections over 46 files. Against that, the native client has been driven through exactly one interaction — the smoke's settings-button click — plus a rendering sweep. Every dialog, every workspace tab, and every settings form has been exercised only in `tests/e2e`, which runs the Web/mock adapter under Chromium.

That substitution replaces the two things most likely to break a desktop build. It swaps the data source, so real IPC timing and real native error shapes never appear; and it swaps the rendering engine, so WebKitGTK layout and focus behavior are never observed. A Web e2e suite of 156 passing tests is therefore not evidence about the desktop client, and it is structurally incapable of becoming that evidence.

Settings persistence shows the gap most sharply. `tests/e2e/personalization-settings.spec.ts` asserts persistence by reading `localStorage`; the desktop client persists through the native settings service instead. The Web assertion cannot fail for a defect in the path the desktop client actually uses.

## What Changes

- Add `desktop-session-workspace`: create a real session against the fixture CLI Agent, visit all nine workspace tabs in the native client, and assert each panel renders its own content rather than merely mounting.
- Add `desktop-dialogs`: drive the main-path dialogs in the native client — open, receive focus, close on Escape, and submit — proving the dialog contract holds under WebKitGTK focus behavior.
- Add `desktop-settings-persistence`: change a setting through the real UI, assert it reached native storage through the settings service, relaunch the application against the same application-data directory, and assert both native storage and the rendered UI still carry it. Two ordered specs share one run context so the relaunch is real rather than simulated.
- Extend the shared desktop wdio configuration to accept an explicit ordered spec list, which the persistence layer needs and the glob cannot express.
- **No product code changes.** No Rust file, no Tauri command, no React component, no service boundary.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-runtime-verification` — added requirements for native UI interaction coverage and for settings persistence across a real relaunch.

## Impact

- `tests/desktop/specs-session-workspace/`, `tests/desktop/specs-dialogs/`, `tests/desktop/specs-settings-persistence/` — new spec directories, one per layer, kept disjoint so each wdio configuration owns its own.
- `tests/desktop/wdio-shared.mjs` — gains explicit ordered specs.
- `tests/desktop/wdio.session-workspace.conf.mjs`, `wdio.dialogs.conf.mjs`, `wdio.settings-persistence.conf.mjs` — new configurations.
- `scripts/test-desktop.mjs` — three modes join `build`/`smoke`/`cli-terminal`/`all`.
- `scripts/desktop-orchestrator.node-test.mjs` — extended for the added layers.
- `package.json`, `AGENTS.md` — new scripts and the layer list.
- CI `Desktop Smoke` job — gains three layers on all three platforms. No new secret, credential, or network dependency; the session layer reuses the existing CLI fixture, so no layer performs a model call.
