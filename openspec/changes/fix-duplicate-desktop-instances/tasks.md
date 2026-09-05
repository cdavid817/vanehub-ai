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
- [ ] 2.5 Verify on a packaged release client that a second launch surfaces the running window instead of starting a second process, including while the window is hidden in the tray

## Notes

Task 2.5 is the one check that cannot be automated here, and it needs a *release* build: debug builds deliberately do not claim the lock. The guard's mechanism was read end to end in the plugin's Windows implementation — the duplicate claims the identifier-keyed mutex, fails with `ERROR_ALREADY_EXISTS`, hands its argv to the running instance over `WM_COPYDATA`, and calls `std::process::exit(0)` — and the callback it triggers is covered by the wiring assertions. What remains unverified is the observed behavior of a real double-click.
