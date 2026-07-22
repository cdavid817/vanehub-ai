## 1. Connection Pool Foundation

- [x] 1.1 Add the connection-pool dependency (`r2d2` + `r2d2_sqlite`) plus a connection customizer (`SqliteConnectionManager::with_init`) that sets `busy_timeout`, WAL, and `foreign_keys` per physical connection.
- [x] 1.2 Build the pool in `NativeDatabase` with a bounded max size (12) and 5s checkout timeout; run `migrate()` + `seed_registry()` exactly once during `new()` (single-threaded bootstrap, so no init race remains).
- [x] 1.3 Change `NativeDatabase::connection()` to return a pooled guard (`PooledSqlite`) that dereferences to `rusqlite::Connection`, keeping the existing `DatabaseError` return type.

## 2. Call-Site Migration

- [x] 2.1 Update the 7 repository `connection()` wrappers to return `PooledSqlite`, and change inline delegations `&self.connection()?` → `&*self.connection()?` (deref-reborrow to `&Connection`); removed one now-unused `Connection` import.
- [x] 2.2 Confirm no `transaction()` / by-value `Connection` call site broke (`cargo check --tests` clean; the pooled guard's `DerefMut` covers `&mut Connection` where used).

## 3. Verification

- [x] 3.1 Added a pooled-concurrency test: workers > `MAX_POOL_SIZE` write+read in parallel under WAL, which also exercises checkout back-pressure (excess threads wait for a returned connection) and asserts seeding ran exactly once. (A dedicated 5s checkout-timeout test was omitted in favor of the faster back-pressure path; the timeout itself is r2d2's tested contract.)
- [x] 3.2 `cargo test` (560 passed / 0 failed), `cargo clippy --all-targets` (clean), `cargo check --tests` (clean).
- [x] 3.3 Confirmed the hot path no longer performs `Connection::open` + pragma configuration per operation — open/configure now happens only on pool growth via the customizer.
