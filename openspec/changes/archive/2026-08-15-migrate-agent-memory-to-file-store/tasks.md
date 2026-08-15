## 1. Domain foundation

- [x] 1.1 Add `domain/memory_document.rs` with the four-value `MemoryType` and a tolerant `parse` that returns `None` for absent or unrecognized values
- [x] 1.2 Add memory metadata validation: `name` required and filename-safe, `description` required and single-line, provenance fields optional
- [x] 1.3 Add a frontmatter parser for memory files following the shape of `skills/infrastructure/filesystem/document.rs::parse`, ignoring unknown keys and returning a typed error for a missing frontmatter block
- [x] 1.4 Add unit tests covering absent type, unrecognized type, unknown extra keys, CRLF input, and a file with no frontmatter

## 2. Directory store

- [x] 2.1 Resolve the memory directory from the app data directory, honoring the existing `VANEHUB_APP_DATA_DIR` override, and create it if absent
- [x] 2.2 Add `infrastructure/memory_directory.rs` with scan, read-one, write-one, and delete-one operations; scan reads only the frontmatter region and skips unparseable files
- [x] 2.3 Implement `MEMORY.md` index assembly and rewriting, derived from the directory scan so the directory stays authoritative
- [x] 2.4 Add path canonicalization plus a memory-root prefix check used by every write and delete entry point
- [x] 2.5 Add tests for scan skipping malformed files, index reconciliation when a line points at a missing file, index reconciliation when a file has no line, and `..` and symlink traversal rejection

## 3. Migration from the row store

- [x] 3.1 Implement row-to-file conversion: body verbatim, slugged `name` with collision suffix, `description` from the leading sentence, `type` omitted, provenance and `migrated_from` preserved
- [x] 3.2 Wire migration into startup so it runs once when the directory is uninitialized, off the UI thread, with per-row failure isolation
- [x] 3.3 Add tests for idempotence via `migrated_from`, for a second run not overwriting an edited file, and for one unconvertible row not aborting the batch

## 4. Write paths

- [x] 4.1 Extend the `remember` tool's argument schema to name, description, type, and content; keep its catalog name and position unchanged
- [x] 4.2 Reimplement `remember` to write the memory file and its index line as one operation, replacing an existing file when the name matches
- [x] 4.3 Confirm the catalog-length assertions and provider tool-declaration tests still pass with the name and position unchanged
- [x] 4.4 Add the memory directory as an auto-approved read and write scope for the generic file tools in the tool permission mapping
- [x] 4.5 Add a permission test asserting that a write outside the memory directory keeps its previous approval behavior, and that a memory-directory write is rejected while the memory enablement toggle is off

## 5. Extraction returns actions

- [x] 5.1 Define the extraction action schema (`create` / `update` / `delete` with name, description, type, body) and its validator
- [x] 5.2 Change `memory_extraction_gateway.rs` to request structured output and return a validated action list instead of `Option<String>`
- [x] 5.3 Include the existing-memory manifest and the bodies of the most relevant existing memories in the extraction prompt so update actions are expressible
- [x] 5.4 Apply surviving actions to the directory; reject per action, not per call, and log each rejection without failing the generation
- [x] 5.5 Repoint OnePiece's compaction-triggered extraction and the CLI post-turn extraction at the directory sink, leaving both trigger conditions unchanged
- [x] 5.6 Add tests for an action naming a path outside the memory root, a partially invalid action list, an empty action list, and an unparseable response

## 6. Read paths

- [x] 6.1 Change `format_memory_section` to enumerate the directory, ordered by last modification time, keeping the existing character budget and section shape
- [x] 6.2 Repoint `list_agent_memories`, `delete_agent_memory`, and `reset_agent_memories` at the directory, keeping command names unchanged
- [x] 6.3 Extend the memory record payload with name, description, and type
- [x] 6.4 Add tests for a memory updated in-session sorting ahead of untouched older memories, and for the listing reflecting an out-of-band edit without a restart

## 7. Retrieval index

- [x] 7.1 Change the `agent_memory` document identity from a row id to a directory-relative path
- [x] 7.2 Switch `agent_memory` reconciliation to take the directory scan as its authoritative snapshot
- [x] 7.3 Resolve search hits by reading the memory file, omitting a hit whose file is gone rather than returning indexed text
- [x] 7.4 Add tests for out-of-band file deletion revoking recall, and for `workspace_file` documents being untouched by memory reconciliation

## 8. Frontend service boundary

- [x] 8.1 Extend the memory record type in `src/services/agent-service.ts` with name, description, and type
- [x] 8.2 Update `tauri-agent-client.ts` for the richer payload without introducing `invoke()` outside the adapter
- [x] 8.3 Update `web-agent-client.ts` to back the same shape with an in-process list, keeping mock memory events and toggle gating intact
- [x] 8.4 Update the memory management UI to surface name, description, and type
- [x] 8.5 Add component tests for the management view rendering the new fields and for an untyped memory rendering without error

## 9. Verification

- [x] 9.1 `npm run lint:ci`
- [x] 9.2 `npm run test`
- [x] 9.3 `npm run build`
- [x] 9.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 9.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 9.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 9.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 9.8 `openspec validate migrate-agent-memory-to-file-store --strict` and `openspec validate --specs --strict`
- [x] 9.9 `npm run test:coverage` and `npm run contracts:check`, since the service contract changed
- [x] 9.10 `npx playwright test` for the memory management UI change
- [x] 9.11 Record implementation verification results for the archive gate

## Verification results

Run on Windows 11, branch `worktree-context`. Every `cargo test` invocation carried
`no_proxy`/`NO_PROXY` for loopback, without which the local-HTTP fixtures hang against this
machine's system proxy rather than failing.

| Command | Result |
|---|---|
| `npm run lint:ci` | clean |
| `npm run test` | 261 files, 1193 tests passed |
| `npm run build` | built, 16 lazy chunks, 127.0 KiB gzip static closure |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 2859 lib + 47 integration passed, 0 failed, 15 ignored |
| `cargo check` | clean |
| `openspec validate migrate-agent-memory-to-file-store --strict` | valid |
| `openspec validate --specs --strict` | 119 passed, 0 failed |
| `npm run test:coverage` | 1193 passed; statements 71.1%, branches 67.2%, functions 67.02%, lines 75.15% |
| `npm run contracts:check` | 3 passed |
| `npm run coverage:policy:test` | 0 failed |
| `npm run version:unit:test` | 0 failed |
| `npm run docs:check` | verified |
| `npx playwright test` | 113 passed (8.9m) |

The full Playwright suite was run rather than only the personalization spec: this change adds
memory type labels whose text ("User", "Feedback", "Project", "Reference") is generic enough to
collide with unrelated specs through substring matching. No collision occurred.

Desktop smoke (`npm run test:desktop`) was not run. This change alters no Tauri startup, IPC, or
desktop runtime behavior, and CI runs that job on all three platforms.
