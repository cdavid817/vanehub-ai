## Why

`portable_pty.rs::terminate_shell` kills a managed shell and then calls `child.wait()`. That wait has no deadline. `start_exit_monitor` does the same thing from a background thread, and it holds the `child` mutex for the whole blocking call. So two code paths can each park forever on the same lock, and nothing in the design bounds either of them.

On macOS this is not theoretical. CI run `32675752815`, job `97283440248`, ran `cargo test --workspace` from 00:20:30 to 03:09:39 and was cancelled at the job ceiling. Its last output was at 00:29:51 — **two hours and forty minutes of silence** — and the final two lines name the tests still running:

```
portable_pty::tests::a_blocked_shell_writer_does_not_stall_other_shells has been running for over 60 seconds
portable_pty::tests::manager_routes_input_resize_and_cleanup_by_shell_id has been running for over 60 seconds
```

Both spawn a real `$SHELL` into a real PTY and then call `stop`. On macOS the killed child is not reaped, `wait()` never returns, and neither test carries a deadline of its own to escape it.

This went unseen because macOS ran no Rust tests at all until `fix-sqlite-deferred-write-upgrade-contention` added them. The first real macOS test run is what found it. The defect is older than that change and lives in a different context, so it gets its own branch off `main` and its own review rather than being grafted onto a storage fix.

The user-visible shape of the same bug: a shell whose child stops responding to `kill` makes session switch, archive, delete, and application exit hang with no diagnostic — the caller cannot distinguish "still reaping" from "wedged forever", because the code has no vocabulary for the difference.

## What Changes

* **Replace the unbounded `wait` with bounded `try_wait` polling against a monotonic deadline.** `portable_pty::Child::try_wait` is documented not to block; it is the primitive this code should have used.
* **Name every termination outcome.** `AlreadyExited`, `KillRequested`, `Reaping`, `Reaped`, `ReapTimedOut`, `KillFailed`, `ReapFailed` become a closed enum. A timeout is a distinct answer, not a shrug.
* **Never report a timeout as termination.** `ReapTimedOut` returns a stable `reap_timed_out` code and keeps cleanup ownership recorded, so an unreaped child stays visible instead of being dropped on the floor while the caller is told it succeeded.
* **Keep kill and reap outside the registry lock.** The manager's routing map must never be held across process work, so one wedged shell cannot stall input, resize, or cleanup for any other shell.
* **Make repeated termination idempotent, and single-flight.** A second `stop` on the same shell must not start a second reap of the same child.
* **Test every branch with a fake `Child`/`ChildKiller`,** including an assertion that the blocking `wait()` is never reached from a production path — the fake fails the test if it is called.

Explicitly out of scope, and forbidden in this change: fixed sleeps as a synchronisation device, test-level retry, `#[ignore]` or skipped tests, and any raise of a CI timeout. Each of those hides this defect instead of removing it; the previous ceiling raise is what proved that, since 169 minutes bought nothing but more silence.

## Impact

* Affected specs: `session-shell`
* Affected code: `src-tauri/src/contexts/workspaces/infrastructure/portable_pty.rs`
* Unblocks: macOS `cargo test --workspace`, which is the last open gate on `fix-sqlite-deferred-write-upgrade-contention` and therefore on `add-unified-extension-platform` Task Group 4. Neither of those is archived or unblocked by this change alone — they re-qualify on all three platforms after this lands on `main`.
