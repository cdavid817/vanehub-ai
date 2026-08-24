# Tasks

## 1. Locate

- [x] 1.1 Record the two unbounded call sites and the lock they contend for: `terminate_shell` (kill, then `child.lock()`, then unbounded `child.wait()`) and `start_exit_monitor`'s thread (holds the `child` mutex across an unbounded `child.wait()`). Note that the monitor can park `terminate_shell` on `child.lock()` before it reaches its own wait, so there are two distinct hangs, not one.
- [x] 1.2 Record the two hanging tests and the evidence: `a_blocked_shell_writer_does_not_stall_other_shells` and `manager_routes_input_resize_and_cleanup_by_shell_id`, CI run `32675752815`, job `97283440248`, last output 00:29:51, cancelled 03:09:39.

## 2. Regression tests first

- [x] 2.1 Add tests that fail within a short deadline of their own rather than by exhausting the CI timeout. A test that proves a bound must not depend on an unbounded harness to notice.
- [x] 2.2 Drive them with a fake child that never exits, so the failure is deterministic on every platform instead of only where the real PTY wedges.

## 3. Keep process work out of the routing lock

- [x] 3.1 Take the shell out of the registry and release the map guard *explicitly* before any kill or reap, in `stop`, `stop_for_session`, `insert`, and `Drop`. Relying on a temporary guard's lifetime is not a property a later edit preserves.
- [x] 3.2 Assert it directly: while one shell's reap is in flight, another shell still serves input, resize, and cleanup.

## 4. Bounded reap

- [x] 4.1 Replace `child.wait()` with `portable_pty::Child::try_wait` polled against a deadline, on every production path including the exit monitor.
- [x] 4.2 Use a monotonic `Instant` deadline computed once at entry.
- [x] 4.3 Back off from 1 ms to a 25 ms ceiling between polls. This is not a fixed sleep: each iteration reads real state, the loop ends on a real answer or a real deadline, and no test asserts on elapsed time.

## 5. Model the outcomes

- [x] 5.1 Add a closed `ShellTerminationOutcome`: `AlreadyExited`, `KillRequested`, `Reaping`, `Reaped`, `ReapTimedOut`, `KillFailed`, `ReapFailed`.
- [x] 5.2 Give each a stable code string, `reap_timed_out` among them.
- [x] 5.3 A timeout SHALL NOT be reported as terminated or successful, and SHALL NOT collapse into a generic failure — `kill_failed` and `reap_failed` stay distinct from it because they call for different responses.
- [x] 5.4 On `ReapTimedOut`, write redacted evidence naming the session and shell whose child was left unreaped, so cleanup ownership stays visible rather than being erased by a cheerful return.

## 6. Idempotent and single-flight

- [x] 6.1 Hold termination state in an atomic transitioned by compare-exchange; the first caller out of `Idle` owns the reap.
- [x] 6.2 A concurrent or repeated request observes `Reaping` and returns immediately, without starting a second reap and without queueing on the child mutex.
- [x] 6.3 The exit monitor participates in the same state machine rather than holding the child lock across a blocking call.

## 7. Fakes and the class-level guarantee

- [x] 7.1 Add `FakeChild`/`FakeKiller` over scripted `try_wait` answers, covering every outcome branch.
- [x] 7.2 `FakeChild::wait()` — the blocking one — SHALL fail the test if it is ever reached from a production path, so a reintroduced blocking wait fails by name instead of hanging until a CI ceiling cancels it.

## 8. Verification

- [x] 8.1 Focused `portable_pty` tests: Windows — 20 passed / 0 failed in 5.15s. The two tests that hung for 169 minutes on macOS are among them, and both now run against a scripted child rather than a real spawned shell.
- [x] 8.2 Focused `portable_pty` tests: Linux — 20/20 inside the `Rust` job's workspace run. Linux has no separately named focused step: `native-platform-check` is a windows/macos matrix, so the Linux evidence is the 20 `portable_pty::tests::*` lines within `cargo test --workspace`, all `ok`.
- [x] 8.3 Focused `portable_pty` tests: macOS — step `PTY shell termination suite`, conclusion `success`, `running 20 tests` → `20 passed; 0 failed; 0 ignored`, 5.17s.
- [ ] 8.4 `cargo test --workspace`: Windows. **Locally PASSED, on CI SKIPPED.** The local exclusive run is 3727 lib + 44 architecture + every integration suite, 0 failed. But `native-platform-check`'s `Rust tests` step is gated `if: runner.os == 'macOS'`, so on the Windows leg its conclusion is `skipped`, not `success`. A local result is not a CI result and this box stays open until they are the same thing. Closing it means either extending the workspace run to the Windows leg — a standing runner-cost decision, roughly +25 minutes per pull request — or accepting the local run as the Windows evidence. That is a call for the change's owner, not something to tick quietly.
- [x] 8.5 `cargo test --workspace`: Linux — 3723 + 43 architecture + 3 + 3 + 25, 0 failed, 13 ignored (pre-existing `#[ignore]`, none added here).
- [x] 8.6 `cargo test --workspace`: macOS — the platform this change exists for. Step `Rust tests`, conclusion `success`, `running 3736 tests` → 3723 passed / 0 failed / 13 ignored in 194.05s, plus 43 architecture, 3, 3, 25. The job finished in 24m58s where the same step previously ran 169 minutes without completing.
- [x] 8.7 `clippy`, `fmt`, `architecture:check`, `native:panic:check`, and `openspec validate --strict`.
- [x] 8.8 Carry the CI change that makes 8.3 and 8.6 possible. `main`'s `native-platform-check` runs `cargo build` on macOS and then two steps both gated `if: runner.os == 'Windows'`, so macOS executes no tests and reports green. That step was written on the SQLite branch and is not on `main`, so without it this PR could not produce macOS evidence for its own fix. It moves here, which is also where it belongs: the configuration that lets macOS run tests should land with the fix that makes those tests pass. Adds a named `PTY shell termination suite` step on both legs, `cargo test --workspace` on macOS, a Rust cache, and a 180-minute ceiling that bounds a cold run rather than targeting one.

