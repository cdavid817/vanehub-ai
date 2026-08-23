# Design

## The defect, precisely

Under WAL, SQLite gives a deferred transaction a shared read lock at its first read. A later write must upgrade that to the write lock, and SQLite **refuses the upgrade with `SQLITE_BUSY` without consulting `busy_timeout`**. This is not a tuning problem: the timeout exists to make a writer wait for a writer, and it is deliberately not applied here, because a reader that waited would be waiting to abandon the snapshot it is holding — the wait could never succeed.

`BEGIN IMMEDIATE` takes the write lock at the start. That is the case `busy_timeout` *does* cover, so contention becomes a bounded wait instead of an instant failure.

Three consequences shape everything below:

* A **read-then-write** deferred transaction is broken under concurrency, always, regardless of timeout.
* A **write-first** deferred transaction is fine: its first statement takes the write lock, and `busy_timeout` covers it.
* An **immediate** transaction serialises every other writer for its whole life, so making one long, or letting it wait on something outside the database, is a different and worse bug.

That third point is why this change classifies before it edits. The obvious remedy — convert everything to `BEGIN IMMEDIATE` — trades an intermittent failure for a permanent bottleneck and, on any transaction containing external I/O, for a database-wide stall lasting as long as a keychain prompt goes unanswered.

## Inventory

Produced by `scripts/audit-sqlite-transactions.py`, which walks every `.transaction()`, `unchecked_transaction()`, `transaction_with_behavior(...)`, `begin_write_transaction(...)`, `begin_read_transaction(...)`, raw `BEGIN`, `SAVEPOINT`, and `execute_batch` in `src-tauri/src` and classifies each by what its body does between opening and commit. It is checked in so a reviewer can re-run it rather than trust a table. There are no `SAVEPOINT` uses in the repository.

**The scanner was wrong four times, and each was found by reading a site it flagged.** They are recorded because a scanner nobody distrusts is a scanner that decides the work:

* It counted `prepare` as a read. Preparing a statement takes no lock, so `prepare` before `execute` is still write-first. Two sites — `retrieval/infrastructure/sqlite_repository.rs:75` and `workspaces/infrastructure/output_search.rs:90` — would have been converted for nothing.
* It judged test code by filename, so an inline `#[cfg(test)] mod tests` inside a production file read as production. That reported a test's `unwrap()` in `sessions/infrastructure/review_repository.rs:538` as a forbidden production `unwrap`. It is test code, where `unwrap` is permitted. Fixing this moved twelve sites out of the production count.
* Its external-I/O pattern matched `keyring` anywhere, and fired on a *string literal* naming a keyring entry in `execution_observability/infrastructure/settings_repository.rs:57` — a reference being written to a column, not a call.
* The same pattern fired on the two helpers' own doc comments in `platform/database/mod.rs`, which describe the external-I/O rule in prose.

After those corrections: **101 production sites.**

| Shape | Sites | Of which deferred, before this change |
| --- | --- | --- |
| read-then-write | 24 | **11** |
| multi-read consistent snapshot | 5 | 3 |
| write-first | 50 | 41 |
| write-then-read | 1 | 0 |
| body not visible to the scanner | 21 | 16 |

Only the deferred read-then-write column was the defect.

### The defect class, and what was done to each

Eleven sites read, then wrote, inside a deferred transaction, and therefore failed under concurrency with an error no timeout affects. Each was read before conversion, and each turned out to have a write that genuinely depends on the read — a guard, a revision check, or a rank computed from what was there:

| Site | What the write depends on |
| --- | --- |
| `agent_runtime/.../sqlite_repository.rs:389` | the stored `agent_origin`, which decides whether the delete is permitted |
| `agent_runtime/.../sqlite_repository.rs:585` | a count proving the built-in agent exists |
| `agent_runtime/.../sqlite_repository.rs:628` | the profile row being activated |
| `agent_runtime/.../sqlite_repository.rs:663` | whether the profile being deleted was the active one |
| `communications/.../sqlite_repository.rs:309` | the pairing intent's connector, session, and expiry |
| `execution_observability/.../retention.rs:27` | the last retention timestamp, which decides whether to run at all |
| `tooling/prompt_hooks/.../sqlite_repository.rs:348` | the current published version and the draft revision |
| `tooling/skills/.../sqlite_repository.rs:1321` | a duplicate-alias count |
| `tooling/skill_tools/.../sqlite_repository.rs:201` | the revision row the trust decision authorises |
| `workspaces/.../capture_maintenance.rs:56` | the running total of captured bytes |
| `work_board/api.rs:78` | the rank of the item being inserted before |

