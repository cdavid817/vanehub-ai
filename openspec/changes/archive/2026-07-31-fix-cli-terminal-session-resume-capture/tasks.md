## 1. Provider invocation identity

- [x] 1.1 Extend interactive invocation specs to assign provider-valid ids for fresh Claude Code and Gemini CLI launches while preserving exact-id resume arguments for all stable Agents.
- [x] 1.2 Add provider invocation tests covering fresh assignment, exact resume, empty-id handling, and stable Agent coverage.

## 2. Provider-allocated session capture

- [x] 2.1 Add a Codex rollout baseline/discovery reader that validates a unique new `session_meta` id against the terminal working directory.
- [x] 2.2 Add an OpenCode SQLite baseline/discovery reader that validates a unique new session id against creation metadata and the normalized terminal working directory.
- [x] 2.3 Add deterministic reader tests for unique, missing, malformed, wrong-directory, and ambiguous candidates.

## 3. Agent Terminal integration

- [x] 3.1 Integrate pre-launch capture preparation and post-spawn assigned-id persistence into the native Agent Terminal process path.
- [x] 3.2 Retry provider-allocated discovery at a bounded cadence during terminal activity, publish the existing runtime-session-id event on success, and stop after an id is recorded.
- [x] 3.3 Record capture ambiguity and provider-store failures through the unified redacted Agent terminal logging port without adding feature-local logs.
- [x] 3.4 Add terminal/application tests proving assigned and discovered ids are persisted, retained, returned, and used by exact-id reopen.

## 4. Compatibility and verification

- [x] 4.1 Confirm the frontend Agent service, Tauri adapter, and Web/mock adapter require no contract changes and keep deterministic Web runtime ids covered by tests.
- [x] 4.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 4.3 Run `cargo test`, `cargo check`, and `cargo clippy` for `src-tauri/Cargo.toml`.
- [x] 4.4 Run `openspec validate fix-cli-terminal-session-resume-capture --strict` and `openspec validate --specs --strict`.
