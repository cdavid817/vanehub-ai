## Context

`NativeDatabase` holds a `db_path` plus a one-time-init flag; `connection()` opens a fresh `rusqlite::Connection` per call. There are ~173 call sites, each a synchronous operation running on a Tauri command worker thread (not the async executor). With WAL enabled, readers no longer block the single writer, but every operation still pays a file open + pragma round-trip + cold page cache.

## Goals / Non-Goals

Goals:
- Eliminate per-operation connection open/configure overhead by reusing connections.
- Bound the number of live SQLite handles and provide back-pressure under load.
- Keep the change surface at the ~173 call sites near-zero.

Non-Goals:
- No change to SQL, schema, or migration content.
- No move to an async SQLite driver.
- No change to the service/command boundary, Tauri commands, or DTOs.

## Decisions

### Pool implementation

- **Option A — `r2d2` + `r2d2_sqlite`**: battle-tested. Provides a `CustomizeConnection` hook for per-connection pragmas and a manager that opens connections lazily. `PooledConnection<SqliteConnectionManager>` dereferences to `rusqlite::Connection`, so `connection()` returning it keeps call sites unchanged. Layers two crates over the already-approved `rusqlite`.
- **Option B — hand-rolled pool**: `Arc<Mutex<Vec<Connection>>>` with a checkout guard whose `Drop` returns the connection. Zero new dependency, but re-implements customizer/health/timeout/exhaustion logic and is easy to get subtly wrong (mutex poisoning, guard return on panic, fairness).

Recommendation: **Option A**. It satisfies the "SQLite via rusqlite" stack constraint (additive layer, not a replacement) and avoids bespoke concurrency code. If a new dependency is unacceptable, fall back to a minimal Option B with an explicit bounded size and checkout timeout.

### Connection configuration

Move `busy_timeout(5s)`, `PRAGMA foreign_keys=ON`, and `journal_mode=WAL` into a pool connection customizer that runs once per physical connection (instead of on every `connection()` call). Run `migrate()` + `seed_registry()` exactly once at pool build (or on first checkout), guarded so racing checkouts cannot double-apply migrations — the current `Mutex<bool>` gate already provides this and can be reused.

### Pool sizing

Default max size ≈ the Tauri worker-thread count (e.g. 8–16). WAL supports many concurrent readers with a single writer, so a small pool suffices and bounds handles. Expose the size and checkout timeout as constants for later tuning.

### Call-site compatibility

Change `connection()`'s return type from `Connection` to the pooled guard. Because the guard implements `Deref`/`DerefMut` to `Connection`, `conn.prepare(...)`, `conn.execute(...)`, and `conn.transaction()` keep compiling. The few sites that consume a `Connection` by value (rare — most bind `let connection = ...; connection.prepare(...)`) need a local adjustment.

## Risks / Trade-offs

- **New dependency** (`r2d2_sqlite`) — mitigated: it wraps the already-approved `rusqlite`; called out in `proposal.md` Impact.
- **`transaction()` needs `&mut Connection`** — pooled guards provide `DerefMut`, so `let mut connection = db.connection()?` works; sites using transactions must keep the `mut` binding.
- **Pool exhaustion under a burst** — bounded by a checkout timeout; pick a sane max + timeout and surface a structured `DatabaseError` rather than blocking indefinitely.

## Migration Plan

1. Add the pool to `NativeDatabase`, keeping the `connection()` name and `DatabaseError` return type.
2. Move pragmas into the customizer; run migrate/seed once at build behind the existing gate.
3. Compile; fix any by-value `Connection` uses among the ~173 call sites.
4. Add concurrency + exhaustion tests; run full `cargo test` + `cargo clippy` and confirm no regression.

## Open Questions

- Preferred maximum pool size and checkout timeout values.
- Accept the `r2d2_sqlite` dependency, or require the zero-dependency hand-rolled pool (Option B)?
