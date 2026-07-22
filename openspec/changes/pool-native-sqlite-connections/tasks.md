## 1. Connection Pool Foundation

- [ ] 1.1 Add the connection-pool dependency (`r2d2` + `r2d2_sqlite`) or a hand-rolled pool module, plus a connection customizer that sets `busy_timeout`, `foreign_keys`, and WAL per physical connection.
- [ ] 1.2 Build the pool in `NativeDatabase` with a bounded max size and checkout timeout; run `migrate()` + `seed_registry()` exactly once, reusing the existing race-safe init gate.
- [ ] 1.3 Change `NativeDatabase::connection()` to return a pooled guard that dereferences to `rusqlite::Connection`, keeping the existing `DatabaseError` return type.

## 2. Call-Site Migration

- [ ] 2.1 Compile the crate and resolve any call sites that consume `Connection` by value or rely on `Connection`-specific ownership.
- [ ] 2.2 Confirm `transaction()` / `&mut` usages bind `let mut connection = ...`.

## 3. Verification

- [ ] 3.1 Add tests: parallel readers + writer under WAL, pool-exhaustion back-pressure/timeout, migrate-once-under-races.
- [ ] 3.2 Run `cargo test`, `cargo clippy --manifest-path src-tauri/Cargo.toml`, and `cargo check`; confirm no regression across the existing 550+ tests.
- [ ] 3.3 Confirm the hot path no longer performs `Connection::open` + pragma configuration per operation (open/configure happens only on pool growth).
