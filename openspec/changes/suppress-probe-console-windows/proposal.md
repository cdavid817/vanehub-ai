## Why

Launching the packaged Windows application raised a modal error dialog for each capability probe it runs at startup — `[出现错误 2147942632 (0x800700e8) (启动""where" claude"时)]`, and the same for `"node" --version`. Each dialog had to be dismissed by hand before the application was usable.

Process capture confirmed the app spawns 22 console-subsystem descendants during startup (`where node`, `where wt`, `where git`, `reg query …App Paths\…`, `node --version`), each with its own `conhost.exe`. The app is a GUI-subsystem process with no console to inherit, so Windows allocates one per child. When a launch then fails, the allocated console host — not the application — reports the failure in a modal dialog the application cannot suppress, log, or recover from.

`CREATE_NO_WINDOW` was absent from the entire native codebase.

## What Changes

- Windows process construction suppresses the child console window, so a console-subsystem child never gets a console allocated and no console host can raise UI on the application's behalf.
- A failing probe surfaces as a handled `io::Error` on the application's own error path instead of a modal dialog.
- An architecture test guards both command constructors, so a future constructor cannot silently reintroduce the dialog.

`spawn_detached` is deliberately unchanged: it passes `DETACHED_PROCESS`, and Windows ignores `CREATE_NO_WINDOW` when that flag is present.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-runtime-architecture`: guarded external command execution gains a requirement that process construction must not let the operating system attach interactive console UI to a child, so a failed launch stays on the application's error path.

## Impact

**Runtime scope: desktop only.** Web/mock runtime is unaffected — it spawns no processes. No React component, frontend service interface, Tauri adapter, or Tauri command signature changes, so no runtime adapter boundary moves.

Affected files:

- `src-tauri/src/platform/process/mod.rs` — `std_command` and `tokio_command`
- `src-tauri/tests/architecture.rs` — guard test

Every native caller benefits without changing, because `runtime_processes_and_append_logs_use_shared_adapters` already forbids constructing a `Command` outside `platform/process/mod.rs`.

This does not explain why a probe launch fails with `ERROR_NO_DATA` on an affected machine. That failure is environment-specific, was never reproducible on a second Windows machine across four attempts, and remains open — the change removes the unhandleable dialog, not the underlying launch failure.
