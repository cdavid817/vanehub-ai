# Fact map: where a Session Shell's life actually happens

Written from the code as it stands after this change, so it can be checked rather than believed.
Each row names the file that owns the decision, because the recurring failure in this area was two
places deciding the same thing and disagreeing.

## Ownership at a glance

| Concern | Owner |
| --- | --- |
| Lifecycle states and legal transitions | `contexts/workspaces/domain/session_shell.rs` |
| Generation, reason codes, close budget | `contexts/workspaces/domain/session_shell_lifecycle.rs` |
| Registration, capacity, close orchestration, sweep | `application/session_shell_registry.rs` |
| The entry, its replay buffer, its attachment | `application/session_shell_store.rs` |
| Atomic capacity reservation | `application/session_shell_capacity.rs` |
| The bounded retry queue | `application/session_shell_reaper.rs` |
| Local PTY acquisition and close | `infrastructure/retained_shell_runtime.rs` (+ `_process`, `_support`) |
| Remote channel and the local/remote route | `infrastructure/retained_remote_shell.rs` |
| Frontend contract | `src/types/session-workspace-shell-frames.ts`, `src/services/session-shell-service.ts` |

## Opening

1. `create` takes a per-identity gate, so two tabs asking for the same default Shell serialize and
   the loser re-reads rather than spawning a second process.
2. Capacity is reserved **before** anything external exists. The count-then-open version let every
   concurrent request see the same free slot.
3. The Shell is registered as `Opening` **before** the runtime is invoked, so the reader thread's
   first byte lands in an entry that already exists. `Opening` is addressable and not writable.
4. `RoutedShellRuntime` claims the id for this generation **before** calling the underlying runtime.
   A remote worker publishes the moment its channel is up, and a route recorded afterwards left that
   window falling through to the *local* runtime.
5. `LocalLaunchGuard` owns the child, killer, PTY handles, writer, and workers from first
   acquisition. Every failure path rolls it back explicitly and reports whether cleanup confirmed.
6. The transition to `Running` is conditional on the state the runtime reports. A Shell that echoed
   and exited before this line has already reached a terminal state, and writing `Running` over it
   would leave a dead process reported as live.

**Two endings for a failed start.** Confirmed cleanup finalizes and returns the slot. Unconfirmed
cleanup keeps the Shell — `Reaping` if the Reaper took it, `CloseFailed` if its queue was full —
because returning the slot while a child may still hold a thread is the defect this change is about,
one step earlier. Nothing is published either way: no `ShellOpened` was reported, so a `ShellClosed`
would be an ending for something that never began.

## Operating

`write` and `resize` require the current attachment and a `Running` state. `rename` requires only
that the entry is not being torn down: it reaches no runtime, so an ended Shell whose transcript a
reader is keeping can still be relabelled.

Attach registers the listener **before** claiming the Shell and replays what arrived in between.
Doing it the other way round loses every frame in that window with contiguous sequence numbers, so
nothing downstream can detect the loss.

## Closing

`close` stops input, waits for the child within the graceful, terminate and force stages, **then
releases the terminal**, and only then waits for the workers. The release is not tidiness and its
position is not arbitrary: on Windows the pseudoconsole keeps its output pipe open while any handle
to it lives, so the reader sees EOF when the last master handle drops rather than when the child
exits. Waiting for that worker while still holding the master waits for something this code is
itself preventing, and every ordinary close on Windows timed out into `Reaping` — a Shell the reader
pressed Close on and could not make go away.

Having settled that, `close` settles the Shell:

- **Confirmed / NotHeld** → finalize, publish `ShellClosed`, release the slot.
- **Retained** → hand to the Reaper. `Reaping` when it was taken, `CloseFailed` when the queue was
  full. Both keep the entry, the handles, the replay buffer, and the capacity lease.

The entry is deliberately not removed first. Removing and then killing is what makes a failed close
unrecoverable: the handles are gone, a retry has nothing to retry, and the process has no owner.

The route is removed only on a confirmation for the *same* generation. Removing it unconditionally
sent the user's retry to the local runtime, which found nothing and reported success for a channel
that was still open.

## The Reaper

Drained by whoever already runs a sweep, never by threads of its own, so the number of attempts in
flight is a number somebody chose. A completion for a generation that is no longer current is a
no-op — releasing there returns a slot the current generation is using — and is recorded through
`ShellLifecycleDiagnosticsPort`, because a correct no-op otherwise leaves nothing behind.

Attempts are exhausted rather than retried forever. What is left is `CloseFailed` with ownership
intact and a manual retry that works.

## Session end and application exit

`kill_shells_for_session` closes every Shell a session owns and refuses the archive or delete with
`session_shell_cleanup_incomplete` when any is unconfirmed. The frontend turns that code into the
list of Shells still winding down via `sessionShellCleanupReport`, assembled from `listSessionShells`
rather than from a report crossing two contexts.

`shutdown_session_shells` runs on `RunEvent::Exit`, closes each Shell, advances the Reaper once, and
**records** rather than waits on residual cleanup. Blocking until every child died would make an
unkillable process into an application that cannot be closed.

## What is still not covered

- **Granular local startup failures.** Reader, writer, and worker acquisition failures cannot be
  staged: the runtime reaches `native_pty_system()` directly, so there is no seam a fake can occupy.
  The registry-level failures (open fails, open fails with cleanup pending) are staged and tested.
- **`portable-pty` per-platform behaviour.** The close sequence is tested against fakes and against a
  real PTY on the host that runs the suite. Descendant-process trees are not guaranteed on any
  platform, and no test claims they are.
- **Desktop-layer coverage.** Not run in this environment; results must be reported per platform
  rather than inferred.
