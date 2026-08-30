## 1. Focused Harness

- [x] 1.1 Add an independently runnable desktop Agent-evaluation layer and register its fixture and live modes in the orchestrator, package scripts, and spec manifest.
- [x] 1.2 Add prerequisite detection that distinguishes installed OpenCode, authenticated OpenCode, and available OnePiece credentials without exposing secret values.
- [x] 1.3 Add bounded evidence metadata and secret-audit coverage for `PASSED`, `FAILED`, `BLOCKED`, and `NOT RUN` outcomes.

## 2. WebdriverIO Coverage

- [x] 2.1 Add a focused OpenCode fixture evaluation spec that selects stable id `opencode` and verifies arena lifecycle, persistence, diagnostics, and rendered detail.
- [x] 2.2 Run the focused OpenCode fixture evaluation and fix every reproducible Agent-evaluation defect it exposes.
- [x] 2.3 Run live OpenCode qualification when authenticated; otherwise record a truthful `BLOCKED` result without substituting fixture evidence.
- [x] 2.4 Add and run the credential-gated OnePiece evaluation path, verifying stable id `onepiece`, terminal results, UI projection, and safe evidence.

## 3. Regression and Validation

- [x] 3.1 Add unit tests for config selection, prerequisite classification, and evidence redaction.
- [x] 3.2 Run `npm run test:desktop:agent-evaluation`, `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 3.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`.
- [x] 3.4 Run `openspec validate fix-agent-evaluation-webdriverio --strict` and `openspec validate --specs --strict`, then record the verification outcome.

Verification evidence: focused fixture OpenCode `PASSED` with terminal outcome `task_failed`, 0 dispatch diagnostics, and evidence-safety `PASSED`; live OpenCode qualification `BLOCKED` because the installed CLI has no credentials; live OnePiece `PASSED` with stable id `onepiece`, real provider generation completion, terminal outcome `task_failed`, 0 dispatch diagnostics, rendered result projection, and a 4-file/11,081-byte evidence scan with 0 secret findings; lint, frontend tests, build, Rust formatting, workspace check, panic check, and all workspace tests passed; strict Clippy was run but remains blocked by three unrelated warnings in concurrent Skill Evolution test files; both strict OpenSpec validations passed on 2026-08-28.
