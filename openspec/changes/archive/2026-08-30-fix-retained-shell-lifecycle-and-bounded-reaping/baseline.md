# Baseline: configured Shell limits and timing

Recorded so the fix can be judged against what was already there. **None of these numbers were
raised as part of this change.** A lifecycle fix that also widened a ceiling would leave nobody able
to say which of the two made the symptom go away — and a ceiling raised to make a leak survivable is
a leak that now takes longer to notice.

Every value below is the `Default` impl in the file named. They are defaults rather than settings:
nothing reads them from configuration today, and the tests that need a different number construct
their own rather than mutating a global.

## Capacity

`src-tauri/src/contexts/workspaces/application/session_shell.rs` — `ShellCapacities::default`

| Limit | Value | What it bounds |
| --- | --- | --- |
| `per_session` | 8 | Live Shells one session may hold |
| `total` | 32 | Live Shells the process may hold |

A Shell occupies its lease from the moment capacity is reserved until cleanup is *confirmed*, which
includes `Closing`, `Reaping` and `CloseFailed`. That is the point of the intermediate states: a
Shell whose process may still exist has not given its slot back.

## Close budget

`src-tauri/src/contexts/workspaces/domain/session_shell_lifecycle.rs` — `ShellCloseBudget::default`

| Stage | Value | What it waits for |
| --- | --- | --- |
| `graceful` | 150 ms | A shell that was already finishing, after input is stopped |
| `terminate` | 600 ms | The process, after a termination request |
| `force` | 600 ms | The operating system to reap, after a forceful kill |
| `worker` | 250 ms | Workers to report themselves complete — never a blocking join |
| `total` | 1 800 ms | The whole command path |
| `poll` | 10 ms | How often an observation is retried inside a stage |

The stages sum to 1 600 ms, inside the 1 800 ms total. That relationship is asserted by
`the_default_close_budget_bounds_the_command_path`, because a caller stating "close returns within
`total`" would otherwise be wrong on the slowest path.

## Reaper

`src-tauri/src/contexts/workspaces/application/session_shell_reaper.rs` —
`ShellReaperLimits::default`

| Limit | Value | What it bounds |
| --- | --- | --- |
| `queue_capacity` | 32 | Shells waiting for cleanup at once |
| `max_active_per_drain` | 4 | Attempts one drain may make |
| `max_attempts` | 5 | Automatic attempts before a Shell is left `CloseFailed` for a person |
| `initial_backoff_millis` | 250 | First wait between attempts |
| `max_backoff_millis` | 8 000 | Ceiling on that wait |

`queue_capacity` is sized against the Shell ceiling itself: every live Shell failing to close at the
same time still fits, and nothing beyond that is reachable. A full queue is therefore safe to refuse
— ownership stays with the caller rather than being dropped somewhere with no owner.

## Retained output

`src/types/session-workspace-shell-frames.ts` — `SHELL_RETAINED_OUTPUT_BYTES` = 1 MiB per Shell,
enforced by the native registry. The Web mock retains 4 096 characters instead
(`MOCK_SHELL_RETAINED_CHARACTERS`), deliberately far below it: a browser demo that had to produce a
megabyte before a gap appeared would never show one, and the gap marker is behaviour the mock exists
to exercise.

## Remote helper

`src-tauri/src/contexts/workspaces/infrastructure/remote_helper/protocol.rs` —
`HELPER_TIMEOUT_SECONDS` = 20 s bounds one exchange with a remote host. It is the *exchange*
timeout, not a walk deadline; the walk carries its own, sent from the shared inspection budget.

## What is not configured anywhere

There is no idle sweep interval and no shutdown grace constant in this subtree. Shells are reclaimed
on explicit close, on the session-done edge (archive and delete), and at process shutdown — each of
which is an event rather than a timer. Recorded here because an absent constant and an unfound one
look identical to somebody reading a task list.

## Where the close budget is actually spent

Recorded after the desktop layer was run, because the numbers above say what the stages *allow* and
not what a real close *uses*. On Windows a PowerShell child under a pseudoconsole is reaped inside
the graceful stage — input closes, the shell notices, it exits — so `terminate` and `force` are
normally untouched.

The worker stage is the one that mattered. Before the terminal was released ahead of it, the reader
never completed and every close spent the full 250 ms and then reported `Reaping`. It now completes
as soon as the master drops. That is the whole reason the ordering is stated in the fact map rather
than left as an implementation detail: the budget was never too small, it was being spent waiting
for something this code was preventing.
