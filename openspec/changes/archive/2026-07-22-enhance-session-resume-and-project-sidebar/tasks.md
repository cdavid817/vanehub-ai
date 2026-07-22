## 1. Resume Id Persistence

- [x] 1.1 Trace current Agent Terminal start, runtime event, and session persistence paths for `runtimeSessionId`.
- [x] 1.2 Persist newly observed runtime session ids from terminal start results or runtime events onto the owning session.
- [x] 1.3 Ensure terminal reopen uses the persisted runtime session id for provider-specific resume when no retained process exists.
- [x] 1.4 Add or update Rust tests for runtime session id persistence and resume invocation behavior.
- [x] 1.5 Update Web/mock session and terminal behavior to preserve deterministic mock runtime session ids.

## 2. Project Grouping

- [x] 2.1 Add sidebar model utilities for deriving project group keys and labels from worktree, project, folder, or remote workspace metadata.
- [x] 2.2 Add project grouping presentation to the session sidebar while preserving search, agent filter, pinned, archived, category, and batch-management behavior.
- [x] 2.3 Persist project group expansion state as a local UI preference.
- [x] 2.4 Add focused frontend tests for project grouping, ungrouped sessions, and stable in-group ordering.

## 3. Resizable Sidebar

- [x] 3.1 Add bounded session sidebar width state with local UI preference persistence.
- [x] 3.2 Add an accessible horizontal resize handle that is active only while the sidebar is expanded.
- [x] 3.3 Update workspace grid sizing so resizing, collapse, and information-panel layout remain coherent.
- [x] 3.4 Add focused frontend tests for width clamping, persistence, and collapse/expand restoration.

## 4. Verification

- [x] 4.1 Run `npm run test`.
- [x] 4.2 Run `npm run build`.
- [x] 4.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 4.4 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 4.5 Run `openspec validate --specs --strict`.
- [x] 4.6 Run `openspec validate enhance-session-resume-and-project-sidebar --strict`.

## 5. Verification Follow-up

- [x] 5.1 Add Playwright coverage for sidebar resize persistence, collapse accessibility, and project-group expansion persistence.
- [x] 5.2 Add Rust coverage for runtime-session-id event persistence, retained-terminal refresh, and event publication.
- [x] 5.3 Re-run focused tests and the required frontend, Rust, and OpenSpec validation commands.
