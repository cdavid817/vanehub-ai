## Why

The desktop verification layer proves the app starts, crosses IPC, and navigates. It never proves the one thing the product is for: that a managed CLI Agent actually runs inside the app and that a keystroke reaches it and its output comes back.

That gap was measured, not assumed. Driving the real desktop client by hand showed the chain works — `claude-code`, `codex-cli`, and `opencode` each launched a real PTY, answered a real prompt, and streamed the answer back through `agent-terminal:event`. The existing `smoke.e2e.mjs` covers none of it: it exercises `create_session` and `open_code_review`, never `open_agent_terminal`, and its evaluation branch reports `BLOCKED` on any host without a managed SDK installed — which is every CI runner.

The reason this was never automated is cost, not difficulty: a real CLI Agent means a real model call, real credentials, and a nondeterministic answer. None of those belong in CI. A fixture CLI removes all three while leaving the part under test — the PTY round trip through the native runtime — completely real.

## What Changes

- Add a fixture CLI executable that behaves like a managed CLI Agent's interactive binary: it prints a ready banner, echoes what it is sent with a stable marker, and exits on a stop command. It performs no network I/O and holds no credentials.
- Add a `desktop-cli-terminal` verification layer that puts the fixture ahead of the real one on `PATH`, so the native runtime resolves the builtin `opencode` Agent to the fixture and the whole launch path — CLI profile load, executable resolution, PTY creation, reader/writer setup — runs unchanged against it.
- Assert the round trip: the Agent terminal reaches `running`, the banner arrives as an `output` event, input sent through `send_agent_terminal_input` comes back echoed, and `stop_agent_terminal` ends it with no owned process left behind.
- Run the new layer under its own wdio configuration so the existing smoke keeps its current environment. Shadowing `opencode` makes it `available`, and `smoke.e2e.mjs` starts an evaluation for the first available CLI Agent it finds — sharing one environment would silently change what the smoke tests.
- **No product code changes.** No Rust file, no Tauri command, no React component, no service boundary.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-runtime-verification` — one added requirement covering the CLI Agent terminal round trip and the fixture-Agent isolation it depends on.

## Impact

- `tests/desktop/fixtures/cli/` — new fixture executable.
- `tests/desktop/specs-cli-terminal/` — new spec directory, kept apart from `specs/` so each wdio configuration owns a disjoint set.
- `tests/desktop/wdio.cli-terminal.conf.mjs` — new configuration; same tauri service options as `wdio.conf.mjs`, plus the fixture `PATH`.
- `scripts/test-desktop.mjs` — `smokeDesktop` generalizes into a reusable wdio-layer runner; a `cli-terminal` mode joins `build`/`smoke`/`all`, and `all` gains the new layer.
- `scripts/desktop-orchestrator.node-test.mjs`, `scripts/desktop-verification-entrypoint.node-test.mjs` — extended to cover the added layer.
- `package.json` — `test:desktop:cli-terminal` script.
- `AGENTS.md` — the desktop bullet names the new layer.
- CI `Desktop Smoke` job — gains one layer on all three platforms. No new secret, credential, or network dependency.
