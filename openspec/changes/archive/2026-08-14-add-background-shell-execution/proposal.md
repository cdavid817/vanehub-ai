## Why

The native tool loop's `shell` tool runs every command in the foreground under one fixed 60-second timeout. That budget is smaller than the ordinary verification commands this repository itself requires — `cargo clippy --all-targets`, `npm run build`, `npx playwright test` — so a native Agent cannot finish the work it is asked to do, and cannot run a dev server, watcher, or any other process that is supposed to outlive a single tool call. The model's only recourse today is to guess at splitting a command into artificially small pieces, which changes what is actually verified.

## What Changes

- Extend the baseline `shell` tool with an explicit, bounded per-call timeout and an opt-in background execution mode that returns a handle instead of blocking on completion.
- Add two baseline tools, `shell_output` and `shell_kill`, so a started background command can be polled incrementally and terminated deliberately.
- Add a session-scoped background command registry with bounded concurrency, a bounded rolling output buffer, and a bounded maximum lifetime, so an unattended process cannot accumulate memory or outlive its session.
- Reap background commands when their owning session ends and when the desktop runtime exits, reusing the existing process-group and job-object containment rather than introducing a second process-management mechanism.
- Keep starting a background command on exactly the same approval surface as an ordinary shell call, classify `shell_output` as a read-only observation, and classify `shell_kill` as an effect-reducing operation that needs no approval.
- Preserve the current default behavior: a `shell` call that supplies neither new parameter still runs in the foreground under the existing 60-second budget.

## Capabilities

### Modified Capabilities

- `agent-tool-execution`: Adds the `shell_output` and `shell_kill` baseline tools, an explicit per-call shell timeout, background shell execution with a bounded lifecycle, and the approval classification for the two new tools.

## Impact

- Only the Rust runtime and the native tool-use loop are affected; no SQLite schema, Tauri command, frontend service contract, or React surface changes.
- The background registry is deliberately in-memory and session-scoped: a background command is a runtime artifact, not a durable record, so a desktop restart ends it rather than resurrecting an unattended process.
- Existing foreground `shell` behavior, its permission classification, and its output caps are unchanged.
- The Web/mock runtime keeps simulating tool calls without real process execution, so its parity behavior is unchanged.
- No new package dependencies are introduced.
