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
- [x] 3.1.1 Two more arrived with `main`, and the rule caught them rather than a person. `cli_parameters/.../sqlite_profile_repository.rs::replace_if_revision` and `::reset_if_revision` are the defect exactly: a deferred transaction, `read_metadata` for the current revision, the compare, then `write_rows`. Two concurrent saves of one agent's CLI parameters would fail immediately with `database is locked`, and raising `busy_timeout` would not have helped. Both now open with `begin_write_transaction`. Thirteen sites converted in total.
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
  Bound to `(file, function, shape, deferred)` rather than to a per-file count. A count cannot tell a deleted call from a new one, so removing an old transaction and adding a raw one in the same file would have passed; and it says nothing about shape, so a write-first transaction growing a read before its first write -- the defect -- would have passed too. A deferred read-then-write is refused outright and can never be recorded in the baseline at all. The baseline shrank from 62 to 60 in this change, then grew to 66 for the six write-first sites that 5.3 made visible; the two the merge brought in that were the *defect* were fixed rather than added, which is the asymmetry the refusal enforces.
- [x] 5.2 Add fixtures for the rule in both directions, including the migration exemption.
- [x] 5.3 Fix the rule failing open on `#[cfg(test)]`. It took the first `#[cfg(test)]` in a file as the start of the inline test module and skipped everything below it. That attribute marks two different things: on a `mod` it opens a test module, but on any other item it marks that item alone and production code continues underneath. So one test-only helper near the top of a file hid every production transaction below it — which is how `cli_parameters/legacy_baseline.rs` (test-only helper at line 659, real transaction at line 784) read as having none. The rule now marks a single `#[cfg(test)]` item by brace depth and only treats `mod` as the module boundary. A rule that passes by not looking is worse than no rule, because it is also evidence. The checked-in Python audit already required `mod` on the following line and did not have this bug.
- [x] 5.4 Classify what the fix made visible. Six production transactions in four files had been hidden behind an item-level `#[cfg(test)]`, and each was read rather than baselined on sight: `loop_repository::save_continue_transition` and `::save_recovery_transition`, `communications::save_pairing_intent`, `::save_configuration` and `::delete_configuration` all open with `UPDATE`, `DELETE`, or `INSERT ... ON CONFLICT`, so all five are write-first and correct. `sessions::insert` reads as the defect until you follow `allocate_message_sequences`, which opens with `UPDATE ... RETURNING` — a write. It is recorded as `Opaque` because that is what the classifier can determine through a helper call; recording the shape a human read would make the entry stop matching and the rule stop binding.

## 6. Verification

- [x] 6.1 Focused concurrency suite green (Windows).
- [x] 6.2 `cargo test --workspace` green (Windows).
- [x] 6.3 `clippy`, `fmt`, `architecture:check`, `contracts:check`, `docs:check`, and `openspec validate --strict` green (Windows).
- [ ] 6.4 Report the concurrency suite separately for Windows, Linux, and macOS as PASSED / FAILED / BLOCKED / NOT RUN. A result on one platform is never reported for another.

  **Qualification evidence.** PR #211 head `d494fb65a5956eff016c4633929d910387bccf30`, base `c37caa4af7d5d8d2a2df88bb3f6a968891286843`, actual checkout `036f7709b55d9864e89e702108d4b2f117953778` (`refs/pull/211/merge`), workflow run `32675752815`, macOS job `97283440248`.

  | platform | focused concurrency suite | `cargo test --workspace` |
  | --- | --- | --- |
  | Windows | PASSED — 13 passed / 0 failed (CI job `97283440125`) | PASSED — 4492 lib + 53 architecture + all integration, local |
  | Linux | PASSED — within the workspace run | PASSED — CI `Rust` job, 17m32s |
  | macOS | **PASSED** — 13 passed / 0 failed / 0 ignored, 4.71s, step conclusion `success` (not skipped) | **FAILED (cancelled at the job ceiling)** — step conclusion `cancelled`, not skipped |

  The macOS concurrency suite covers all six tests this change adds, including `a_deferred_read_then_write_cannot_upgrade_but_an_immediate_one_waits_and_wins` — the reproduction of the defect itself. So the SQLite question this change exists to answer **is** answered on macOS.

