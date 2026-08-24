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

- [x] 5.1 Split the two questions apart. `TerminationOutcome` is what one attempt did -- `Reaped`, `ReapTimedOut`, `KillFailed`, `ReapFailed` -- and is fixed once the owner settles it. `CleanupState` is where the child stands -- `NotRequired`, `Reaping`, `Pending`, `ReapedLater`, `UnresolvedAtShutdown` -- and keeps moving afterwards. The first shape of this change had one enum carrying both, which is why a reap could report a timeout honestly and still drop the child: there was nowhere to say "and something still owns it".
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

Every result below was earned on `caad3277` and is **withdrawn**: production code and the workflow both changed afterwards, so none of it describes the tree being proposed. Re-earned on one final merge SHA or not at all.

- [ ] 8.1 Focused `portable_pty` tests: Windows, on CI.
- [ ] 8.2 Focused `portable_pty` tests: Linux, on CI. Now its own named step rather than 20 lines buried in a workspace log.
- [ ] 8.3 Focused `portable_pty` tests: macOS, on CI.
- [ ] 8.4 `cargo test --workspace`: Windows, on CI. Previously locally PASSED but CI-SKIPPED, which is not the same thing -- a gate that takes a developer's machine for one platform and CI for the others is two standards, not one. The step is no longer macOS-gated.
- [ ] 8.5 `cargo test --workspace`: Linux, on CI.
- [ ] 8.6 `cargo test --workspace`: macOS, on CI.
- [x] 8.7 `clippy`, `fmt`, and `openspec validate --strict` on the new tree.
- [x] 8.8 Make the workflow run the same two things on all three platforms. `main`'s `native-platform-check` ran `cargo build` on macOS and then two steps both gated `if: runner.os == 'Windows'`, so macOS executed no tests and reported green. That is now gone: both legs of the matrix run the focused suite and the workspace suite unconditionally, and the `rust` job gains a named focused step so Linux reports the same way. Cost is controlled with a Rust cache and the existing `cancel-in-progress`, not by narrowing what runs.

## Forbidden

- [x] X.1 No fixed sleep as a synchronisation device, no test-level retry, no `#[ignore]` or skipped test, and no raise of a CI timeout. The previous ceiling raise is the evidence: 169 minutes bought nothing but more silence.
- [x] X.2 No change to `fix-sqlite-deferred-write-upgrade-contention` from this branch. That change re-qualifies on all three platforms after this one lands on `main`; it is not unblocked by this change's existence.

## 9. Concurrency invariants

- [x] 9.1 **Single termination ownership.** `claim()` compare-exchanges `Idle -> InFlight`, so exactly one caller runs `killer.kill()` and exactly one runs the reap loop. Proven under real threads by `concurrent_stops_elect_one_owner_and_the_rest_do_not_claim_a_result`: eight threads released together by a `Barrier`, then `kills() == 1` and exactly one report carries an outcome.
- [x] 9.2 **Followers return immediately and claim nothing.** A caller arriving mid-flight gets `outcome: None, cleanup: Reaping` -- it does not queue on the child mutex and does not report a result another thread produced. Once the owner settles there is exactly one final outcome, and every later ask returns it without touching the child. The semantics are the ones stated in the decision: one owner, fast followers, one settled answer afterwards.
- [x] 9.3 **A child that outlives its termination is owned, not dropped.** `TerminationOutcome` (what the attempt did) and `CleanupState` (where the child stands) are now separate types. `ReapTimedOut` and `KillFailed` both transfer the child to `PendingReapRegistry` **before** the caller releases the `ManagedShell`, so the sole handle to a live process is never dropped. Sweeps use `try_wait` only. A later exit sets `ReapedLater` and deliberately leaves the outcome alone -- a reap that timed out and was then reclaimed is not the same history as one that succeeded.
- [x] 9.4 **Monitor polling releases the child lock before each backoff, does not hot-loop, and does not repeat warnings.** One `try_wait` per lock acquisition; backoff to a 250 ms ceiling; at most one log, and only for a non-`Reaped` outcome.
- [x] 9.5 **Shutdown is explicit and bounded, and no monitor outlives it.** The `Arc::strong_count` early return is gone. The runtime is a thin handle over a `ManagerCore` that owns the shutdown token, the monitor handles, the shells, and the pending reaps; `Drop for ManagerCore` runs exactly once, when the last handle goes, with `&mut self` proving it. It signals shutdown, unparks and joins every monitor, drains the registry and releases the lock, terminates every child against **one** shared deadline, sweeps, and records whatever is still owed as `UnresolvedAtShutdown`.

## 10. Tests added for the above

- [x] 10.1 N threads stopping one shell elect a single kill/reap owner (`concurrent_stops_elect_one_owner_and_the_rest_do_not_claim_a_result`).
- [x] 10.2 `ReapTimedOut` leaves the registry holding the child (`a_timed_out_reap_keeps_the_child_instead_of_dropping_the_last_handle`).
- [x] 10.3 A later exit becomes `ReapedLater` without rewriting the outcome (`a_child_that_exits_later_becomes_reaped_later_without_rewriting_the_timeout`).
- [x] 10.4 A refused kill on a live child keeps ownership too (`a_refused_kill_on_a_live_child_also_keeps_ownership`).
- [x] 10.5 `Drop` finishes on its own budget with every monitor gone (`manager_drop_ends_within_the_deadline_and_leaves_no_monitor_running`).
- [x] 10.6 One pending shell does not hold up another's shutdown (`one_shell_pending_cleanup_does_not_hold_up_another_shells_shutdown`).
- [x] 10.7 `FakeChild::wait()` still panics if a production path reaches it, asserted across every outcome branch.
- [x] 10.8 The `open_shell` failure paths were the last place a `kill_failed` could still drop the only handle to a live child. They now share the registered route, and the owned-child code path is deleted rather than left as a second way to do it.

## Notes

One existing assertion changed rather than being preserved. `child_shutdown_failures_write_generic_warnings` expected two warnings — `"Shell process termination failed."` then `"Shell process wait failed."` — for a single event. `FailingChild` refuses the kill and keeps reporting itself as running, which is one outcome, `kill_failed`, so it now writes one warning naming that code. The old pair described the same failure twice and named neither, which is exactly the vagueness this change replaces. The redaction assertion is unchanged and still passes: the child's error text carries a secret and the outcome code carries none of it.

## Status

Previous qualification SHA `caad3277` is **superseded**. Production code and the workflow both changed, so all six results must be re-earned on the new final merge SHA.

- Implementation: **COMPLETE**
- Windows focused `portable_pty`: **PENDING** (CI)
- Windows `cargo test --workspace`: **PENDING** (CI) -- the step is no longer macOS-gated, so this is a real CI result rather than a local one
- Linux focused `portable_pty`: **PENDING** (CI, now its own named step)
- Linux `cargo test --workspace`: **PENDING** (CI)
- macOS focused `portable_pty`: **PENDING** (CI)
- macOS `cargo test --workspace`: **PENDING** (CI)
- Archive: **BLOCKED** until all six pass on one final merge SHA
- `fix-sqlite-deferred-write-upgrade-contention`: **BLOCKED**
- `add-unified-extension-platform` Task Group 4: **BLOCKED**
