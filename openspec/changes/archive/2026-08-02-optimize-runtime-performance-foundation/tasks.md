## 1. Native release configuration

- [x] 1.1 Restore the approved optimized Cargo release profile
- [x] 1.2 Add an automated release-profile contract test

## 2. First-visit settings loading

- [x] 2.1 Convert every settings page registry entry to a dynamic module loader
- [x] 2.2 Mount only the default and subsequently visited settings pages while preserving visited state
- [x] 2.3 Add focused first-visit loading and state-preservation tests

## 3. Indexed historical-session search

- [x] 3.1 Add and test the SQLite message-content FTS migration, backfill, and synchronization triggers
- [x] 3.2 Replace wildcard/N+1 message search with one bounded indexed result query and a short-query fallback
- [x] 3.3 Debounce and length-bound React search submissions without changing desktop or Web/mock service contracts
- [x] 3.4 Add repository and frontend search scheduling regression tests

## 4. Bounded terminal replay storage

- [x] 4.1 Implement and test the native 1 MiB chunked transcript buffer
- [x] 4.2 Implement and test the frontend 1 MiB chunked replay cache while preserving duplicate-replay protection

## 5. Performance regression gates

- [x] 5.1 Extend frontend chunk validation with manifest-based raw and gzip budgets
- [x] 5.2 Record before/after deterministic bundle and query-plan evidence

## 6. Project validation

- [x] 6.1 Run frontend lint, unit tests, and production build
- [x] 6.2 Run Rust format, clippy, tests, and check
- [x] 6.3 Run strict change and main-spec OpenSpec validation

## 7. Verification follow-ups

- [x] 7.1 Reject every Cargo release-profile form that enables debug information
- [x] 7.2 Verify archived pre-migration messages remain searchable after FTS backfill
