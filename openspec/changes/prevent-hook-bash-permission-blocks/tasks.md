## 1. Managed Claude Code Launch Scope

- [x] 1.1 Add the managed permission-hook scope to Claude Code chat and interactive launch environments.
- [x] 1.2 Verify Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI launch environments remain free of the Claude-specific scope.

## 2. Permission Hook Pass-Through

- [x] 2.1 Add an explicit pass-through outcome for valid unscoped hook requests before discovery or loopback access.
- [x] 2.2 Emit no stdout decision for pass-through while preserving existing managed allow, deny, malformed-input, timeout, and offline behavior.

## 3. Regression Coverage

- [x] 3.1 Add sidecar tests for unscoped Bash pass-through and managed offline/policy decisions.
- [x] 3.2 Add native Agent Runtime tests covering the Claude-only marker across chat and interactive policy projection.

## 4. Verification

- [x] 4.1 Run `openspec validate prevent-hook-bash-permission-blocks --strict` and `openspec validate --specs --strict`.
- [x] 4.2 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 4.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 4.4 Run `npm run desktop:unit:test` for the desktop launch and permission-hook behavior change.
- [ ] 4.5 Obtain a passing `npm run test:desktop` result; the local Windows run is currently blocked by a host-wide WebDriver DirectEval `fetch failed` condition and requires CI runner verification.
