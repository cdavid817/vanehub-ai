# Tasks

## 1. Inventory

- [ ] 1.1 Scan every `transaction()`, `unchecked_transaction()`, `transaction_with_behavior(...)`, raw `BEGIN`, `SAVEPOINT`, and `execute_batch` in `src-tauri/src`, and record each site's shape. Keep the scan reproducible so a later reviewer can re-run it rather than trust the table.
- [ ] 1.2 Classify all 113 production sites as `single read`, `multi-read consistent snapshot`, `write-first`, `read-then-write`, `CAS / state transition`, `long-compute-then-write`, `external-I/O-inside-transaction`, or `migration`. Read every site the scanner could not classify from its body alone.
- [ ] 1.3 Record the classification in `design.md` with the per-site evidence for each planned edit. No site is edited before it appears there.

## 2. Helpers and failure identities

- [ ] 2.1 Keep `begin_write_transaction` and `begin_read_transaction` as the two entry points, and state in each doc comment which classifications it is for and which it is not.
- [ ] 2.2 Give `database_busy`, `database_storage_failure`, and stale-revision answers stable, distinct identities that survive being flattened into a `String`, with a test that a caller can still tell them apart afterwards.
- [ ] 2.3 Add a test that a rolled-back, constraint-violating, CAS-losing, or dropped transaction releases the write lock in every case.

## 3. Convert what the classification says to convert

- [ ] 3.1 Convert the thirteen deferred read-then-write sites to `begin_write_transaction`, one context at a time, reading each first. Where the read turns out to be incidental, move the read out instead of taking the lock earlier — a conversion is only correct when the write depends on what was read.
- [ ] 3.2 Convert multi-statement consistent reads to `begin_read_transaction`, and drop the transaction from single-statement reads.
- [ ] 3.3 Move long computation out of transaction bodies.
- [ ] 3.4 Move external I/O out of every writer reservation, and restructure the flow rather than shortening the I/O.
- [ ] 3.5 Repair `sessions/infrastructure/review_repository.rs:538`, which opens its transaction with `unwrap()` — a separate defect at the same site, and forbidden in production Rust here.
- [ ] 3.6 Leave the forty-four deferred write-first sites alone, and say so in the inventory so a later reader does not "finish the job".
- [ ] 3.7 Leave the migration runner on its own protocol.

## 4. Concurrency tests

- [ ] 4.1 Add two-connection tests with deterministic barriers — channels, never sleeps — each stating the interleaving it forces.
- [ ] 4.2 Prove the deferred read-then-write defect is gone: what previously failed with `SQLITE_BUSY` now waits and succeeds, under a *reduced* busy timeout so the test proves the timeout is consulted rather than that the machine is fast.
- [ ] 4.3 Prove a read snapshot does not mix WAL generations across a concurrent commit.
- [ ] 4.4 Prove no writer reservation is held across external I/O: a second connection takes the write lock while the first is doing the outside work.
- [ ] 4.5 Build a real regression for the Linux Desktop Smoke `database is locked` report — one that fails before this change and passes after it. If it cannot be reproduced, say so and say what was tried, rather than closing the item.

## 5. Enforcement

- [ ] 5.1 Add a fitness rule: no file under `contexts/*/infrastructure/` constructs a raw transaction; repositories use the two helpers. Exempt `platform/database/migrations/` by path, because the migration runner owns a different protocol.
- [ ] 5.2 Add fixtures for the rule in both directions, including the migration exemption.

## 6. Verification

- [ ] 6.1 Focused concurrency suite green.
- [ ] 6.2 `cargo test --workspace` green.
- [ ] 6.3 `clippy`, `fmt`, `architecture:check`, `contracts:check`, `docs:check`, and `openspec validate --strict` green.
- [ ] 6.4 Report the concurrency suite separately for Windows, Linux, and macOS as PASSED / FAILED / BLOCKED / NOT RUN. A result on one platform is never reported for another.
- [ ] 6.5 Report the Linux Desktop Smoke scenario.
- [ ] 6.6 Working tree clean.
- [ ] 6.7 Only then unblock `add-unified-extension-platform` Task Group 4.

## Forbidden

- [ ] X.1 No widened `busy_timeout`, no unbounded retry, and no fixed sleep anywhere in this change. Each hides the defect and makes its next occurrence harder to recognise. If a fix appears to need one, the classification was wrong.
