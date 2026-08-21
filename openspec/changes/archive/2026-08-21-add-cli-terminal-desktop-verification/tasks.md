## 1. Fixture CLI

- [x] 1.1 Add `tests/desktop/fixtures/cli/opencode` — an executable that prints a ready banner, echoes each received line with a stable marker, and exits on a stop command
  - Splits on `\r` as well as `\n`: a PTY delivers a carriage return when the user presses Enter, so a newline-only reader would never see a line at all.
- [x] 1.2 Confirm it performs no network I/O, reads no credential store, and tolerates whatever arguments the interactive CLI profile passes
  - Arguments are ignored rather than parsed — an unrecognized flag would otherwise fail the layer for the wrong reason. Asserted in `desktop-orchestrator.node-test.mjs`.
- [x] 1.3 Confirm the executable bit survives a fresh clone (`git ls-files --stage` shows mode `100755`)
  - It did not, at first: this repository sets `core.fileMode=false`, so the fixture was staged `100644` and would have arrived non-executable in CI, failing the layer on all three platforms. Fixed with `git update-index --chmod=+x`; the test now guards it.

## 2. Layer wiring

- [x] 2.1 Add `tests/desktop/wdio.cli-terminal.conf.mjs` reusing the tauri service options from `wdio.conf.mjs`, with the fixture directory ahead of the inherited `PATH`
  - The shared options moved to `tests/desktop/wdio-shared.mjs` and both configurations now call it, so a service option cannot drift between layers.
- [x] 2.2 Add `tests/desktop/specs-cli-terminal/cli-terminal.e2e.mjs` asserting terminal `running`, banner output, input echo, and clean stop
  - `stop_agent_terminal` takes `terminalId`, not `sessionId`; the first run failed on that and the spec was corrected.
  - The surviving-process assertion is baseline-relative. Counting absolutely made a fixture left behind by an earlier aborted run permanently unsatisfiable, which failed the layer for an unrelated reason.
- [x] 2.3 Generalize `smokeDesktop` in `scripts/test-desktop.mjs` into a reusable wdio-layer runner; keep the `desktop-smoke` layer's behavior and evidence identical
- [x] 2.4 Add the `cli-terminal` mode and include the layer in `all`
  - `all` takes the worst layer result rather than the last one, so a failing first layer cannot be masked by a passing second.
- [x] 2.5 Add the `test:desktop:cli-terminal` npm script

## 3. Evidence and guards

- [x] 3.1 Verify the fixture is resolved rather than a real CLI Agent, by asserting on a marker only the fixture emits
  - Negative check: with the fixture moved aside the layer reports `FAILED` with `Command 'opencode' was not found on PATH.` rather than passing vacuously or falling back to an installed Agent.
- [x] 3.2 Verify the existing smoke layer's environment is unchanged — `smoke.e2e.mjs` still reports its evaluation branch the same way
  - `desktop-smoke` still reports `PASSED` and still logs `BLOCKED: native evaluation requires one installed managed CLI Agent`. This is why the layer needs its own configuration: shadowing `opencode` makes it `available`, and the smoke starts an evaluation for the first available CLI Agent it finds.
- [x] 3.3 Extend `scripts/desktop-orchestrator.node-test.mjs` and `scripts/desktop-verification-entrypoint.node-test.mjs` for the added layer
  - `desktop:unit:test` 14/14. `scripts/test-verify.mjs` also needed the cargo `--workspace` commands and `native:panic:check` that AGENTS.md had already moved to; its entrypoint test was failing on that drift.
- [x] 3.4 Record the per-platform result; this branch can only claim Linux
  - Linux `PASSED` (WebKitGTK 605.1.15). Windows and macOS `NOT RUN`; CI's `Desktop Smoke` matrix runs `npm run test:desktop`, so both layers reach all three platforms without a workflow change.

## 4. Documentation and validation

- [x] 4.1 Name the new layer in the AGENTS.md desktop bullet
- [x] 4.2 `openspec validate add-cli-terminal-desktop-verification --strict`
- [x] 4.3 Run the mandatory command set from AGENTS.md
  - `lint:ci`, `test` (1301), `build`, `docs:check`, `openspec validate --specs --strict` (138), `desktop:unit:test` (14), `test:desktop` (both layers `PASSED`) all green. No Rust file is touched by this change; the last full `cargo test --workspace` on this tree passed 3590/0.
