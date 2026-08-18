## Context

See proposal.md — Why for the measurements and motivation.

The file is not 79 self-contained migrations. Its actual composition:

| Region | Lines | Content |
|---|---:|---|
| `migrate()` orchestration | ~6-490 | 79 ordered `apply_migration` / `apply_transactional_migration` calls |
| `EXPECTED_MIGRATIONS` | 763-843 | the 79-entry ground-truth registry |
| Local migration bodies | scattered | **24** `fn apply_*(conn: &Connection)` definitions |
| Application helpers | 1229-1300 | `apply_migration`, `apply_transactional_migration` |
| Inline `mod tests` | 1425-2301 | **877 lines (38% of the file)** |

The other **56** migrations do not have bodies here at all — they delegate to `apply_schema` functions owned by the bounded context that owns the table, e.g. `crate::contexts::tooling::skills::infrastructure::apply_schema`. That decomposition already happened.

## Goals / Non-Goals

**Goals:**

- Make the file a version-number picker reads before choosing a number small enough to read in one sitting.
- Get the 877-line test module out of the library compilation surface.
- Preserve the existing collision defenses exactly, since they are the reason a collision is caught at all.

**Non-Goals:**

- Renumbering, reordering, merging, or rewriting any migration. Version numbers are load-bearing against databases already in the field.
- Relocating the 24 local bodies to their owning contexts. That is the right long-term direction — it is what the other 56 already did — but doing it here would mix a mechanical move with 24 ownership judgments.
- Adding a registry or a density/duplication test. Both already exist.

## Decisions

### Split by role, not one file per migration

The optimization ticket proposed `m0001_init.rs … m0052_xxx.rs`, one file per migration. Against the actual file that would produce 56 files containing a single delegating call each — navigation cost with nothing moved into them. The split is instead:

| File | Content |
|---|---|
| `migrations/mod.rs` | `migrate()`, `EXPECTED_MIGRATIONS`, `apply_migration`, `apply_transactional_migration`, density verification |
| `migrations/inline_schema.rs` | the 24 local `apply_*` bodies |
| `migrations/tests.rs` | the inline `mod tests` |

`mod.rs` then holds the ordered call list and the registry side by side — exactly the two things someone picking a version number needs — with the SQL bodies out of the way.

### Preserve `EXPECTED_MIGRATIONS` and its density test verbatim

The ticket asked for "a test asserting migration numbers are contiguous and non-duplicated". `EXPECTED_MIGRATIONS` plus `migration_sequence_is_dense_and_matches_expected` plus the runtime `assert_migration_history_is_dense` already do this, and the registry carries a comment recording the real incident that motivated them. Rewriting them during a file move would put the one guard that catches collisions into the same diff as the move that could introduce one.

### The 24 inline bodies move as one block, not 24 decisions

They are the migrations whose owning context either did not exist when they were written or has not claimed them. Grouping them in one file makes that status visible and leaves a clean seam for a later change to relocate them one at a time, matching how the other 56 already live in their contexts.

### The path budget is satisfied by absence; the subtree budget is what actually binds

`migrations.rs` ceases to exist, which `freeze-large-file-line-budgets` treats as satisfied. The `platform/database` subtree budget of 2,914 continues to bound the replacement, so a split that duplicates instead of moving still fails. Per-file module boilerplate will push the aggregate up slightly; that raise is explicit and reasoned in the same commit.

## Risks / Trade-offs

- **A migration is silently dropped during the move** → `EXPECTED_MIGRATIONS` is the control: `migration_sequence_is_dense_and_matches_expected` compares it against what `migrate()` actually applies, so a dropped call fails the test rather than shipping.
- **The six hard-coded `79` assertions are invisible to the compiler and to clippy** → They are enumerated as explicit tasks rather than left to be discovered: `migrations.rs:480`, `migrations.rs:1747`, `mod.rs:261`, `mod.rs:323`, `migration_fixture_tests.rs:25`, `migration_fixture_tests.rs:487`. This change adds no migration, so all six must keep their current value; any of them changing is a defect in the move.
- **Every worktree on this machine shares one `ai.vanehub.app` SQLite database** → Do not launch the desktop app from another branch while this lane is open. This change introduces no new version number, so it cannot itself collide, but a concurrent branch that does will present as an opaque "no such table" crash that looks like this lane's fault.
- **A reviewer cannot tell a moved SQL body from an edited one** → The migration fixture tests run the full 1..=79 upgrade path against a fixture database; a mutated body fails there.

## Migration Plan

No data migration and no deployment step: the applied schema is unchanged, so a database at any version upgrades along a byte-identical path. Rollback is `git revert`. The fixture test that replays versions 1 through 79 is the evidence that the upgrade path did not move.
