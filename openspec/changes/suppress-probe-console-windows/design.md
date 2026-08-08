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

### One trait rather than two differently-named helpers

The std and tokio builders both expose `creation_flags` on Windows, but through unrelated types with no shared bound. A small private `SuppressConsoleWindow` trait gives one name at both call sites and one obvious home for the non-Windows no-op. The neighbouring `network::apply_to_std_command` / `apply_to_tokio_command` pair shows the two-function alternative; a single name was preferred here because the guard test asserts on it, and one name keeps the assertion precise rather than pattern-matching a prefix.

### Leave `spawn_detached` alone

It passes `DETACHED_PROCESS`, and Windows ignores `CREATE_NO_WINDOW` when `DETACHED_PROCESS` or `CREATE_NEW_CONSOLE` is present. Adding it there would be dead configuration that implies a protection the flag combination does not provide.

### Guard with an architecture test, not a behavioural one

The guard parses `platform/process/mod.rs` and asserts both constructors call `suppress_console_window`, following the existing `syn`-based tests in `tests/architecture.rs`.

A behavioural test was considered and rejected. `std::process::Command` exposes no way to read back its creation flags, and asserting on window visibility is not sound: the probes live for tens of milliseconds, so polling either misses them or reports a false negative. Two attempts at exactly that measurement — `MainWindowHandle` at 700 ms and `EnumWindows`/`IsWindowVisible` filtered to console window classes at 50 ms — both returned zero against the *unfixed* binary, on a machine where the defect does not manifest. A test that cannot fail on broken code is worse than no test.

## Risks / Trade-offs

- **The fix was verified on the affected machine, not by an automated test.** → The reporter confirmed the dialogs are gone with a release build carrying the change. Four reproduction attempts on a second Windows machine were all negative, so no local A/B was possible. The causal chain is nonetheless direct: the dialog is raised by the console host, the console host exists only because a console was allocated, and `CREATE_NO_WINDOW` prevents the allocation.
- **A failing probe is now silent where it used to be loud.** → It becomes a handled `io::Error`. If probes are genuinely failing on a machine, the visible consequence shifts to CLIs reporting as unavailable, which is the correct place for it but is easier to overlook than a modal dialog.
- **The guard is structural, so a constructor that calls the helper on a path that never executes would still pass.** → Accepted. The test's job is to stop a new constructor being added with no suppression at all, which is how this defect arrived.

## Open Questions

- Why does launching `where claude` or `node --version` fail with `ERROR_NO_DATA` on the affected machine? Something is interfering with process creation there — a security product hooking `CreateProcess` and handle pressure are both consistent with the error, neither is confirmed. Worth its own investigation: if the probes genuinely fail, CLI detection has been silently wrong on that machine, and this change hides the last visible symptom of it.
