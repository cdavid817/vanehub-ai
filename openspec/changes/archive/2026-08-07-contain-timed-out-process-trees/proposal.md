## Why

When the native runtime terminates an external command for exceeding its timeout or being cancelled, it kills only the direct child. Any process that child spawned survives as an orphan. This affects every caller of the shared bounded-execution path — CLI detection, npm package operations, SDK and extension installs, plugin integration tools, loop verification, and the agent's own shell tool — so a single timed-out `npm install` or agent-issued build command can leave background processes running for the rest of the app's lifetime.

The obvious fix — reusing the existing `ManagedChild` containment wrapper — is not viable: its job-aware `try_wait` reports "not finished" until the entire process tree drains, which would reclassify a *successful* command that leaves a background process as a timeout. Termination and completion therefore need different scopes, and that distinction deserves to be stated as a requirement rather than left as an implementation accident.

## What Changes

- External commands run through the bounded-execution path SHALL be spawned into a platform containment primitive (Windows Job Object, Unix process group) so the runtime can reach their descendants.
- The timeout and cancellation paths SHALL terminate the whole contained tree instead of only the direct child.
- The completion decision SHALL keep its current scope: a command is finished when the process the runtime launched exits, regardless of whether descendants outlive it. This preserves today's behavior for commands that intentionally leave background processes.
- No change to command construction, argument handling, timeout durations, returned output, or error variants. No change to any Tauri command name or signature.
- Not breaking: callers observe identical results for every command that does not orphan a descendant, and strictly better cleanup for those that do.

## Capabilities

### New Capabilities

None. This tightens an existing native runtime guarantee rather than introducing a new capability.

### Modified Capabilities

- `native-runtime-architecture`: adds a requirement that bounded external command execution terminate the whole process tree on timeout or cancellation, while continuing to decide completion from the launched process alone. The existing "Guarded external command execution" requirement covers only how invocations are *constructed*; nothing currently constrains what happens to descendants when one is torn down.

## Impact

**Runtime scope: desktop only.** The Web runtime executes no external processes, so its adapter contract is untouched.

- Affected code: `src-tauri/src/platform/process/mod.rs` (bounded execution path), `src-tauri/src/platform/process/windows_job.rs` (needs a containment variant whose wait is not tree-scoped), `src-tauri/src/platform/process/managed_child.rs` (shares the containment helpers).
- Frontend/backend isolation: unaffected. This is entirely inside the native platform layer, below every bounded context, and crosses no runtime adapter boundary.
- No new dependencies: `process_wrap` and the Windows job primitives are already in use.
- No database schema or migration impact.
- Platform risk concentrates on Windows and Unix termination differing; both paths need explicit coverage because the existing tree-kill tests assert the *tree-scoped wait* semantics this change deliberately avoids.
