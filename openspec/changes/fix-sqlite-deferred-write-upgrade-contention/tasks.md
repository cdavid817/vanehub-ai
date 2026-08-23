# Tasks

## 1. Inventory

- [x] 1.1 Scan every `transaction()`, `unchecked_transaction()`, `transaction_with_behavior(...)`, raw `BEGIN`, `SAVEPOINT`, and `execute_batch` in `src-tauri/src`, and record each site's shape. Keep the scan reproducible so a later reviewer can re-run it rather than trust the table.
- [x] 1.2 Classify all 101 production sites as `single read`, `multi-read consistent snapshot`, `write-first`, `read-then-write`, `CAS / state transition`, `long-compute-then-write`, `external-I/O-inside-transaction`, or `migration`. Read every site the scanner could not classify from its body alone.
- [x] 1.3 Record the classification in `design.md` with the per-site evidence for each planned edit. No site is edited before it appears there.

## 2. Helpers and failure identities

- [x] 2.1 Keep `begin_write_transaction` and `begin_read_transaction` as the two entry points, and state in each doc comment which classifications it is for and which it is not.
- [x] 2.2 Give `database_busy`, `database_storage_failure`, and stale-revision answers stable, distinct identities that survive being flattened into a `String`, with a test that a caller can still tell them apart afterwards.
- [x] 2.3 Add a test that a rolled-back, constraint-violating, CAS-losing, or dropped transaction releases the write lock in every case.

## 3. Convert what the classification says to convert

- [x] 3.1 Convert the eleven deferred read-then-write sites to `begin_write_transaction`, one context at a time, reading each first. Where the read turns out to be incidental, move the read out instead of taking the lock earlier — a conversion is only correct when the write depends on what was read.
- [x] 3.2 Convert multi-statement consistent reads to `begin_read_transaction`, and drop the transaction from single-statement reads.
  Three candidates, and reading them left **one** conversion. `sessions/.../sqlite_repository.rs::read_terminal_evidence` is a genuine consistent read -- a session's lifecycle and revisions, that session's messages, and whether another run has an unfinished message, all compared against each other -- so it now holds one snapshot. `sessions/.../transactions.rs` opens with `UPDATE ... RETURNING`, which is a *write*: it is a single-statement compare-and-swap, already correct, and converting it would have been the mechanical replacement this change refuses. `operations/.../run_repository.rs:394` was test code that a stale scanner run had reported as production.
- [x] 3.3 Move long computation out of transaction bodies.
  `operations::run_repository::insert` and `save` serialised a run's snapshot to JSON *inside* the transaction. Both now serialise first and open the transaction afterwards, and both moved to `begin_write_transaction` -- `save` is a compare-and-swap on `version`, and the helper's name states that decision. `read_terminal_evidence` already committed before building its evidence bundle, which is the shape the rule asks for.
- [x] 3.4 Move external I/O out of every writer reservation, and restructure the flow rather than shortening the I/O.
  **Nothing to move.** After correcting an over-broad scanner pattern -- it matched `keyring` inside a string literal naming a keyring entry, and matched the helpers' own doc comments describing this rule -- no production transaction performs filesystem, network, credential-store, MCP, Hook, WASM, sidecar, process, or approval work. Recorded as a finding rather than ticked as work.
- [x] 3.5 Repair `sessions/infrastructure/review_repository.rs:538`, which opens its transaction with `unwrap()`.
  **Not a defect.** The site is inside an inline `#[cfg(test)] mod tests`, where `unwrap` is permitted. The scanner judged test code by filename and did not see the module; it does now, which moved twelve sites out of the production count.
- [x] 3.6 Leave the forty-one deferred write-first sites alone, and say so in the inventory so a later reader does not "finish the job".
- [x] 3.7 Leave the migration runner on its own protocol.

## 4. Concurrency tests

