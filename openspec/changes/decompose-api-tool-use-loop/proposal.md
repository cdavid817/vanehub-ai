## Why

`split-api-adapter-modules` turned `api_process_adapter.rs` into a directory module and recorded one
residual in writing: `execute_with_code_intelligence`, 978 lines in a single function, was **moved
intact and not decomposed**. Its proposal said why:

> Decomposing it is the real work, and it is not a move: it means finding seams in live control flow
> that handles tool dispatch, code-intelligence fallback, and error propagation. That belongs in its
> own change, with its own review and its own risk budget, not smuggled into a relocation whose
> entire safety argument rests on "nothing changed but the file it lives in".

This is that change. `src-tauri/tests/architecture.rs` carries the marker:

```
// 978 of these lines are `execute_with_code_intelligence` ... This entry retires when that
// function does.
```

The function is the whole generation path for `launch_kind = "api"` agents: endpoint and credential
resolution, tool-catalog assembly, the SSE streaming round, the permission gate, seven distinct
tool-dispatch branches, compaction, and about forty early returns. Every one of those is reachable
in production on the first message a user sends to an API agent.

## What Changes

- Extract six seams out of `execute_with_code_intelligence` into the modules that already own each
  concern, plus one new `endpoint.rs` for the concern that has no existing home.
- Decline four further candidate seams and record, in `design.md`, the specific loop-carried state
  or exit shape that makes each unsafe to cut.
- Add characterization tests for the three behaviours a seam touches that no existing test covers,
  **before** extracting them, and confirm each passes against the un-split function first.
- Lower the `[ARCH-NATIVE-006]` path budget for `execution.rs` to its measured post-change value and
  rewrite the residual comment to state the new function size.

## The safety argument is different from the ones before it

Every prior change in this work stream — `extract-api-adapter-inline-tests`,
`relocate-heavyweight-inline-tests`, `split-database-migrations`, `split-web-agent-client`,
`split-api-adapter-modules` — proved itself the same way: byte-identical item bodies, an identical
top-level item multiset, an identical test-name list. Nothing changed but the file an item lived in.

**That claim is unavailable here.** Extracting a fragment of a function's body changes control flow:
a `return` inside the fragment becomes a value the fragment produces and the caller returns. So this
change replaces the "nothing changed" argument with three narrower ones, stated and checked
individually in `design.md`:

1. **Every extracted fragment's exits are pure `return <expr>;` with no code after them in the
   parent's iteration.** Each maps to `Err(<the same expr>)` and a caller-side
   `Err(failure) => return failure`, which is observably the same event returned from the same
   logical point. Fragments whose exits are `continue`, or that mutate loop-carried state a caller
   cannot see, are **not** extracted.
2. **Every `?`, every error construction, and every side-effecting call keeps its exact text and its
   exact relative order** inside the moved fragment. The diff is reviewable as "this block, verbatim,
   behind a signature".
3. **Coverage is established per seam before the cut, not asserted after it.** Each seam names the
   tests that exercise it. Where a seam had none, a test for the *pre-split* behaviour is written
   first and confirmed green against the un-split function.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. No Tauri command, SQLite schema, adapter contract, wire format, event shape, or user-visible
runtime behaviour changes in either the desktop or Web runtime. No frontend file is touched. The
change sets `skip_specs: true`.

## Impact

- `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter/execution.rs` — the
  function shrinks; two helpers stay in this module because the tool-use loop is their only concern.
- `.../api_process_adapter/endpoint.rs` — new. Provider config, endpoint metadata, context capacity,
  credential, wire format, image-input capability.
- `.../api_process_adapter/prompt.rs` — gains the tool-catalog assembly, next to the
  `resolve_tool_catalog_with_code_intelligence` it already owns.
- `.../api_process_adapter/invocation.rs` — gains the per-round context analysis, next to the
  `record_context_snapshot` and `estimated_input_characters` it already owns.
- `.../api_process_adapter/interactive.rs` — gains the permission gate, next to the `await_approval`
  and `permission_action_and_resource` it already owns.
- `.../api_process_adapter/mod.rs` — one `mod` declaration and the re-exports `tests.rs` needs.
- `.../api_process_adapter/tests.rs` — **existing tests are not edited.** New characterization tests
  are appended for the three uncovered behaviours.
- `src-tauri/tests/architecture.rs` — the `execution.rs` path budget is lowered; the
  `agent_runtime/infrastructure` subtree budget is re-measured.
