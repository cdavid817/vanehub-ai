## 1. Capture the baseline

- [x] 1.1 Record the physical line counts of `migrations.rs` and the `src-tauri/src/platform/database/` aggregate
      — `migrations.rs` 2,301; subtree 2,914 (`legacy_plan_schema.rs` 205 + `migrations.rs` 2,301 + `mod.rs` 408)
- [x] 1.2 Record the current value of all six hard-coded version assertions so any drift is detectable: `migrations.rs:480`, `migrations.rs:1747`, `mod.rs:261`, `mod.rs:323`, `migration_fixture_tests.rs:25`, `migration_fixture_tests.rs:487`
      — `79,` / `assert_eq!(migration_state, (78, 79));` / `assert_eq!(migration_count, 79);` / `assert_eq!(migration_count, 79);` / `(1..=79).collect()` / `DELETE FROM schema_migrations WHERE version = 79;`
- [x] 1.3 Capture the sorted test-name list for the `migrations` module via `cargo test --lib -- --list`, for the post-move comparison
      — 18 tests under `platform::database::migrations::tests::`; 3,543 lib tests in total

## 2. Create the directory module

- [x] 2.1 Convert `migrations.rs` into `migrations/mod.rs`, keeping `migrate()`, `EXPECTED_MIGRATIONS`, `apply_migration`, `apply_transactional_migration`, and the density verification in it
      — `table_has_column` stays with them, since it is the helper those bodies and `repair_missing_stable_participant_schema` share and `platform::database` re-exports it
- [x] 2.2 Confirm `super::DatabaseError` and the other existing imports still resolve from the new module depth, adjusting import paths only — no item may change visibility beyond what the move requires
      — `mod.rs` keeps `super::DatabaseError` unchanged (its `super` is still `platform::database`). The two path adjustments forced by the extra level are `inline_schema.rs`'s `use super::{table_has_column, DatabaseError}` and one `super::super::legacy_plan_schema::apply_legacy_plan_schema`. The 25 moved bodies gain `pub(super)` — the minimum that lets `migrate()` still call them, and nothing wider

## 3. Move the inline bodies and tests

- [x] 3.1 Move the 24 local `fn apply_*(conn: &Connection)` migration bodies to `migrations/inline_schema.rs` verbatim, re-exporting or importing them into `mod.rs` so the `migrate()` call sites are unchanged
      — the actual census is **25** named local bodies, not 24; `mod.rs` imports them by name so every `migrate()` call site is textually unchanged
- [x] 3.2 Move the inline `mod tests` (lines 1425-2301) to `migrations/tests.rs`, declared as `#[cfg(test)] mod tests;`
- [x] 3.3 Confirm the 56 delegating calls to context-owned `apply_schema` functions are untouched
      — `migrate()` still issues exactly 79 `apply_migration`/`apply_transactional_migration` calls and 54 `crate::contexts::` references, byte-identical to the original
- [x] 3.4 Confirm no migration body's SQL text changed, by diffing the moved bodies against the originals
      — all 25 bodies compared line-by-line against the pre-split file: identical except the `pub(super)` prefix and the one `super::super::` depth fix. `migrate()`, `EXPECTED_MIGRATIONS`, and the two application helpers are byte-identical; `tests.rs` is the inline module dedented one level, with two calls reflowed by rustfmt as a result

## 4. Prove the upgrade path did not move

- [x] 4.1 `migration_sequence_is_dense_and_matches_expected` passes — this is the guard that a migration was not dropped or renumbered
      — the guard ships under the name `migration_sequence_matches_expected` (plus the runtime `assert_migration_history_is_dense`, covered by `density_check_rejects_a_missing_migration_row`); both pass, and `EXPECTED_MIGRATIONS` still holds exactly 79 entries against 79 `apply_migration`/`apply_transactional_migration` calls
- [x] 4.2 The migration fixture tests replaying versions 1 through 79 pass unchanged
      — all 13 `migration_fixture_tests::*` pass, including `empty_fixture_migrates_to_latest_schema`, `legacy_v1_fixture_upgrades_without_losing_records`, and `runner_projection_migration_preserves_legacy_runtime_evidence_and_local_rollback`
- [x] 4.3 Re-check all six version assertions from 1.2 still read `79` — this change adds no migration, so any change to them is a defect
      — all six still read `79`: `migrations/mod.rs:497`, `migrations/tests.rs:319`, `platform/database/mod.rs:261` and `:323` (unmoved), `migration_fixture_tests.rs:25` and `:487` (unmoved)
- [x] 4.4 Re-capture the sorted test-name list and assert it is byte-identical to the baseline from 1.3
      — `diff` is empty for both the 18 `platform::database::migrations::tests::*` names and the whole 3,543-name lib list
- [x] 4.5 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged total test count
      — 3,543 lib tests, same as the baseline. Four failures across two runs were all process-spawn/wall-clock tests (`playwright_sidecar`, `relay_stdio`, `managed_session_tests`, `mcp_relay_provider_invocations`) that pass when re-run in isolation; none touches SQLite. Concurrent sibling worktrees were building at the time

## 5. Update budgets and verify

- [x] 5.1 Confirm the now-absent `migrations.rs` path budget is treated as satisfied by the native budget test, and remove the stale registry entry
      — `path_budget_diagnostic` returns `None` for a missing path, which `line_budget_detector_treats_a_missing_path_as_satisfied_while_its_subtree_still_binds` already pins; the entry is removed, leaving 4 in `NATIVE_PATH_BUDGETS` and `present_paths > 0` still meaningful
- [x] 5.2 Measure the `platform/database` subtree delta; raise the subtree budget by exactly the module-boilerplate amount with a stated reason, or stop and find the cause if the delta is larger than boilerplate explains
      — 2,914 → 2,965, a delta of exactly +51, raised to 2,965 with the accounting recorded in `architecture.rs`: +29 module headers, +28 rustfmt wrapping 14 `pub(super) fn` signatures past 100 columns, −5 for the vanished `mod tests { … }` wrapper and its reflow, −1 blank separator. The budget binds at the measurement with no headroom
- [x] 5.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `cargo check --manifest-path src-tauri/Cargo.toml` pass
      — all three exit 0; clippy emits no warnings at all
- [x] 5.4 `npm run architecture:check` passes
      — exit 0 across all five stages; `oversized_native_paths_stay_within_their_recorded_line_budgets` and the four budget-detector tests pass, 40/40 in the `architecture` target
- [x] 5.5 `openspec validate split-database-migrations --strict` and `openspec validate --specs --strict` pass
      — change valid; specs 138 passed, 0 failed