- [ ] 6.4.2 macOS `cargo test --workspace` is blocked by an unrelated pre-existing hang. Classified as **test synchronisation**, not product semantics, not a platform SQLite difference, and not CI infrastructure.

  Evidence: the step ran 00:20:30 → 03:09:39 and was cancelled at the ceiling, having emitted its last output at 00:29:51 — **two hours and forty minutes of silence**. The final two lines name it:

  ```
  00:29:50 test contexts::workspaces::infrastructure::portable_pty::tests::a_blocked_shell_writer_does_not_stall_other_shells has been running for over 60 seconds
  00:29:51 test contexts::workspaces::infrastructure::portable_pty::tests::manager_routes_input_resize_and_cleanup_by_shell_id has been running for over 60 seconds
  ```

  Root cause is in existing code this change never touched: `portable_pty.rs::terminate_shell` calls `killer.kill()` and then `child.wait()`, and that wait has no deadline. Both tests spawn a real `$SHELL` into a real PTY. On macOS the killed child is not reaped, so `wait()` blocks forever and the test has no timeout of its own to escape it.

  Not a SQLite difference: the concurrency suite passed on the same runner minutes earlier. Not infrastructure: 169 minutes is not a slow compile, and raising the ceiling again would only buy more silence — which is why the ceiling is not being raised again. Not this change's product semantics: `workspaces::portable_pty` has no transaction and no migration in it.

  It went unseen because **macOS ran no tests at all** until 6.4.1 added them; the first real macOS test run is what surfaced it. Fixing it means giving the reap a bounded wait and the tests a deadline, which is a `workspaces` change with its own review, not something to graft onto a storage fix. Forbidden here and there: no fixed sleep, no unbounded retry, no wider timeout, no skipped or ignored test.

  Owned by change **`fix-portable-pty-bounded-termination`**, implemented on its own branch off `origin/main` so no PTY production code enters this change. The gate here is not relaxed: once that change lands on `main`, `main` merges back into PR #211 and all three platforms re-run both steps on the new merge SHA.
- [x] 6.4.1 Make CI capable of producing that report. It was not. The `rust` job runs `cargo test --workspace` on `ubuntu-latest`, so Linux was covered; `native-platform-check` on `macos-latest` ran `cargo build` and then two steps both gated `if: runner.os == 'Windows'`, so **macOS executed no tests at all** and still reported a green check. Waiting on that check would have produced a passing macOS result that proved nothing — the same failure mode this change's own fitness rule exists to prevent. `native-platform-check` now pins Node, runs the focused concurrency suite on both legs, and runs `cargo test --workspace` on macOS; its timeout moves 45 → 75 for the added compile. Which lock SQLite takes and whether it waits is decided per-OS in the VFS layer, so this is the platform evidence the gate asks for rather than a formality.
- [x] 6.5 Report the Desktop Smoke scenario. **NOT APPLICABLE TO THIS DEFECT**, and it stays tracked by 4.5.1. The recorded `database is locked` is Windows, and its cause -- specs sharing one data directory with one app instance after another -- involves no concurrent transactions, while this defect needs two. Not a completion gate here. On run `32675752815` Desktop Smoke passes on all three platforms (Windows 11m59s, macOS 8m48s, Linux 6m35s); the one Linux failure on the previous run was intermittent, confirmed by the same job passing here on a tree that differs by one comment character, with a clean native log and no SQLite, migration, or panic entry in it.
- [x] 6.6 Working tree clean. `git status` empty, `git diff --check` clean, both `openspec validate --strict` valid, and `origin/main` re-fetched: it advanced two dependabot commits touching only `package.json` and `package-lock.json`, so migrations 82-91 and every transaction call site remain conflict-free (`git merge-tree` reports none).
- [ ] 6.7 Only then unblock `add-unified-extension-platform` Task Group 4.

## Status

- Implementation: **COMPLETE**
- Windows: **PASSED** — focused concurrency suite and `cargo test --workspace`
- Linux: **PASSED** — focused concurrency suite and `cargo test --workspace`, CI run `32675752815`
- macOS focused SQLite concurrency: **PASSED** — 13/13, 4.71s, step conclusion `success`
- macOS `cargo test --workspace`: **BLOCKED / CANCELLED**
- Blocker: `workspaces::portable_pty` unbounded termination/reap — owned by `fix-portable-pty-bounded-termination`
- SQLite Change archive: **BLOCKED**
- `add-unified-extension-platform` Task Group 4: **BLOCKED**

The macOS workspace gate is **not** relaxed. It clears only when the PTY change lands on `main`, `main` merges back here, and all three platforms re-run both the focused SQLite suite and `cargo test --workspace` on the resulting merge SHA.

## Forbidden

- [x] X.1 No widened `busy_timeout`, no unbounded retry, and no fixed sleep anywhere in this change. Each hides the defect and makes its next occurrence harder to recognise. If a fix appears to need one, the classification was wrong.