- [x] 4.1 Add two-connection tests with deterministic barriers — channels, never sleeps — each stating the interleaving it forces.
- [x] 4.2 Prove the deferred read-then-write defect is gone: what previously failed with `SQLITE_BUSY` now waits and succeeds, under a *reduced* busy timeout so the test proves the timeout is consulted rather than that the machine is fast.
- [x] 4.3 Prove a read snapshot does not mix WAL generations across a concurrent commit.
- [x] 4.4 Prove no writer reservation is held across external I/O: a second connection takes the write lock while the first is doing the outside work.
  Adapted, because there is no production site doing external I/O in a transaction to test. What is proved instead is the property that would make one safe and that the read helper depends on: a `begin_read_transaction` holds no write lock, so a second connection takes the writer while the read is open.
- [x] 4.5 Build a real regression for the Desktop Smoke `database is locked` report.
  **Not this defect, and the premise was wrong twice.** The report is `docs/reports/desktop-client-verification-2026-08-20.md`, it is on **Windows** rather than Linux, and it already carries its cause: every spec shares one data directory and the specs run one app instance after another, so a fresh instance opens a database the previous one has not finished releasing. Two sequential processes have no concurrent transactions, and the upgrade defect needs two. What was built instead is a reproduction of the defect at the level where it exists -- a deferred transaction that reads, then cannot upgrade -- which fails without this change. Recorded as 4.5.1 so the suite's real cause keeps an owner.
- [ ] 4.5.1 Give each desktop spec its own data directory, so one app instance never opens a database the previous instance still holds. Recorded from `docs/reports/desktop-client-verification-2026-08-20.md`, which lists suite stability as one of three conditions for the twelve non-smoke specs entering CI. Not this change's to fix: it is a harness-isolation problem, and doing it here would couple a storage fix to a test-suite redesign.

## 5. Enforcement

- [x] 5.1 Add a fitness rule: no file under `contexts/*/infrastructure/` constructs a raw transaction; repositories use the two helpers. Exempt `platform/database/migrations/` by path, because the migration runner owns a different protocol.
  Bound to `(file, function, shape, deferred)` rather than to a per-file count. A count cannot tell a deleted call from a new one, so removing an old transaction and adding a raw one in the same file would have passed; and it says nothing about shape, so a write-first transaction growing a read before its first write -- the defect -- would have passed too. A deferred read-then-write is refused outright and can never be recorded in the baseline at all. The baseline shrank from 62 to 60 in this change.
- [x] 5.2 Add fixtures for the rule in both directions, including the migration exemption.

## 6. Verification

- [x] 6.1 Focused concurrency suite green (Windows).
- [x] 6.2 `cargo test --workspace` green (Windows).
- [x] 6.3 `clippy`, `fmt`, `architecture:check`, `contracts:check`, `docs:check`, and `openspec validate --strict` green (Windows).
- [ ] 6.4 Report the concurrency suite separately for Windows, Linux, and macOS as PASSED / FAILED / BLOCKED / NOT RUN. A result on one platform is never reported for another.
- [x] 6.5 Report the Desktop Smoke scenario. **NOT APPLICABLE** to this change: the recorded `database is locked` is Windows, and its cause -- specs sharing one data directory with one app instance after another -- involves no concurrent transactions. Tracked by 4.5.1. Not a completion gate here.
- [x] 6.6 Working tree clean.
- [ ] 6.7 Only then unblock `add-unified-extension-platform` Task Group 4.

## Status

- Implementation: **COMPLETE**
- Windows: **PASSED** — focused concurrency suite and `cargo test --workspace`
- Linux: **NOT RUN**
- macOS: **NOT RUN**
- Archive: **BLOCKED** on 6.4
- `add-unified-extension-platform` Task Group 4: **BLOCKED** on this change's archive

## Forbidden

- [x] X.1 No widened `busy_timeout`, no unbounded retry, and no fixed sleep anywhere in this change. Each hides the defect and makes its next occurrence harder to recognise. If a fix appears to need one, the classification was wrong.