## Forbidden

- [x] X.1 No fixed sleep as a synchronisation device, no test-level retry, no `#[ignore]` or skipped test, and no raise of a CI timeout. The previous ceiling raise is the evidence: 169 minutes bought nothing but more silence.
- [x] X.2 No change to `fix-sqlite-deferred-write-upgrade-contention` from this branch. That change re-qualifies on all three platforms after this one lands on `main`; it is not unblocked by this change's existence.

## 9. Concurrency invariants at close-out

Checked against the code rather than against intent. Two of the three hold; the third is a real gap and one of the first two is partial. None is fixed here — the tree is frozen at the qualification SHA — and each is recorded so it cannot be mistaken for done.

- [x] 9.1 **Single termination ownership.** `claim()` compare-exchanges `Idle → InFlight`, so exactly one caller runs `killer.kill()` and exactly one runs the reap loop. `repeated_termination_is_idempotent_and_starts_only_one_reap` proves the loop runs once by asserting the fake child's poll count is unchanged across a second `stop`. Once settled, every later caller reads the recorded outcome, so a shell that timed out keeps reporting `reap_timed_out`.
- [ ] 9.2 **Concurrent callers do not all receive the same settled outcome, by design — and this is untested under real threads.** A caller arriving while the reap is in flight gets `Reaping`, not the outcome the owner will eventually settle on. Returning an identical settled outcome would require waiting for the owner, which reintroduces exactly the blocking this change removes; `Reaping` is the honest answer to "what is happening right now". Whichever way that is resolved, the current tests exercise it single-threaded (`an_in_flight_reap_turns_a_concurrent_stop_away_rather_than_queueing_it` drives the state machine directly). A genuine multi-threaded `stop` race test is missing.
- [ ] 9.3 **A timed-out reap still drops the child handle.** `stop` removes the shell from the registry, terminates, and returns — so the `ManagedShell`, and with it the only `Arc` to the child, is dropped at end of scope even on `ReapTimedOut`. The exit monitor's clone does not save it: `terminate_shell` has already settled the state, so the monitor's next poll returns `Reaping` and it exits, releasing the last reference. What survives is a redacted log line naming the session and shell and the `reap_timed_out` code, which is evidence that a child was left unreaped — but there is no `cleanup_pending` record and no retained handle, so nothing can retry the reap or report the process later. The requirement was not to discard the sole child handle while claiming success; the claim is correct and the handle is still discarded.
- [x] 9.4 **Monitor polling releases the child lock before each backoff, does not hot-loop, and does not repeat warnings.** `probe_shared_child` takes the lock for one `try_wait` and drops it; backoff rises to a 250 ms ceiling; the monitor logs at most once and only for a non-`Reaped` outcome.
- [ ] 9.5 **The monitor can outlive `Manager` `Drop`, indefinitely.** `Drop` early-returns unless `Arc::strong_count(&self.shells) == 1`, and every monitor thread holds a clone of that `Arc`. A shell that is never stopped and never exits therefore keeps its monitor alive, which keeps the count above one, which makes `Drop` terminate nothing at all. This predates the change — the old monitor blocked in `wait()` for the same effect — and is neither introduced nor repaired here.

## Notes

One existing assertion changed rather than being preserved. `child_shutdown_failures_write_generic_warnings` expected two warnings — `"Shell process termination failed."` then `"Shell process wait failed."` — for a single event. `FailingChild` refuses the kill and keeps reporting itself as running, which is one outcome, `kill_failed`, so it now writes one warning naming that code. The old pair described the same failure twice and named neither, which is exactly the vagueness this change replaces. The redaction assertion is unchanged and still passes: the child's error text carries a secret and the outcome code carries none of it.

## Status

Qualification SHA: `caad3277808abc74983c1fd1348fe88ad229ebbf` (PR #217 head). Base `42b6a6495f46aa753d015364623b3290aced2514`. Checkout `07b86d9499dd1d46b24461fef255b34c1241e257` (`refs/pull/217/merge`). Run `32704450752`.

- Implementation: **COMPLETE**
- Windows focused `portable_pty`: **PASSED** — 20/20, job `97362556124`
- Windows `cargo test --workspace`: **SKIPPED on CI**, PASSED locally. See 8.4 — a local result is not a CI result.
- Linux focused `portable_pty`: **PASSED** — 20/20 within the workspace run, job `97362555965`
- Linux `cargo test --workspace`: **PASSED**
- macOS focused `portable_pty`: **PASSED** — 20/20, job `97362556125`
- macOS `cargo test --workspace`: **PASSED** — 3723/0/13 in 194.05s; the job that previously ran 169 minutes without completing now finishes in 24m58s
- Archive: **BLOCKED** — on 8.4, and on the close-out gaps 9.2, 9.3, and 9.5
- `fix-sqlite-deferred-write-upgrade-contention`: still **BLOCKED**. It re-qualifies on all three platforms after this change lands on `main`; it is not unblocked by this change existing.
- `add-unified-extension-platform` Task Group 4: **BLOCKED**
