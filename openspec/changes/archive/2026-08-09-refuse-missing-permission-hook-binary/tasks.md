## 1. Refuse to name a binary that is not there

- [x] 1.1 Guard `ClaudeCodeHookAdapter::install` on `wrapper_path.is_file()`, returning a `cli_config` infrastructure error that names the resolved path
- [x] 1.2 Leave `remove` unguarded so a hook installed by an earlier build stays clearable
- [x] 1.3 Record in a comment why this matters: the entries live in Claude Code's global settings and outlive the process

## 2. Tests

- [x] 2.1 Point the existing install test at a real file so it asserts the success path rather than passing by accident
- [x] 2.2 Add a test that an absent binary fails and the projection is never called
- [x] 2.3 Add a test that `remove` still works with the binary absent
- [x] 2.4 Update the projection-failure test to use a present binary, so it still exercises the projection error rather than the new guard

## 3. Tell users

- [x] 3.1 State the limitation in `.github/PREVIEW_RELEASE_NOTES.md`, including that global Claude Code settings are left untouched

## 4. Verification

- [x] 4.1 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 4.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 4.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.4 `npm run lint:ci`, `npm run test`, `npm run build`, `npm run docs:check`
- [ ] 4.5 `openspec validate refuse-missing-permission-hook-binary --strict` and `openspec validate --specs --strict`
  - Change validation passed on 2026-08-09. Main-spec validation is blocked by pre-existing duplicate Requirement names in `agent-skill-injection` and `skill-management`.

## 5. Follow-up

- [x] 5.1 Add `bundle.externalBin` and a tested per-target wrapper build-and-rename script used by every Tauri dev/build npm entry point
- [x] 5.2 Resolve the wrapper beside the main executable before the resource-directory compatibility fallback
- [x] 5.3 Add configuration and resolution regression tests without committing generated platform binaries
- [ ] 5.4 Validate the preparation and package path on Windows x64, macOS arm64/x64, and Linux x64 before `0.1.0`
  - Windows x64 locally verified on 2026-08-09: the NSIS installer was produced and the release directory contained `vanehub-permission-hook.exe` beside `vanehub-ai.exe`.
