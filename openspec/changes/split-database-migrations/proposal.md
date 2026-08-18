## Why

`src-tauri/src/platform/database/migrations.rs` holds **79 migrations in 2,301 lines**. Every new migration recompiles the whole file, and reviewing one means scrolling past all 78 predecessors. The file is 79% of the entire `platform/database/` subtree (2,301 of 2,914 lines).

The number that makes this urgent is not the line count but the collision surface. `EXPECTED_MIGRATIONS` already carries a comment recording that a version-number collision has *already happened* across shared local databases — every worktree on this machine shares one `ai.vanehub.app` database, and a duplicate version is silently skipped by the version-gated `apply_migration`, surfacing later as an opaque "no such table" crash. Picking a free version number today means reading a 2,301-line file, and the reader who gets it wrong does not find out at compile time.

## What Changes

- Convert `migrations.rs` into a directory module `migrations/`, one file per migration, with `mod.rs` retaining `EXPECTED_MIGRATIONS`, `migrate()`, and the existing density verification.
- Keep `EXPECTED_MIGRATIONS`, the `migration_sequence_matches_expected` test that guards it, and the runtime `assert_migration_history_is_dense` exactly as the single source of truth. **The optimization ticket proposed adding a registry and a density-and-duplication test; both already exist** — this change preserves them rather than duplicating them.
- Preserve every one of the six hard-coded `79` version assertions spread across three files, none of which the compiler or clippy can see.
- **No schema, migration order, migration content, or upgrade-path change.** The migration a database at version N applies to reach N+1 is byte-identical before and after.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure file-organization refactor. The `native-runtime-architecture` requirements "Versioned SQLite migrations" and "Migration application is transactional with startup density verification" describe behavior this change deliberately preserves unchanged, so neither needs a delta. The change sets `skip_specs: true`.

## Impact

- `src-tauri/src/platform/database/migrations.rs` — becomes `migrations/mod.rs`; the 79 migration bodies move to per-migration files.
- `src-tauri/src/platform/database/mod.rs` — holds two of the six hard-coded version assertions (lines 261 and 323); they must keep passing.
- `src-tauri/src/migration_fixture_tests.rs` — holds two more (`(1..=79).collect()` at line 25, and a `DELETE FROM schema_migrations WHERE version = 79` at line 487).
- `src-tauri/tests/architecture.rs` — the `migrations.rs` path budget becomes satisfied-by-absence; the `platform/database` subtree budget continues to bind and is expected to need a small explicit raise for per-file module boilerplate.
- No frontend file is touched. No Tauri command signature or SQLite schema changes.
