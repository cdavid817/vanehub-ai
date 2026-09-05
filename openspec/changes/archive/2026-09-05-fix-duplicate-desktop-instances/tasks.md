## 1. Single-Instance Guard

- [x] 1.1 Add bootstrap wiring assertions proving the desktop builder registers the guard ahead of every other plugin, that a duplicate launch restores the running window, and that the relay dispatch still precedes desktop bootstrap
- [x] 1.2 Add `tauri-plugin-single-instance` to the workspace and native crate dependencies
- [x] 1.3 Register the guard as the first plugin in the desktop builder chain, claimed by release builds only
- [x] 1.4 Restore the running instance's main window on a duplicate launch by showing, unminimizing, and focusing it, recording a redacted warning through unified logging when restoration fails
- [x] 1.5 Restart through `request_restart` on the update path, so the successor is not spawned while the lock is still held and this application's own exit handlers still run

## 2. Verification

- [x] 2.1 Run `cargo fmt`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`
- [x] 2.2 Run `npm run lint:ci`, `npm run test`, and `npm run build` to confirm the change stays native-only
- [x] 2.3 Run `npm run native:panic:check`, `npm run architecture:check`, `openspec validate --specs --strict`, and `openspec validate fix-duplicate-desktop-instances --strict`
- [x] 2.4 Confirm the wiring assertions fail when the guard is removed, and that they scan code with prose stripped so a comment naming the plugin cannot stand in for the registration
- [x] 2.5 Verify on a release client that a second launch exits instead of starting a second process, and that the original keeps running

## Notes

Task 2.5 was verified against a real release build on Windows (`target/release/vanehub-ai.exe`, `tauri build --no-bundle`, 64m), launched twice against an isolated `VANEHUB_APP_DATA_DIR`:

```
first launch  pid=2516   still running: true
second launch pid=29084  exit code: 0
```

The two launches were tracked by pid rather than by counting processes named `vanehub-ai.exe`. Other clients were running on the machine at the time, and a name count would have folded them in; they are older builds carrying no guard, so they hold no lock and could not affect the result either way.

The mechanism behind that result was read end to end in the plugin's Windows implementation: the duplicate claims the identifier-keyed mutex, fails with `ERROR_ALREADY_EXISTS`, hands its argv to the running instance over `WM_COPYDATA`, and calls `std::process::exit(0)`.

What this run does not observe is the window actually being raised, or the tray-hidden case — both need a person at the screen. The restoration path itself is covered by the wiring assertions.
