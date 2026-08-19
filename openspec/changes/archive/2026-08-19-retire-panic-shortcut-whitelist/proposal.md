## Why

`freeze-panic-shortcuts-in-production-code` built the gate (`npm run native:panic:check`) and measured the real scope: **35 `unwrap()`/`expect()` sites in production Rust across 11 files**, not the ~3,700 the optimization ticket estimated. Each of the 11 files carries a file-level `#![allow(clippy::unwrap_used, clippy::expect_used)]` naming its count and naming *this* change as the one expected to retire it.

That change deliberately did not fix anything. This one does the error-handling work.

The list is short enough to finish, and finishing it is the point: AGENTS.md restricts `unwrap()`/`expect()` to test code, and a whitelist that never shrinks is just the blanket exemption the freeze change was written to avoid.

## What Changes

Every site is routed down exactly one of three paths. Which path is a per-site judgment, not a mechanical rewrite:

1. **Structural — make the invariant unrepresentable.** The panic is asserting something the type system could enforce instead. Preferred wherever it applies, because it removes the site *and* the class of bug.
2. **`debug_assert!` plus a graceful fallback.** A genuine invariant with no caller-visible recovery. Loud in dev and test, degrades safely in release, never aborts the process.
3. **Typed `Result` propagation.** The operation can actually fail — I/O, serialization, a lock that could be poisoned — and there is already a boundary above it that handles errors.

A site is **not** converted merely to make the count reach zero. Propagating a `Result` that the caller can only `.unwrap()` one layer up is not progress; where that is the only available shape, the entry stays whitelisted with a sharper comment that states why panicking is correct rather than deferring the work again.

### Per-file plan

| File | Sites | Path | Shape |
|---|---:|---|---|
| `tooling/skills/application/service.rs` | 12 | 1 + 2 | 11 sites are `Option::expect("checked by system_reconciliation_ready")` — a boolean predicate the type system cannot see. Replace with a borrowed bundle returned as `Option<...>`, so the five dependencies are checked once and destructured. 1 site is a constant `SkillLocation`. |
| `retrieval/domain/code_redaction.rs` | 6 | 2 | `Regex::new(<literal>).expect(...)`. Static input, cannot fail. Fallback must be **fail-closed** — see below. |
| `permissions/application/approval_broker.rs` | 6 | 2 | `Mutex::lock().expect("poisoned")`. |
| `task_orchestration/domain/graph.rs` | 3 | 1 | `BTreeMap::get_mut().expect(...)` on keys validated a few lines earlier. The entry API makes them infallible. |
| `permissions/infrastructure/hook_bridge_wait_registry.rs` | 2 | 2 | `Mutex::lock().expect("poisoned")`. |
| `tooling/skills/infrastructure/filesystem/transaction.rs` | 1 | 3 | `Mutex::lock().expect(...)` in `begin()` — every sibling method in the same file already does `.lock().map_err(lock_error)?`. `begin()` is the outlier. |
| `sessions/infrastructure/scheduled_tasks.rs` | 1 | 2 | `NaiveDate::from_ymd_opt(...).expect("valid month")` in `days_in_month`. |
| `retrieval/application/search_service.rs` | 1 | 2 | `new()` calls `new_scoped()` with two **literal** constants and expects the validation to pass. |
| `retrieval/application/indexing_service.rs` | 1 | 2 | Same shape as `search_service.rs`. |
| `permissions/infrastructure/hook_bridge_discovery.rs` | 1 | 3 | `serde_json::to_string(...).expect(...)` inside a function that already returns `io::Result<()>`. |
| `agent_runtime/infrastructure/runner_registry.rs` | 1 | 2 | Trait method `capabilities()` returns a plain value, so `Result` is not available without changing the `AgentRunner` trait. |

### `code_redaction.rs` needs its fallback chosen deliberately

Six `Regex::new(<string literal>).expect(...)`. Textbook path 2 — except the usual "graceful fallback" for a redaction routine is dangerous: if an expression is unavailable and the function returns the input **unredacted**, the failure mode is a silent secret leak into the retrieval index. The whole module exists to prevent exactly that.

So the fallback here is fail-closed, not pass-through: a redaction expression that cannot be built is treated as *matching everything it was meant to catch*, and `redact_code` reports that it degraded rather than silently returning weaker output. `debug_assert!` still fires in dev and test, where a malformed literal would be a genuine authoring bug caught immediately.

### The two `domain` files come first

`retrieval/domain/code_redaction.rs` and `task_orchestration/domain/graph.rs` hold 9 of the 35, and the freeze change's whitelist comments call out that a panic shortcut in a pure domain layer is this codebase's least defensible placement. They are cleared first.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `repository-governance`: the whitelist mechanism's meaning changes. The requirement introduced by `freeze-panic-shortcuts-in-production-code` says an exemption records "the reason and the work expected to retire it" — phrasing that assumes every entry is temporary debt. After this change, any entry that remains is one where panicking was reviewed and judged **correct**, and there is no work expected to retire it. The spec must be able to describe that state, or it is false about the code.

### Not a capability change

No behavioral capability spec is modified, and this is a deliberate finding rather than an omission:

- Every conversion preserves the success path **exactly**. The observable difference appears only in a branch that is unreachable in practice (a literal regex failing to compile, a `BTreeMap` key that was validated three lines earlier) or that can only be entered after some *other* bug has already panicked (mutex poisoning).
- Several of these sites are reachable from Tauri commands — `approval_broker` via the permissions command handlers, `service.rs` via the Skill commands. Reachability was the thing worth checking. But no caller can depend on the panic: today it aborts the operation with no typed error at all, so there is no contract to preserve. Turning an abort into a typed `Result` at a boundary that already returns `Result<T, String>` per AGENTS.md *adds* a defined outcome where there was none.
- No specified capability documents "this operation panics" as its behavior. `grep` over `openspec/specs/` finds `repository-governance` as the only spec mentioning panics at all, and it is about enforcement, not runtime semantics.

## Ordering dependency

`freeze-panic-shortcuts-in-production-code` is implemented and merged but **not archived**, so its `repository-governance` delta has not yet been folded into `openspec/specs/repository-governance/spec.md`. This change's delta is written against the post-freeze text and therefore assumes the freeze change archives first. If the order slips, the two deltas both modify `Existing source constraints remain enforced` and must be reconciled by hand at archive time.

## Impact

- Eleven Rust source files under `src-tauri/src/contexts/` — real error-handling changes, not annotations.
- No new dependency: `thiserror` is already in `src-tauri/Cargo.toml` and most contexts already own an error enum (`SkillApplicationError`, `PermissionsApplicationError`, `RetrievalError`, `GraphValidationError`). This change reuses them and introduces no new context-level error type unless a file has no existing channel.
- `src-tauri/Cargo.toml` unchanged — the freeze change's reasoning for keeping `[lints]` out of it still holds.
- No frontend change, no adapter-parity change, no migration.
