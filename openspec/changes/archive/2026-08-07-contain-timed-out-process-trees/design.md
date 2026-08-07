## Context

`platform::process::output_with_control` is the native runtime's bounded execution path: spawn a command, stream its output, enforce a timeout, honor cancellation. Every non-interactive external command goes through it — CLI detection, npm package operations, SDK and extension installs, plugin integration tools, loop verification, and the agent's shell tool.

It spawns with a plain `std::process::Command`, so on timeout or cancellation `child.kill()` reaches only the process it launched. Anything that process spawned is orphaned.

The codebase already has containment: `ManagedChild` wraps commands in a Windows Job Object (`KillOnCloseJobObject`) or a Unix process group. Reusing it directly does not work, and the reason is the central constraint of this design:

- `KillOnCloseJobChild::try_wait` (`windows_job.rs`) returns `Ok(None)` until the whole job drains. A successful command that leaves a background process would never be seen as finished and would be reported as `TimedOut`.
- `KillOnCloseJobObject` sets `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (`windows_job.rs:91`), so descendants are killed when the job handle drops — even for a command that succeeded.

Both behaviors are correct for `ManagedChild`'s callers (MCP relay children, long-lived managed processes, which are supervised as a tree and torn down as a tree). Both are wrong for a one-shot "run it and collect output" call.

This is a native-layer change only. It sits below every bounded context, is reached through no Tauri command directly, and crosses no runtime adapter boundary. The Web/mock adapter is unaffected because the browser runtime executes no external processes.

## Goals / Non-Goals

**Goals:**

- Terminating a timed-out or cancelled command reaches its descendants.
- The completion decision keeps its current scope: the launched process exiting means done.
- A command that succeeds leaves its descendants alone.
- No change to command construction, timeouts, collected output, error variants, or any Tauri command signature.

**Non-Goals:**

- Changing `ManagedChild` / `ManagedTokioChild` semantics. Their tree-scoped wait and kill-on-close are deliberate and stay as they are.
- Reaping processes that escape containment by design (Windows `CREATE_BREAKAWAY_FROM_JOB`, Unix `setsid`). Containment is best-effort against ordinary children.
- Touching `spawn_detached`, which exists precisely to launch processes that must outlive the call.
- Revisiting output collection, which was already bounded separately so a lingering descendant holding the pipe cannot block the collector.

## Decisions

### Separate the kill scope from the wait scope

The containment primitive is used only as a *kill handle*. Completion is still decided by `try_wait` on the directly launched process.

Alternative considered: reuse `KillOnCloseJobObject` as-is. Rejected — it inverts the wait scope and turns successful commands with background children into timeouts. This is the failure that motivated the whole change.

Alternative considered: keep the plain spawn and enumerate descendants by walking the process table on timeout. Rejected — racy (PID reuse, processes spawned during the walk), platform-specific in a messier way, and duplicates what the OS primitives already guarantee.

### Windows: a job object that terminates explicitly and does not kill on close

Add a containment wrapper alongside `KillOnCloseJobObject` that:

- assigns the child to a job, as today;
- does **not** set `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so dropping the handle after a successful run leaves descendants alive;
- delegates `try_wait` and `wait` to the inner child rather than gating on job completion;
- terminates the job in `start_kill`, so the timeout and cancellation paths take out the tree.

The two wrappers should share their job-creation and assignment code; only the limit flag, the wait delegation, and the drop behavior differ.

### Unix: process group, verified before relying on it

`ProcessGroupChild::try_wait` (`process_group.rs:185`) tries the group first and *falls back to the inner child*, so unlike the Windows job it can still observe the direct child's exit. The existing `ProcessGroup::leader()` wrapper is therefore a plausible fit as-is, with group-scoped signalling already available for the kill path.

This must be confirmed by test rather than assumed, since the fallback ordering is the only thing preventing the Windows-style failure mode. If it does not hold, the fallback is to set the process group via `CommandExt::process_group(0)` and signal the negated pgid directly on the kill path, keeping `std`'s unmodified `try_wait`.

### Failure to establish containment is a launch failure

If the job or group cannot be established, the command must not silently run uncontained, and a process that was already started must not be left unsupervised. The existing `KillOnCloseJobObject` already handles this case by killing and reaping the suspended child; the new wrapper follows the same pattern.

## Risks / Trade-offs

- **The new Windows wrapper diverges from the tested one, so its wait path is unproven** → It needs its own tests asserting the opposite of `managed_child_wait_does_not_finish_while_a_descendant_survives`: a command whose descendant survives must be reported as *finished*. Both wrappers keep tests, and the contrast between them is the point.
- **Killing the tree on timeout is more destructive than today** → That is the intended behavior, but it means a timed-out command now takes down background processes it started. Acceptable: the command failed, and the alternative is the current unbounded orphan leak. `spawn_detached` remains available for deliberately detached launches.
- **Unix and Windows termination differ in observable ordering** → Cover both explicitly rather than testing one and assuming the other; the existing suite already has per-platform tests for `ManagedChild` to model this on.
- **Job objects nest** → A VaneHub process already inside a job (some CI agents, some debuggers) can fail job assignment. Handled by the launch-failure path above, and nested jobs are supported on Windows 8+, which is below the app's baseline.

## Migration Plan

No data migration, no schema change, no persisted state. The change is a drop-in replacement of the spawn and kill calls inside one function.

Rollback is reverting the commit: nothing outside `platform::process` observes the difference except through improved cleanup.

## Open Questions

- Does `ProcessGroupChild::try_wait`'s inner-child fallback hold in practice for a child that exits while its descendant survives? The implementation reads that way; a test decides whether the Unix path needs the manual `process_group(0)` fallback described above.
