## Context

The application is built with the Windows GUI subsystem, so it owns no console. `platform/process/mod.rs` is the only place allowed to construct a `Command` — `runtime_processes_and_append_logs_use_shared_adapters` in `tests/architecture.rs` enforces that — and everything funnels through `std_command` or `tokio_command`. Neither set any Windows creation flags, and `CREATE_NO_WINDOW` appeared nowhere in the native codebase.

Startup runs a burst of capability probes. A clean capture, filtered by process ancestry to descendants of `vanehub-ai.exe`, recorded 22 console-subsystem children: `where node`, `where wt`, `where git`, `reg query "HK*\…\App Paths\{wt,idea64,webstorm64}.exe" /ve`, `node --version`, each paired with its own `conhost.exe`.

The reported failure was one modal dialog per probe, naming the exact probe command line, requiring manual dismissal:

```
出现错误 2147942632 (0x800700e8) (启动""where" claude"时)
出现错误 2147942632 (0x800700e8) (启动""node" --version"时)
```

`0x800700E8` is `ERROR_NO_DATA` — the pipe is being closed.

## Goals / Non-Goals

**Goals:**

- No process the application starts can put a window on the user's desktop.
- A launch failure reaches the calling native code as an error it can log and act on.
- The guarantee survives the next command constructor someone adds.

**Non-Goals:**

- Explaining `ERROR_NO_DATA`. See Open Questions.
- Reducing how many probes startup runs, or caching their results.
- Changing `spawn_detached`.

## Decisions

### Suppress the console at the two shared constructors

`CREATE_NO_WINDOW` (`0x0800_0000`) is applied in `std_command` and `tokio_command`. Because the architecture test already forbids constructing a `Command` anywhere else, two call sites cover every caller, and no feature module needs to remember anything.

The alternative — applying the flag at each spawn site — was rejected for the same reason the funnel exists: a guarantee enforced at 40 call sites is a guarantee that lapses at the 41st.

### Every `creation_flags` call must carry the suppression itself

Constructing the flag at the funnel is necessary but not sufficient. `CommandExt::creation_flags` **replaces** the creation-flag word rather than merging into it, so any later call discards what an earlier one set.

Both Windows job wrappers set `CREATE_SUSPENDED` in `pre_spawn`, which runs after the constructor and immediately before the spawn. `TerminateTreeJobObject` is installed by `add_execution_containment` on every bounded execution — which is the exact path every startup capability probe takes. Suppressing at the constructor alone therefore had no effect at all on the probes that raised the dialogs, and a first attempt that did only that was reported still broken.

Each wrapper now passes `CREATE_SUSPENDED.0 | CREATE_NO_WINDOW`, and the guard below covers every such call rather than only the constructors.

### One trait rather than two differently-named helpers

The std and tokio builders both expose `creation_flags` on Windows, but through unrelated types with no shared bound. A small private `SuppressConsoleWindow` trait gives one name at both call sites and one obvious home for the non-Windows no-op. The neighbouring `network::apply_to_std_command` / `apply_to_tokio_command` pair shows the two-function alternative; a single name was preferred here because the guard test asserts on it, and one name keeps the assertion precise rather than pattern-matching a prefix.

### Leave `spawn_detached` alone

It passes `DETACHED_PROCESS`, and Windows ignores `CREATE_NO_WINDOW` when `DETACHED_PROCESS` or `CREATE_NEW_CONSOLE` is present. Adding it there would be dead configuration that implies a protection the flag combination does not provide.

### Guard with an architecture test, not a behavioural one

Two guards, because the first one alone let the real defect through. One parses `platform/process/mod.rs` and asserts both constructors call `suppress_console_window`. The second scans every `creation_flags` call under `platform/process/` and requires each to name `CREATE_NO_WINDOW` or `DETACHED_PROCESS`; it reported `windows_job.rs:44` and `windows_job.rs:166` the moment it was written.

The second guard is deliberately a line scan rather than a `syn` visitor. Rendering a parsed argument expression back to text needs `quote`, which is not a dependency, and the property being asserted — "this call names the flag" — is a property of the written line.

A behavioural test was considered and rejected. `std::process::Command` exposes no way to read back its creation flags, and asserting on window visibility is not sound: the probes live for tens of milliseconds, so polling either misses them or reports a false negative. Two attempts at exactly that measurement — `MainWindowHandle` at 700 ms and `EnumWindows`/`IsWindowVisible` filtered to console window classes at 50 ms — both returned zero against the *unfixed* binary, on a machine where the defect does not manifest. A test that cannot fail on broken code is worse than no test.

## Risks / Trade-offs

- **The fix was verified on the affected machine, not by an automated test.** → The reporter confirmed the dialogs are gone using a `tauri build` package carrying the complete change. Four reproduction attempts on a second Windows machine were all negative, so no local A/B was possible. The causal chain is nonetheless direct: the dialog is raised by the console host, the console host exists only because a console was allocated, and `CREATE_NO_WINDOW` prevents the allocation.
- **An earlier confirmation of the incomplete fix was a false positive, and the verification artifact caused it.** → It was obtained from a bare `cargo build` binary. Tauri's dev/production switch is driven by the Tauri CLI, not the cargo profile, so such a binary loads from `devUrl` and shows `ERR_CONNECTION_REFUSED` against `127.0.0.1:5174` instead of the UI. The failing probe is frontend-triggered, so with no UI it never ran and the dialogs could not appear. **Only a `tauri build` artifact can verify runtime behaviour.** CI is unaffected: `package.yml` invokes `tauri build` through the `package:*` npm scripts.
- **A failing probe is now silent where it used to be loud.** → It becomes a handled `io::Error`. If probes are genuinely failing on a machine, the visible consequence shifts to CLIs reporting as unavailable, which is the correct place for it but is easier to overlook than a modal dialog.
- **The guard is structural, so a constructor that calls the helper on a path that never executes would still pass.** → Accepted. The test's job is to stop a new constructor being added with no suppression at all, which is how this defect arrived.

## Open Questions

- Why does launching `where claude` or `node --version` fail with `ERROR_NO_DATA` on the affected machine? Something is interfering with process creation there — a security product hooking `CreateProcess` and handle pressure are both consistent with the error, neither is confirmed. Worth its own investigation: if the probes genuinely fail, CLI detection has been silently wrong on that machine, and this change hides the last visible symptom of it.
