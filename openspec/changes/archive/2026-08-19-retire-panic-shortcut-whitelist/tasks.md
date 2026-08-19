## 1. Baseline

- [x] 1.1 Re-measure with the allows temporarily removed, using the gate's own invocation, and confirm the per-file inventory against the freeze change's record — 35 sites, 11 files, per-file counts identical, nothing drifted
- [x] 1.2 `cargo test --manifest-path src-tauri/Cargo.toml` passes on the untouched branch, and the test count is recorded so it can be compared afterwards — 3534 lib tests plus the integration targets

## 2. Domain layer first — the least defensible placement

- [x] 2.1 `task_orchestration/domain/graph.rs` (3): replace `get_mut(...).expect(...)` with the `BTreeMap` entry API so the validated-key invariant is structural, not asserted
- [x] 2.2 `retrieval/domain/code_redaction.rs` (6): accessors return `Option<&'static Regex>`; `redact_code` fails **closed** on an unavailable expression rather than passing text through unredacted
- [x] 2.3 Both files' existing tests pass unedited; remove each `#![allow(...)]` and confirm `npm run native:panic:check`

## 3. The six single-site files

- [x] 3.1 `retrieval/application/search_service.rs`: `new()` constructs directly from the two constant arguments, with `debug_assert!` on the scope/kind pair
- [x] 3.2 `retrieval/application/indexing_service.rs`: same shape as 3.1
- [x] 3.3 `permissions/infrastructure/hook_bridge_discovery.rs`: map the serde error into the `io::Result` the function already returns
- [x] 3.4 `tooling/skills/infrastructure/filesystem/transaction.rs`: `begin()` returns `Result<_, SkillApplicationError>` via the file's existing `lock_error` helper, matching every sibling method
- [x] 3.5 `sessions/infrastructure/scheduled_tasks.rs`: `days_in_month` falls back to 28 so the caller's loop still terminates
- [x] 3.6 `agent_runtime/infrastructure/runner_registry.rs`: `capabilities()` falls back to all-false capabilities — the trait returns a plain value, so `Result` is not available
- [x] 3.7 Each file's `#![allow(...)]` removed and `npm run native:panic:check` confirmed after each

## 4. The mutex-poisoning pair

- [x] 4.1 `permissions/application/approval_broker.rs` (6): recover the guard with the repository's existing `unwrap_or_else(|poisoned| poisoned.into_inner())` pattern, via one private helper rather than six repetitions
- [x] 4.2 `permissions/infrastructure/hook_bridge_wait_registry.rs` (2): same pattern
- [x] 4.3 Confirm the guarded maps cannot be left torn by a panic mid-operation, so `into_inner()` returns sound state — both are only ever touched by `insert`/`remove`/`get`
- [x] 4.4 Both files' `#![allow(...)]` removed and the gate confirmed

## 5. `skills/application/service.rs` — the 12

- [x] 5.1 Introduce a borrowed `SystemReconciliation` bundle returned as `Option<...>`, replacing the `system_reconciliation_ready() -> bool` predicate
- [x] 5.2 Thread it through the 11 `expect("checked by system_reconciliation_ready")` sites so each dependency is destructured once instead of re-reached-for
- [x] 5.3 Convert the remaining constant-`SkillLocation` site — added the infallible `SkillLocation::global()`
- [x] 5.4 File's tests pass unedited; `#![allow(...)]` removed and the gate confirmed

## 6. Governance

- [x] 6.1 For any file whose entry survives, rewrite its comment from a deferral into a justification naming why panicking is correct — **not needed: no entry survived.** All 35 sites were convertible without inventing a fake error channel
- [x] 6.2 Confirm no `#![allow(clippy::unwrap_used, clippy::expect_used)]` remains in a file that no longer needs one — zero remain in `src-tauri/src/`

## 7. Verification

- [x] 7.1 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged test count, and every test that needed editing is listed with the reason — **no test needed editing**
- [x] 7.2 `npm run native:panic:check` passes
- [x] 7.3 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes
- [x] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` passes
- [x] 7.5 `npm run architecture:check` passes
- [x] 7.6 `openspec validate retire-panic-shortcut-whitelist --strict` and `openspec validate --specs --strict` pass
- [x] 7.7 Record which of the 11 files were retired and which kept their entry, with the reason for each survivor — all 11 retired; see the findings below

## Findings worth carrying forward

- **Removing a panic revealed dead code.** `SearchService::new_scoped` and `IndexingService::new_scoped` had no caller anywhere once `new()` stopped delegating through them — the panic shortcut was the only thing keeping them reachable. Annotated rather than deleted, following the `ResolvedApproval` precedent in `approval_broker.rs`.
- **`graph.rs` still has `Index`-based panics.** `ordinals[id]` and `outgoing[&id]` panic on a missing key just as `expect()` did, but they are `clippy::indexing_slicing`, which this gate does not enable. The file is free of `unwrap`/`expect`, not free of panics. Enabling `indexing_slicing` on `--lib --bins` is a candidate follow-up, and would want its own measurement first.
- **The gate does not see `#[cfg(test)]` modules at all.** `--lib` builds without `cfg(test)`, which is why these 11 files show hundreds of `unwrap()`s to `grep` but only 35 to clippy. Worth knowing before anyone re-measures with `grep`.
