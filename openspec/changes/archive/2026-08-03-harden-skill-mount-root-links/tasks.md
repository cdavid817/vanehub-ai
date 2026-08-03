## 1. Native Mount-Root Safety

- [x] 1.1 Add cross-platform non-following mount-root component inspection that accepts normal directories and classifies live or broken symlink/junction/reparse-point roots without mutating them.
- [x] 1.2 Run mount-root preflight inside CLI binding and repair transactions before directory creation or target staging, returning actionable stable-Agent errors while preserving bindings on failure.
- [x] 1.3 Add filesystem and application tests for normal, absent, live-linked, broken-linked, and atomic rejection behavior, including Windows reparse-point detection.

## 2. Diagnostics and Settings Feedback

- [x] 2.1 Add safe `agentId` context to unified CLI Skill bind/unbind logs without logging absolute mount or external target paths.
- [x] 2.2 Verify Web/mock granular binding behavior remains deterministic and add Settings interaction coverage for row-level failure presentation and unchanged Available state.

## 3. Verification

- [x] 3.1 Run `npm run lint`, `npm run test`, `npm run build`, and targeted Skill Playwright tests.
- [x] 3.2 Run `cargo test`, `cargo check`, and `cargo clippy` for `src-tauri/Cargo.toml`.
- [x] 3.3 Run `openspec validate harden-skill-mount-root-links --strict` and `openspec validate --specs --strict`, and record the verification result.

## Verification Result

- Frontend: lint, 433 Vitest cases, production build, and 3 Skill Playwright cases passed.
- Native: 1038 Rust unit tests and 11 architecture tests passed; 3 fixture helpers remained intentionally ignored; `cargo check` and all-target clippy with warnings denied passed.
- OpenSpec: the change passed strict validation and all 84 main specs passed strict validation.
