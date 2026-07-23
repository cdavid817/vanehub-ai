## Why

The native runtime opens a brand-new SQLite connection on every operation — `NativeDatabase::connection()` is called at ~173 sites, each performing `Connection::open` plus per-connection pragma setup. A prior change (`fix(runtime)` PR) removed the redundant per-connection migration/seed work and added `busy_timeout` + WAL, but the connect-per-operation model still pays a fresh file open, pragma configuration, and cold page cache for every read and write, and offers no back-pressure on the number of live database handles under concurrent load. Reusing pooled, pre-configured connections removes this fixed per-operation overhead and bounds concurrent handles.

## What Changes

- Introduce a bounded SQLite connection pool inside `NativeDatabase`, replacing per-operation `Connection::open` with checkout/return of reused connections.
- Configure each pooled connection once on creation (`busy_timeout`, `foreign_keys`, WAL) via a connection customizer, and run schema migration + registry seeding exactly once at pool initialization.
- Preserve the `NativeDatabase::connection()` call contract so the ~173 existing call sites keep using a value that dereferences to `rusqlite::Connection` with minimal or no edits.
- Add concurrency and pool-lifecycle regression tests (parallel readers/writers, pool-exhaustion back-pressure, migrate-once-under-races).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-runtime-architecture`: Require the native runtime to serve database operations from a bounded pool of reused, pre-configured SQLite connections rather than opening a new connection per operation, initializing schema exactly once.

## Impact

- Affects `src-tauri/src/platform/database/mod.rs` (connection lifecycle and pragma customizer). Call sites that bind `let connection = self.database.connection()?` continue to compile if the returned guard dereferences to `rusqlite::Connection`.
- Adds a connection-pool dependency (`r2d2` + `r2d2_sqlite`, which layer over the existing `rusqlite`) or a hand-rolled equivalent — see `design.md`. No change to the SQLite storage format, migrations, DTOs, or the service/command boundaries.
- WAL (already enabled) permits concurrent readers against the pool; writers remain serialized by SQLite with `busy_timeout` back-pressure. The `unified-log-management` and other persistence-backed capabilities are unaffected functionally; only the connection-acquisition mechanism changes.