All eleven now use `begin_write_transaction`. The scanner reports zero deferred read-then-write sites remaining.

### Already correct, and left alone

Thirteen read-then-write sites already used `BEGIN IMMEDIATE` — through `transaction_with_behavior` in `sessions`, `skill_evolution_evidence`, and `tooling::skills`, or through `begin_write_transaction` in the four subdomains Task Group 3 added.

Forty-one deferred **write-first** sites are correct as they stand: their first statement takes the write lock and `busy_timeout` covers them. Converting them would be a no-op at best and, for the longer ones, would extend the window in which every other writer is blocked. They are listed here so a later reader does not "finish the job".

### External I/O inside a transaction: none found

After correcting the pattern, no production transaction performs filesystem, network, credential-store, MCP, Hook, WASM, sidecar, process, or approval work. The rule is still written down and still enforced by review, because the next transaction someone writes is the one it is for — but this change moves nothing, and says so rather than claiming work it did not do.

## Governance rules

Written down here because the classification is only useful if the next person applies the same rule to a site this change did not touch.

* **A single-statement read needs no transaction.** Adding one costs a lock acquisition and buys nothing: one statement is already atomic.
* **A multi-statement read whose statements must agree uses `begin_read_transaction`.** Under WAL each bare statement takes its own snapshot, so a sequence of reads can straddle a commit and return a state that never existed — half from before, half from after, each half individually consistent, which is what makes the bug survive review.
* **Read-then-write, compare-and-swap, and state transitions use `begin_write_transaction`.** These are the cases the deferred upgrade breaks.
* **Long computation goes outside the transaction.** Hashing, serialisation, sorting a large set: do it first, then open the transaction and write.
* **Filesystem, network, credential store, MCP, Hook dispatch, WASM, sidecar, process spawn, and user approval never happen inside a writer reservation.** Each can block for seconds or forever, and every one of them blocks *all* database writers while it does. A flow needing both does the outside work first, or afterwards under a compensating step — never in between.
* **The migration runner keeps its own protocol.** It is the one place that legitimately wraps schema changes, it runs before the pool is shared, and it has its own atomicity requirements. It is not converted to either helper.

## Failure identities

`WriteTransactionError` already separates `Busy` from `Storage`, and renders the stable code through `Display` so the distinction survives being flattened into a `String`. This change extends the same treatment to the third answer callers branch on:

* `database_busy` — contention. Worth retrying, at a layer that can decide to.
* `database_storage_failure` — anything else the database said. Not worth retrying.
* `*_stale_revision` — a compare-and-swap lost. Not a database failure at all; the caller re-reads and decides.

The three must never collapse. A retry loop that cannot tell contention from corruption retries corruption; a caller that cannot tell a lost CAS from a storage failure reports "the database is broken" for an ordinary concurrent edit.

## How this is tested

Two independent pooled connections, and a deterministic rendezvous — channels, not sleeps. Each test states which interleaving it forces and asserts the answer that interleaving must produce.

* **The deferred defect is gone**: a read-then-write transaction that would previously fail with `SQLITE_BUSY` now waits and succeeds, with a *reduced* `busy_timeout` so the test proves the timeout is consulted rather than proving the machine is fast.
* **A read snapshot does not mix WAL generations**: a reader pauses mid-transaction, a writer commits, and the reader's remaining statements still see one whole generation.
* **The write lock is released** after commit, after rollback, after a constraint violation, after a lost CAS, and after the guard is dropped without either.
* **No writer reservation is held across external I/O**: a second connection acquires the write lock while the first is doing the outside work.
The desktop smoke suite is deliberately not among them. Its recorded `database is locked` is on Windows, and its cause is already written down: the specs share one data directory and run one app instance after another, so a fresh instance opens a database the previous one has not released. Two sequential processes have no concurrent transactions, so the upgrade defect cannot be what fails there. Building a "regression" for it here would attach this change to a symptom it does not cause, and the suite's real fix — per-spec data directories — would then look already done.

A test that needs a `sleep` to pass is a test that will fail on a loaded runner and be silenced by lengthening the sleep. None are used.

## Architecture rule

A new fitness check: no file under `contexts/*/infrastructure/` may call `transaction()`, `unchecked_transaction()`, or `transaction_with_behavior(...)` directly. Repositories go through `begin_read_transaction` or `begin_write_transaction`, whose names state the decision and whose documentation states the rule. `platform/database/migrations/` is the single exemption, by path, because the migration runner owns a different protocol.

The rule is what keeps this change from being undone one repository at a time.
