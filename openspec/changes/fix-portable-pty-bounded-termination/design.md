# Design

## The defect, precisely

Two call sites block without a bound, and they contend for the same lock.

`terminate_shell` (`portable_pty.rs:130`):

```rust
if let Ok(mut killer) = shell.killer.lock() {
    let _ = killer.kill();
}
if let Ok(mut child) = shell.child.lock() {
    if child.wait().is_err() { /* warn */ }
}
```

`start_exit_monitor` (`portable_pty.rs:231`):

```rust
let wait_failed = child.lock().map_or(true, |mut child| child.wait().is_err());
```

The monitor takes the `child` mutex and holds it across a blocking `wait()`. If the child never exits, that guard is never released, and `terminate_shell` parks on `child.lock()` forever — before it ever reaches its own unbounded `wait()`. Two independent ways to hang, one of which is invisible in a stack trace of the other.

`clone_killer` exists in `portable-pty` precisely so a killer can be used "from a thread that may be blocked in `.wait`". The code already clones the killer. It just never bounded the wait that the killer was cloned to work around.

### Why macOS and not Windows or Linux

Not established, and this change does not need it established. `try_wait` is correct on every platform regardless of which one currently exposes the hang: a reap that cannot be bounded is a defect wherever it runs, and the platform that happens to reveal it is an accident of scheduling and PTY teardown order. Recording a root cause we have not proven would be worse than recording the bound we can prove.

What is established: the same runner passed 13 SQLite concurrency tests nine minutes before the hang, so this is not a general macOS runner failure.

## Decisions

### The bound is a monotonic deadline, not a retry count

`Instant` is monotonic and unaffected by wall-clock adjustment. A retry count would be a bound on *iterations*, which says nothing about elapsed time once the machine is loaded — and CI machines are loaded. The deadline is computed once at entry and every poll compares against it.

### Backoff is not a fixed sleep

The forbidden pattern is sleeping a fixed span in the hope that a race has resolved, then proceeding as if it had. That is a synchronisation device standing in for a fact.

Polling backoff is a different thing: each iteration reads the *actual* state via `try_wait`, the loop terminates on a real answer or a real deadline, and the interval only controls how often the true state is sampled. The interval doubles from 1 ms to a 25 ms ceiling so that a fast exit — the overwhelmingly common case — is observed in about a millisecond, while a wedged child costs a handful of syscalls per second instead of a spin.

No test asserts on elapsed time, and no test sleeps to let something happen.

### A timeout is its own outcome

```rust
enum ShellTerminationOutcome {
    AlreadyExited,   // try_wait answered before any signal was sent
    KillRequested,   // signal delivered, not yet observed to have taken effect
    Reaping,         // a reap is already in flight for this child
    Reaped,          // observed exit
    ReapTimedOut,    // deadline reached with no observed exit
    KillFailed,      // the signal itself was refused
    ReapFailed,      // try_wait reported an error
}
```

`ReapTimedOut` renders as the stable code `reap_timed_out`. It is not folded into success, and it is not folded into a generic error, because the two demand different responses: a failed kill may be retried, while a timed-out reap means a live process is still out there and something must keep owning it.

**Cleanup ownership survives a timeout.** On `ReapTimedOut` the shell is not silently discarded — the outcome is logged with session and shell context and the caller is told `reap_timed_out`, so the evidence that a child was left unreaped exists rather than being erased by a cheerful return value. This is the same principle as the SQLite change's refusal to let a rule pass by not looking: a cleanup that reports success without having cleaned up is worse than one that fails, because it is also evidence.

### Single-flight, so a second stop cannot start a second reap

Termination state lives in an `AtomicU8` on the shared `ShellTermination`, transitioned with `compare_exchange`. The first caller to move it out of `Idle` owns the reap; a concurrent caller observes `Reaping` and returns immediately rather than queueing on the child mutex. Repeated termination is therefore idempotent *and* non-blocking, which the previous code achieved for neither.

This also removes the monitor-versus-terminate contention: the monitor participates in the same state machine instead of holding the child lock across a blocking call.

### The registry lock is never held across process work

`stop`, `stop_for_session`, `insert`, and `Drop` all take the shell out of the map first and release the map lock before touching the child. The existing code was already careful here — `stop` relies on the temporary guard dropping at the end of the statement — but "already correct by accident of temporary lifetime" is not a property a later edit will preserve. The shells are now explicitly collected and the guard explicitly dropped, and a test asserts the property directly.

### Fakes, and the assertion that matters most

`FakeChild` implements `portable_pty::Child` over scripted `try_wait` answers. Its `wait()` — the blocking one — sets a flag and panics. Every production path is exercised through it, so if any future edit reintroduces a blocking wait, the test suite fails immediately and by name rather than by hanging until a CI ceiling cancels it.

That is the real repair. Bounding today's wait fixes today's hang; making the blocking call unreachable from production and *proving* it fixes the class.
