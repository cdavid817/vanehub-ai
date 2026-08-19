## Context

`freeze-panic-shortcuts-in-production-code` left 35 production `unwrap()`/`expect()` sites behind a file-level whitelist in 11 files. Re-measured on this branch with the allows temporarily removed, using the gate's own invocation:

```
cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins -- -D clippy::unwrap_used -D clippy::expect_used
```

35 errors, in exactly the 11 files and exactly the per-file counts the freeze change recorded. Nothing drifted since it merged, so its inventory is still the authoritative one.

## Goals / Non-Goals

**Goals:**

- Remove the whitelist entry from every file where the panic can be eliminated without inventing a fake error channel.
- Keep the success path byte-identical. This change is about what happens on the branch that should never be taken.
- Where an entry survives, leave a comment that says *why panicking is correct*, not *when someone will get around to it*.

**Non-Goals:**

- Reaching zero for its own sake. Four of the five paths below produce a strictly better program; the fifth (`Result` a caller can only unwrap) produces a worse one, and is not used.
- Touching test code. The gate exempts it by design and `--lib` never compiles `#[cfg(test)]` modules — which is why the 11 files show hundreds of `unwrap()`s to `grep` but only 35 to clippy.
- Introducing a new error enum where a context already has one. Every context involved already does.
- Adding `[lints]` to `Cargo.toml`. The freeze change's reasoning is unchanged: it would turn ~9,560 test sites into hard errors under `--all-targets`.

## Decisions

### Prefer making the invariant unrepresentable over asserting it

Two files hold a panic that exists only because a *check* and its *use* are separated by a type the compiler cannot connect.

**`service.rs` — 11 of 12 sites.** `SkillService` holds five independent `Option` dependencies. `system_reconciliation_ready()` returns `bool` after testing all five, and every downstream user then re-reaches for each one with `.expect("checked by system_reconciliation_ready")`. The comment is accurate and the code is correct — but the correctness is maintained by hand across ~1,000 lines, and nothing stops a sixth dependency from being added to the struct and forgotten in the predicate.

Replacing the predicate with a borrowed bundle:

```rust
struct SystemReconciliation<'a> { /* five &-fields */ }

fn system_reconciliation(&self) -> Option<SystemReconciliation<'_>> {
    Some(SystemReconciliation {
        effective_catalog: self.effective_catalog.as_ref()?,
        /* ... */
    })
}
```

checks the five once, and hands downstream code references it cannot fail to dereference. All 11 sites disappear as a consequence of the restructure rather than being individually rewritten, and adding a sixth dependency becomes a compile error instead of a latent panic.

**`graph.rs` — all 3 sites.** `outgoing.get_mut(k).expect("validated predecessor")` where `k`'s presence was established by a `contains_key` a few lines above. `BTreeMap`'s entry API expresses the same operation without the failure case at all (`entry(k).or_default()`), so the assertion is not replaced — it is deleted along with the possibility it guarded.

*Alternative rejected — add an `Internal`/`Unreachable` variant to `GraphValidationError`*: the enum is part of a domain API whose variants are matched exhaustively by callers and asserted on in tests. Adding a variant that can never be constructed pushes a dead match arm onto every consumer to describe a state that cannot occur.

### `debug_assert!` plus a graceful fallback, where no caller could act on an error

Used where the failure is genuinely impossible *and* the function has no error channel that would mean anything to its caller. The shape is uniform:

```rust
let Some(value) = fallible_thing() else {
    debug_assert!(false, "<why this cannot happen>");
    return <conservative fallback>;
};
```

Loud in dev, test and CI — `cargo test` builds with `debug_assertions` on, so any of these firing fails the suite exactly as the `expect()` did. Silent-but-safe in release, where the alternative was aborting the process.

The fallback value is chosen per site to be the conservative direction, never merely the convenient one:

| Site | Fallback | Why that direction |
|---|---|---|
| `runner_registry.rs::capabilities()` | all-false capabilities | Under-reporting capability degrades features; over-reporting invites callers into unsupported paths. |
| `scheduled_tasks.rs::days_in_month()` | `28` | Valid in every month of every year, so the caller's `while candidate.day() != …` loop still terminates. A larger guess can spin. |
| `search_service.rs` / `indexing_service.rs::new()` | direct construction | The two arguments are compile-time constants; the validation call is removed and `debug_assert!` keeps it honest. |
| `code_redaction.rs` | fail closed — see below | Pass-through would leak the secrets the module exists to remove. |

### Mutex poisoning is recovered, not asserted away — reusing the pattern already in this repo

Eight of the 35 (`approval_broker.rs` 6, `hook_bridge_wait_registry.rs` 2) are `Mutex::lock().expect("… poisoned")`. A poisoned lock means *another* thread already panicked while holding it. Aborting a second thread in response does not recover anything; it converts one failure into two.

