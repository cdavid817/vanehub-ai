## 1. Contracts and Data Model

- [x] 1.1 Add TypeScript contracts for session search results, categories, export requests/results, automatic archival settings, and chat file references
- [x] 1.2 Extend `AgentService` with additive methods for session search, category CRUD/assignment, session export, archival settings, and file-reference-aware send input
- [x] 1.3 Add SQLite migrations for `session_categories`, nullable `sessions.category_id`, archival settings defaults, and message file-reference metadata
- [x] 1.4 Update Rust session/message models and serialization structs to include category id and safe file-reference metadata

## 2. Native Session Search and Categories

- [x] 2.1 Implement Rust commands for bounded historical session search across title, project/worktree metadata, and message content
- [x] 2.2 Implement Rust category list/create/rename/delete operations with duplicate and empty-name validation
- [x] 2.3 Implement Rust session category assignment and uncategorized behavior when deleting a category
- [x] 2.4 Add unit tests for search result boundaries, archived result inclusion, category assignment, and category deletion preserving sessions

## 3. Export

- [x] 3.1 Implement Rust export serialization for JSON with versioned metadata, messages, thinking, tool-use, token usage, statuses, errors, and file references
- [x] 3.2 Implement Rust export serialization for Markdown with readable chronological transcript output
- [x] 3.3 Add Tauri command and adapter flow for selecting or receiving a destination directory and writing export files through Rust
- [x] 3.4 Add desktop and Web/mock tests for successful export, unsupported destination behavior, and export failure feedback

## 4. Automatic Archival and Recovery

- [x] 4.1 Implement automatic archival settings read/write with defaults of enabled and 10 inactivity days
- [x] 4.2 Implement Rust startup archival pass and hourly background scheduler that skips pinned, archived, `starting`, and `running` sessions
- [x] 4.3 Implement Rust startup recovery for orphan `starting`/`running` sessions and unfinished assistant messages, marking them failed while preserving partial content and runtime session ids
- [x] 4.4 Write unified log entries for archival and recovery mutations with redacted diagnostics
- [x] 4.5 Add Rust tests for default settings, disabled archival, threshold eligibility, skip rules, recovery transitions, and resume metadata preservation

## 5. Frontend Adapters and Web Runtime

- [x] 5.1 Implement new Tauri adapter methods in `tauri-agent-client.ts` without moving `invoke()` into React components
- [x] 5.2 Implement matching deterministic Web/mock behavior for search, categories, export simulation, archival settings, Mermaid-ready messages, and file references
- [x] 5.3 Update frontend service tests proving Tauri/Web adapter parity and no direct Tauri usage outside adapters

## 6. Sidebar Session Management UI

- [x] 6.1 Add localized sidebar search input and result rendering with archived/category/project metadata
- [x] 6.2 Add category view grouping with collapsible category and uncategorized groups
- [x] 6.3 Extend session context menu with move-to-category, create-category-and-move, remove-category, and export actions
- [x] 6.4 Add drag-and-drop assignment from session cards to category groups while preserving context-menu parity
- [x] 6.5 Add localized feedback for category and export success/failure without blocking unrelated navigation

## 7. Chat Mermaid and File References

- [x] 7.1 Add safe Mermaid fenced-code rendering to chat messages with fallback source display on render failure
- [x] 7.2 Add composer `@` detection, bounded file candidate UI, file-reference chips, and remove-reference behavior
- [x] 7.3 Extend send-message flow to submit selected file references and render persisted reference metadata in message history
- [x] 7.4 Validate file references through the service/native boundary and surface localized errors for unsafe, oversized, binary, or unavailable files
- [x] 7.5 Add frontend tests for Mermaid render/fallback behavior and file-reference candidate, send, persistence, and rejection flows

## 8. Verification

- [x] 8.1 Run `npm run test`
- [x] 8.2 Run `npm run build`
- [x] 8.3 Run `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; $env:CARGO_NET_OFFLINE="true"; cargo check --manifest-path src-tauri\Cargo.toml`
- [x] 8.4 Run `openspec validate "improve-session-management" --strict`
