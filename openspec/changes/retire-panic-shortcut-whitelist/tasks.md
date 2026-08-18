## 1. Baseline

- [ ] 1.1 Re-measure with the allows temporarily removed, using the gate's own invocation, and confirm the per-file inventory against the freeze change's record
- [ ] 1.2 `cargo test --manifest-path src-tauri/Cargo.toml` passes on the untouched branch, and the test count is recorded so it can be compared afterwards

## 2. Domain layer first — the least defensible placement

- [ ] 2.1 `task_orchestration/domain/graph.rs` (3): replace `get_mut(...).expect(...)` with the `BTreeMap` entry API so the validated-key invariant is structural, not asserted
- [ ] 2.2 `retrieval/domain/code_redaction.rs` (6): accessors return `Option<&'static Regex>`; `redact_code` fails **closed** on an unavailable expression rather than passing text through unredacted
- [ ] 2.3 Both files' existing tests pass unedited; remove each `#![allow(...)]` and confirm `npm run native:panic:check`

## 3. The six single-site files

- [ ] 3.1 `retrieval/application/search_service.rs`: `new()` constructs directly from the two constant arguments, with `debug_assert!` on the scope/kind pair
- [ ] 3.2 `retrieval/application/indexing_service.rs`: same shape as 3.1
- [ ] 3.3 `permissions/infrastructure/hook_bridge_discovery.rs`: map the serde error into the `io::Result` the function already returns
- [ ] 3.4 `tooling/skills/infrastructure/filesystem/transaction.rs`: `begin()` returns `Result<_, SkillApplicationError>` via the file's existing `lock_error` helper, matching every sibling method
- [ ] 3.5 `sessions/infrastructure/scheduled_tasks.rs`: `days_in_month` falls back to 28 so the caller's loop still terminates
- [ ] 3.6 `agent_runtime/infrastructure/runner_registry.rs`: `capabilities()` falls back to all-false capabilities — the trait returns a plain value, so `Result` is not available
- [ ] 3.7 Each file's `#![allow(...)]` removed and `npm run native:panic:check` confirmed after each

## 4. The mutex-poisoning pair

- [ ] 4.1 `permissions/application/approval_broker.rs` (6): recover the guard with the repository's existing `unwrap_or_else(|poisoned| poisoned.into_inner())` pattern, via one private helper rather than six repetitions
- [ ] 4.2 `permissions/infrastructure/hook_bridge_wait_registry.rs` (2): same pattern
- [ ] 4.3 Confirm the guarded maps cannot be left torn by a panic mid-operation, so `into_inner()` returns sound state
- [ ] 4.4 Both files' `#![allow(...)]` removed and the gate confirmed

## 5. `skills/application/service.rs` — the 12

- [ ] 5.1 Introduce a borrowed `SystemReconciliation` bundle returned as `Option<...>`, replacing the `system_reconciliation_ready() -> bool` predicate
- [ ] 5.2 Thread it through the 11 `expect("checked by system_reconciliation_ready")` sites so each dependency is destructured once instead of re-reached-for
- [ ] 5.3 Convert the remaining constant-`SkillLocation` site
- [ ] 5.4 File's tests pass unedited; `#![allow(...)]` removed and the gate confirmed

## 6. Governance

- [ ] 6.1 For any file whose entry survives, rewrite its comment from a deferral into a justification naming why panicking is correct
- [ ] 6.2 Confirm no `#![allow(clippy::unwrap_used, clippy::expect_used)]` remains in a file that no longer needs one

## 7. Verification

- [ ] 7.1 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged test count, and every test that needed editing is listed with the reason
- [ ] 7.2 `npm run native:panic:check` passes
- [ ] 7.3 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes
- [ ] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` passes
- [ ] 7.5 `npm run architecture:check` passes
- [ ] 7.6 `openspec validate retire-panic-shortcut-whitelist --strict` and `openspec validate --specs --strict` pass
- [ ] 7.7 Record which of the 11 files were retired and which kept their entry, with the reason for each survivor
