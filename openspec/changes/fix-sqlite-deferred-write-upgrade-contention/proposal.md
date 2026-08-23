## Why

`Connection::transaction()` and `Connection::unchecked_transaction()` both open a **deferred** transaction. A deferred transaction takes no lock until its first statement, and a read takes a shared lock. When such a transaction later writes, SQLite has to *upgrade* to the write lock — and it refuses that upgrade with `SQLITE_BUSY` **without honouring `busy_timeout`**, because waiting could not help: the reader holds the very snapshot the writer would have to abandon.

So any transaction shaped read-then-write fails immediately under concurrency, on code that looks correct, passes every single-threaded test, and returns `database is locked` to the user. Raising `busy_timeout` does nothing, because the timeout is never consulted for this case.

Task Group 3 of `add-unified-extension-platform` hit this while testing two connections claiming one extension version, and repaired its own four call sites with a `begin_write_transaction` helper (`BEGIN IMMEDIATE`). That change deliberately did **not** touch anyone else's storage: a fix in `retrieval` or `sessions` is unreviewable inside an extension-platform diff, and a mistake there surfaces as a lock held across work that has no business holding one.

A repository-wide inventory finds the same shape in seven more production call sites, across `communications`, `execution_observability`, `tooling::prompt_hooks`, `tooling::skills`, `tooling::skill_tools`, `workspaces`, and `work_board` — eleven in total once Task Group 3's four are counted. It finds no transaction performing external I/O, which is worth stating because the first pass of the scanner claimed otherwise.

A `database is locked` symptom is on record, and it is **not** this defect. `docs/reports/desktop-client-verification-2026-08-20.md` records it on the **Windows** full desktop suite, with the cause already identified: every spec shares one data directory and the specs run one app instance after another, so a fresh instance opens a database the previous one has not finished releasing. That is a process-lifetime and suite-isolation problem, not a lock upgrade inside one process — the defect here needs two *concurrent* transactions, and sequential instances have none.

This change does not claim that symptom. It is named here so that fixing the upgrade defect is not later mistaken for having fixed the suite, and so the suite's real cause keeps its own owner.

## What Changes

* **Classify every transaction site before changing any of them.** All 101 production sites are inventoried as `single read`, `multi-read consistent snapshot`, `write-first`, `read-then-write`, `CAS / state transition`, `long-compute-then-write`, `external-I/O-inside-transaction`, or `migration`. The inventory is in `design.md` and is the evidence for each edit.
* **Convert only what the classification says to convert.** `read-then-write`, CAS, and state transitions become `begin_write_transaction`. Multi-statement consistent reads become `begin_read_transaction`. Single reads lose their transaction. Long computation moves out of the transaction. Nothing is converted because it is nearby.
* **Refuse a global replacement.** `BEGIN IMMEDIATE` holds a write lock for the whole transaction, so applying it to a read-only or external-I/O transaction converts a defect that appears under contention into a serialisation bottleneck that appears always.
* **Write down the external-I/O rule and check it.** Filesystem, network, credential store, MCP, Hook dispatch, WASM, sidecar, process spawn, and user approval never run while a write lock is held. No site violates it today; the rule is for the next transaction someone writes.
* **Give the two helpers stable failure identities.** `database_busy`, `storage_failure`, and `stale_revision` are distinguishable by every caller, so a retry loop retries contention and not corruption.
* **Add an architecture rule.** Repositories may not construct a raw transaction; they go through the two helpers. The migration runner keeps its own entry point and its own atomicity protocol.
* **Test with two independent connections and deterministic barriers.** No `sleep`, no timing assumption. Each test states which interleaving it forces.
* **Reproduce the defect itself**, at the level where it exists: a test that opens a deferred transaction, reads, and demonstrates the write cannot upgrade — then runs the same interleaving through the write helper and shows it waits and wins.

Explicitly out of scope: raising `busy_timeout`, adding retry loops, and inserting fixed sleeps. Each of those hides this defect rather than removing it, and each makes the next occurrence harder to recognise.

## Impact

* Affected specs: `native-runtime-architecture`
* Affected code: `src-tauri/src/platform/database/` (both helpers and their concurrency tests), the classified repositories in nine contexts, and `src-tauri/tests/architecture.rs`. The desktop smoke suite is **not** touched: its recorded `database is locked` has a different, already-identified cause.
* `add-unified-extension-platform` Task Group 4 stays blocked until this change is complete.
