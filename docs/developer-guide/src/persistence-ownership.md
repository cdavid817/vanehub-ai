# Persistence ownership

Which context owns which table, how migrations are numbered and applied, and the key constants of the database layer.

Writing logs, redaction, and trace correlation are in [Unified logging](unified-logging.md).

## SQLite ownership

SQLite is accessed only from Rust infrastructure. Migrations have a global order, but each schema and repository belongs to a bounded context. A foreign-key reference does not grant one context permission to query another context's tables directly.

Migration changes require:

- a versioned migration;
- clean-database and upgrade-path coverage;
- explicit row-to-domain mapping;
- compatibility with current fixtures;
- no `unwrap()` or `expect()` across production command boundaries.

## SQLite ownership and migrations

The database is one application-owned SQLite file. Connections, migration orchestration, and seed registration are centralized in `src-tauri/src/platform/database/mod.rs`, and no context may bypass the pool with its own connection or manage its own schema.

- **One database file**, `vanehub.sqlite`, with `journal_mode=WAL` (many readers, one writer), `foreign_keys=ON`, and `synchronous=FULL`, which syncs the WAL at every recovery-critical commit point.
- **A connection pool** capped at `MAX_POOL_SIZE = 12`, with `busy_timeout = 5s` and `CONNECTION_TIMEOUT = 5s`. The pool size tracks the number of Tauri command worker threads, and WAL keeps readers from being blocked by a writer.
- **Sequential migrations** run once on a single exclusive connection before the pool is shared, with the `schema_migrations` table accounting for each migration by version number and name.
- **`EXPECTED_MIGRATIONS` is the source of truth for the sequence.** The post-migration density check and the `migration_sequence_matches_expected` test both compare against it. A new migration must be appended at the end of the sequence — inserting or reordering is prohibited, because version numbers are assigned across branches and a renumbering breaks every checkout that already applied the old number.
- **Seed registration** runs `seed_registry` once on the same exclusive connection after migrations complete.
- **Bounded-context partitioning** gives each context its own tables, partitioned by which migration writes them. A foreign-key reference does not grant one context permission to query another context's tables directly.

```mermaid
flowchart TD
    AppStart([Application startup]) --> NewPool["Create the connection pool<br/>WAL / foreign_keys=ON / synchronous=FULL<br/>pool ≤ 12, busy_timeout=5s"]
    NewPool --> Exclusive["Take one exclusive connection"]
    Exclusive --> Migrate["migrate(conn)"]
    Migrate --> SchemaMig["CREATE TABLE schema_migrations<br/>if it does not exist"]
    SchemaMig --> ApplySeq["Apply the EXPECTED_MIGRATIONS sequence in order"]
    ApplySeq --> Book["Record each migration's<br/>version and name in schema_migrations"]
    Book --> Seed["seed_registry(conn)"]
    Seed --> SharePool["Share the pool with every context"]
    SharePool --> Ctx1["sessions bounded context<br/>its own tables"]
    SharePool --> Ctx2["agent_runtime bounded context<br/>its own tables"]
    SharePool --> Ctx3["code_intelligence bounded context<br/>its own tables"]
    Ctx1 -.a foreign key grants no cross-context query.-> Ctx2
```

## Database constants and migrations

### Database constants

| Constant | Value | Meaning |
| --- | --- | --- |
| `DATABASE_FILE_NAME` | `"vanehub.sqlite"` | The single database file |
| `MAX_POOL_SIZE` | `12` | Connection pool ceiling, tracking the Tauri command worker thread count |
| `busy_timeout` | `5s` | How long a reader waits while a writer holds the lock |
| `CONNECTION_TIMEOUT` | `5s` | Timeout for acquiring a connection |
| `journal_mode` | `WAL` | Many readers, one writer |
| `foreign_keys` | `ON` | Foreign key constraints enforced |
| `synchronous` | `FULL` | WAL synced at every recovery-critical commit point |

### Migrations

`EXPECTED_MIGRATIONS` in `src-tauri/src/platform/database/migrations/mod.rs` is the source of truth for the migration sequence, and both the post-migration density check and the `migration_sequence_matches_expected` test compare against it. A new migration must be appended at the end; inserting or reordering is prohibited. The `schema_migrations(version, name, applied_at)` table accounts for each applied migration, and `seed_registry` runs once on the same exclusive connection after migrations complete.

This chapter deliberately does not state how many migrations exist. Version numbers are allocated across concurrent branches, so any count written here is stale by the time a second branch merges.