This repository already settled on the recovery form — `.unwrap_or_else(|poisoned| poisoned.into_inner())` appears in 10 places across 7 files, including `retrieval/api.rs`, `skill_tools/application/registry.rs`, `platform/network/proxy.rs`, and `agent_runtime/infrastructure/memory_directory.rs`. This change reuses it rather than inventing a shape, and pairs it with `debug_assert!` so a poisoning that happens under test is not swallowed.

The guarded data in both files is a `HashMap` mutated only by `insert`/`remove`/`get`, none of which can panic mid-operation and leave it torn, so `into_inner()` returns a structurally sound map.

*Alternative rejected — propagate a `PoisonError` as a typed `Result`*: `list_pending()` and `get_pending()` return plain values consumed by command handlers that would have nothing to do with the error but log it, and `HookWaitRegistry::resolve()` returns `bool` by deliberate design (it mirrors `AgentRuntimeApi::resolve_tool_approval`). Threading a new error through them buys a worse signature for an outcome no caller can act on.

### `Result` propagation only where a boundary above already handles errors

Two sites, both where the surrounding function is *already* fallible and the panic is the odd one out:

- **`transaction.rs::begin()`** — every sibling method in the file already does `.lock().map_err(lock_error)?`; `begin()` alone expects. It becomes `Result<SkillFilesystemTransaction, SkillApplicationError>` using the file's own existing `lock_error` helper. This is the one site whose signature changes.
- **`hook_bridge_discovery.rs::write_discovery_file()`** — already returns `io::Result<()>`, and the `serde_json::to_string` is the only step in it that cannot report failure. Mapping the serde error into `io::ErrorKind::InvalidData` costs one line and no new type.

### `code_redaction.rs`'s fallback must be fail-closed

Six `Regex::new(<string literal>).expect(...)` behind `OnceLock`. The inputs are literals, so this is the purest "cannot fail" case in the whole set — and the most dangerous one to hand a naive fallback.

If an expression cannot be built and `redact_code` simply skips it, the function returns text that *looks* redacted and is not, and both callers (`code_chunker.rs`, `code_index_repository.rs`) write that text into the retrieval index, where the agent will later read it back. The graceful-degradation instinct produces a silent credential leak.

So the accessors return `Option<&'static Regex>`, and `redact_code` treats an unavailable expression as a redaction *failure* rather than a no-op: the affected content is replaced wholesale with the redaction marker rather than passed through. Combined with `debug_assert!`, a malformed literal is caught the moment a developer runs the tests, which is the only realistic way one ever gets introduced.

*Alternative rejected — make `redact_code` return `Result` and let the callers decide*: defensible, and it was the first design considered. Rejected because both callers are inside batch indexing loops whose only sensible reaction is "drop this chunk" — which is what fail-closed already does, without a new error type propagating through two infrastructure layers to reach the same outcome.

### An entry may legitimately survive

If a site turns out to have no fallback that is both safe and honest, its file keeps `#![allow(...)]` and the comment is rewritten from a deferral ("N pre-existing sites, retired by <change>") into a justification ("panicking is correct here because …"). That outcome is reported, not hidden — it is why the `repository-governance` delta exists.

## Risks / Trade-offs

- **A release-build fallback masks a bug that `expect()` would have surfaced** → Real. Mitigated by `debug_assert!` at every one of these sites: tests and CI build with assertions on, so anything reachable in practice fails loudly there first. The trade is deliberate — a desktop app aborting mid-session is a worse outcome for the operator than a degraded one.
- **`service.rs` is 3,064 lines and the restructure touches ~1,000 of them** → The change is mechanical (predicate → destructured bundle) and every touched site is covered by the file's existing tests, which must pass unedited. No Rust file-size gate applies (`max-lines` is ESLint, TypeScript only).
- **`transaction.rs::begin()`'s signature change ripples** → Contained: `begin()` is `pub(super)`, so the blast radius is one module, and its callers already return `Result<_, SkillApplicationError>`.
- **Two unarchived changes now modify the same `repository-governance` requirement** → Recorded in proposal.md as an ordering dependency. The freeze change is already merged in code, so it archives first; if it does not, the deltas are reconciled by hand.

## Verification strategy

Per file, in this order — the middle step is the one that matters:

1. `cargo test --manifest-path src-tauri/Cargo.toml` before touching the file, to establish that its tests pass on this branch.
2. Convert, then run the **same tests unedited**. A test that needs editing to accommodate the change is evidence that observable behavior moved, not just panic style, and is treated as a finding to report rather than a test to fix. The one exception is a test that explicitly asserted the panic (`#[should_panic]`), where an edit is the correct and expected consequence.
3. Remove that file's `#![allow(...)]` and run `npm run native:panic:check`. It fails naming the file if any site was missed, so the gate itself is the completeness check — no manual counting.

Full suite at the end: `cargo test`, `npm run native:panic:check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `npm run architecture:check`, and both `openspec validate` invocations.
