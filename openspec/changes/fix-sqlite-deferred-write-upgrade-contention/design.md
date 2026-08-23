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

Produced by walking every `.transaction()`, `unchecked_transaction()`, `transaction_with_behavior(...)`, `begin_write_transaction(...)`, `begin_read_transaction(...)`, raw `BEGIN`, `SAVEPOINT`, and `execute_batch` in `src-tauri/src`, and classifying each by what its body does between opening and commit. There are no `SAVEPOINT` uses in the repository.

**113 production sites.** Counts by shape, as the scanner sees them:

| Shape | Sites | Of which deferred |
| --- | --- | --- |
| read-then-write | 26 | **13** |
| multi-read | 6 | 4 |
| write-first | 53 | 44 |
| write-then-read | 6 | 3 |
| no read or write in body | 22 | 16 |

Only the **deferred read-then-write** column is the defect. The rest is context needed to avoid converting things that are already correct.

### Deferred read-then-write — the defect class

Each of these reads, then writes, inside a deferred transaction, and therefore fails under concurrency with an error no timeout affects.

| Site | Shape |
| --- | --- |
| `agent_runtime/infrastructure/sqlite_repository.rs:389` | read-then-write, long body |
| `agent_runtime/infrastructure/sqlite_repository.rs:585` | read-then-write |
| `agent_runtime/infrastructure/sqlite_repository.rs:628` | read-then-write |
| `agent_runtime/infrastructure/sqlite_repository.rs:663` | read-then-write |
| `communications/infrastructure/sqlite_repository.rs:309` | read-then-write, long body |
| `execution_observability/infrastructure/retention.rs:27` | read-then-write, long body |
| `retrieval/infrastructure/sqlite_repository.rs:75` | read-then-write, long body |
| `tooling/prompt_hooks/infrastructure/sqlite_repository.rs:348` | read-then-write |
| `tooling/skills/infrastructure/sqlite_repository.rs:1321` | read-then-write, long body |
| `tooling/skill_tools/infrastructure/sqlite_repository.rs:201` | read-then-write, long body |
| `workspaces/infrastructure/capture_maintenance.rs:56` | read-then-write |
| `workspaces/infrastructure/output_search.rs:90` | read-then-write |
| `work_board/api.rs:78` | read-then-write |

Every one is read individually before conversion. Where the read turns out to be incidental — a lookup whose result the write does not depend on — the fix is to move the read out, not to take a write lock earlier. A conversion is only correct when the write genuinely depends on what was read.

### Already correct

Thirteen read-then-write sites already use `BEGIN IMMEDIATE`, through `transaction_with_behavior` in `sessions`, `skill_evolution_evidence`, and `tooling::skills`, or through `begin_write_transaction` in the four subdomains Task Group 3 added. These are evidence that the pattern is understood in places; they are not touched.

Forty-four deferred write-first sites are correct as they stand. Their first statement takes the write lock and `busy_timeout` covers them. Converting them would be a no-op at best and, for the longer ones, would extend the window in which every other writer is blocked.

### Sites that need something other than a behaviour change

* **`execution_observability/infrastructure/settings_repository.rs:57`** — the scanner flags external I/O inside the transaction body. Read it and, if confirmed, move the I/O out.
* **`sessions/infrastructure/review_repository.rs:538`** — a deferred transaction opened with `.unwrap()`. `unwrap` is forbidden in production Rust in this repository, so this is a second, separate defect at the same site.
* **The 22 "no read or write in body" sites** — the scanner could not see a statement within its window. Most are helper wrappers that hand the transaction to another function. Each is read; a transaction whose body is in a callee is classified by what the callee does.
* **`operations/infrastructure/run_repository.rs:394`, `sessions/infrastructure/sqlite_repository.rs:61`, `permissions/.../sqlite_rules.rs:172`** — multi-statement reads with computation in the body. These want `begin_read_transaction` for snapshot consistency, and the computation examined for whether it belongs inside at all.

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
* **Linux Desktop Smoke**: the reported `database is locked` gets a reproduction that fails before the fix and passes after it.

A test that needs a `sleep` to pass is a test that will fail on a loaded runner and be silenced by lengthening the sleep. None are used.

## Architecture rule

A new fitness check: no file under `contexts/*/infrastructure/` may call `transaction()`, `unchecked_transaction()`, or `transaction_with_behavior(...)` directly. Repositories go through `begin_read_transaction` or `begin_write_transaction`, whose names state the decision and whose documentation states the rule. `platform/database/migrations/` is the single exemption, by path, because the migration runner owns a different protocol.

The rule is what keeps this change from being undone one repository at a time.
