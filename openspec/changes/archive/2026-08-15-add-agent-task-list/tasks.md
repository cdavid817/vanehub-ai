## 1. Task list state

- [x] 1.1 Add a session-scoped task-list store in the agent runtime's tool infrastructure with whole-list replacement and per-session isolation.
- [x] 1.2 Validate submitted lists: item count bound, item text bound, non-empty text, recognized status, and at most one in-progress item, leaving the previous list unchanged on rejection.
- [x] 1.3 Render the stored list as a bounded system-prompt section, omitted entirely when the list is empty.
- [x] 1.4 Discard a session's list when the session ends, alongside the existing background-command reaping.

## 2. Tool surface

- [x] 2.1 Add the `todo_write` tool definition to the baseline and plan-mode catalogs with a closed schema that accepts no scope argument.
- [x] 2.2 Route `todo_write` through the tool-call executor with session context and no workspace-folder requirement.
- [x] 2.3 Classify `todo_write` as a no-approval operation in the permission mapping.
- [x] 2.4 Inject the task-list section into the generation system prompt after the memory section.

## 3. Tests

- [x] 3.1 Store unit tests for replacement, ordering, clearing, per-session isolation, and session discard.
- [x] 3.2 Validation tests for every rejection case, each asserting the previous list is preserved.
- [x] 3.3 Projection tests for section presence, omission when empty, and reflecting the most recent write.
- [x] 3.4 Catalog and permission tests covering both catalogs and the no-approval classification.
- [x] 3.5 Executor routing tests, including a folder-less session and a rejected write.

## 4. Validation

- [x] 4.1 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 4.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 4.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.4 `openspec validate add-agent-task-list --strict`
