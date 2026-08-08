## 1. Guard the behaviour before changing it

- [x] 1.1 Add `windows_command_constructors_suppress_console_windows` to `src-tauri/tests/architecture.rs`, asserting both `std_command` and `tokio_command` suppress the child console window
- [x] 1.2 Add a guard on the flag value itself so a typo cannot silently reintroduce the visible console
- [x] 1.3 Confirm the new test fails against the unchanged constructors

## 2. Suppress the console window

- [x] 2.1 Add the `CREATE_NO_WINDOW` constant and the `SuppressConsoleWindow` trait to `src-tauri/src/platform/process/mod.rs`, implemented for both the std and tokio builders and a no-op off Windows
- [x] 2.2 Call it from `std_command` and `tokio_command`
- [x] 2.3 Leave `spawn_detached` unchanged, and record why in a comment: Windows ignores `CREATE_NO_WINDOW` alongside `DETACHED_PROCESS`

## 3. Verification

- [x] 3.1 Run `cargo test --manifest-path src-tauri/Cargo.toml --test architecture` and confirm the new guard passes
- [x] 3.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 3.3 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 3.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`, re-running any load-sensitive failure in isolation before treating it as a regression
- [x] 3.5 Build a release binary and have the reporter confirm on the affected machine that the dialogs no longer appear
- [x] 3.6 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run docs:check`, and `openspec validate --specs --strict`
- [x] 3.7 Run `openspec validate suppress-probe-console-windows --strict`

## 4. Follow-up

- [ ] 4.1 Ask the reporter whether CLI detection now reports Claude Code and Node as available. If it does not, the probes were failing all along and `ERROR_NO_DATA` needs its own investigation rather than staying an open question
