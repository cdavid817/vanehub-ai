## 1. Capture the baseline

- [ ] 1.1 Record the physical line counts of `migrations.rs` and the `src-tauri/src/platform/database/` aggregate
- [ ] 1.2 Record the current value of all six hard-coded version assertions so any drift is detectable: `migrations.rs:480`, `migrations.rs:1747`, `mod.rs:261`, `mod.rs:323`, `migration_fixture_tests.rs:25`, `migration_fixture_tests.rs:487`
- [ ] 1.3 Capture the sorted test-name list for the `migrations` module via `cargo test --lib -- --list`, for the post-move comparison

## 2. Create the directory module

- [ ] 2.1 Convert `migrations.rs` into `migrations/mod.rs`, keeping `migrate()`, `EXPECTED_MIGRATIONS`, `apply_migration`, `apply_transactional_migration`, and the density verification in it
- [ ] 2.2 Confirm `super::DatabaseError` and the other existing imports still resolve from the new module depth, adjusting import paths only — no item may change visibility beyond what the move requires

## 3. Move the inline bodies and tests

- [ ] 3.1 Move the 24 local `fn apply_*(conn: &Connection)` migration bodies to `migrations/inline_schema.rs` verbatim, re-exporting or importing them into `mod.rs` so the `migrate()` call sites are unchanged
- [ ] 3.2 Move the inline `mod tests` (lines 1425-2301) to `migrations/tests.rs`, declared as `#[cfg(test)] mod tests;`
- [ ] 3.3 Confirm the 56 delegating calls to context-owned `apply_schema` functions are untouched
- [ ] 3.4 Confirm no migration body's SQL text changed, by diffing the moved bodies against the originals

## 4. Prove the upgrade path did not move

- [ ] 4.1 `migration_sequence_is_dense_and_matches_expected` passes — this is the guard that a migration was not dropped or renumbered
- [ ] 4.2 The migration fixture tests replaying versions 1 through 79 pass unchanged
- [ ] 4.3 Re-check all six version assertions from 1.2 still read `79` — this change adds no migration, so any change to them is a defect
- [ ] 4.4 Re-capture the sorted test-name list and assert it is byte-identical to the baseline from 1.3
- [ ] 4.5 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged total test count

## 5. Update budgets and verify

- [ ] 5.1 Confirm the now-absent `migrations.rs` path budget is treated as satisfied by the native budget test, and remove the stale registry entry
- [ ] 5.2 Measure the `platform/database` subtree delta; raise the subtree budget by exactly the module-boilerplate amount with a stated reason, or stop and find the cause if the delta is larger than boilerplate explains
- [ ] 5.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `cargo check --manifest-path src-tauri/Cargo.toml` pass
- [ ] 5.4 `npm run architecture:check` passes
- [ ] 5.5 `openspec validate split-database-migrations --strict` and `openspec validate --specs --strict` pass
