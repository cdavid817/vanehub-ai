## Why

`add-antigravity-cli` shipped and archived with four tasks open, all waiting on the same thing: a
single authenticated `agy` run. The archive is immutable history, so without this change the four
outstanding facts would stop being tracked anywhere — which is exactly how an inferred field name
quietly becomes a permanent assumption.

Three of the four are verification, not construction. The implementation is complete and green; it
just rests on two facts that were reasoned about rather than observed. That distinction is worth
keeping visible until it is closed.

## What Changes

- **Capture `init` and `step_update` from a live authenticated run.** The `result` envelope is
  pinned by a verbatim capture, but `init.conversation_id` is currently taken from documentation
  plus its presence in the captured `result` payload — consistent, but inferred. `step_update` has
  never been observed at all.
- **Parse `step_update` into incremental output**, replacing the deliberate no-op the parser ships
  today. Until then a turn delivers its whole reply at once through `result.response`, so this
  restores token-by-token streaming rather than fixing a broken turn.
- **Fill the `--model` catalog's known values** from `agy models`, which currently offers only
  `default` because the real slugs need an authenticated session to list.
- **Run the desktop app end to end** against an authenticated install and confirm a managed chat
  invocation streams output and records reported usage.

### Constraint worth stating up front

`agy` cannot be driven from the agent harness on this machine: it loads a valid token from the
Windows keyring in milliseconds and then abandons its own auth step after a hard 10-second budget,
because a non-interactive session has no interactive desktop context. The captures have to come
from a human-run terminal. This is recorded so the next person does not spend the same hour
rediscovering it.

## Capabilities

### New Capabilities
(none — this change completes verification of an existing capability)

### Modified Capabilities
- `native-runtime-architecture`: replaces the requirement that `step_update` be consumed without
  emitting increments with one that maps it to incremental output, once its payload is observed.

## Impact

- Rust, `agent_runtime` context: `providers/output.rs` (the `step_update` arm), and
  `providers/fixtures/antigravity-cli.output.jsonl` re-pinned to a real capture rather than the
  current placeholder line.
- `contexts/tooling/cli_parameters.rs` and `src/services/cli-parameter-catalog.ts`: the `--model`
  known-value list.
- No schema, command-signature, or configuration-format change.
