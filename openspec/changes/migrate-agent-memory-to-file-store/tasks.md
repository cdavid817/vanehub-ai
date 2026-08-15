## 1. Domain foundation

- [ ] 1.1 Add `domain/memory_document.rs` with the four-value `MemoryType` and a tolerant `parse` that returns `None` for absent or unrecognized values
- [ ] 1.2 Add memory metadata validation: `name` required and filename-safe, `description` required and single-line, provenance fields optional
- [ ] 1.3 Add a frontmatter parser for memory files following the shape of `skills/infrastructure/filesystem/document.rs::parse`, ignoring unknown keys and returning a typed error for a missing frontmatter block
- [ ] 1.4 Add unit tests covering absent type, unrecognized type, unknown extra keys, CRLF input, and a file with no frontmatter

## 2. Directory store

- [ ] 2.1 Resolve the memory directory from the app data directory, honoring the existing `VANEHUB_APP_DATA_DIR` override, and create it if absent
- [ ] 2.2 Add `infrastructure/memory_directory.rs` with scan, read-one, write-one, and delete-one operations; scan reads only the frontmatter region and skips unparseable files
- [ ] 2.3 Implement `MEMORY.md` index assembly and rewriting, derived from the directory scan so the directory stays authoritative
- [ ] 2.4 Add path canonicalization plus a memory-root prefix check used by every write and delete entry point
- [ ] 2.5 Add tests for scan skipping malformed files, index reconciliation when a line points at a missing file, index reconciliation when a file has no line, and `..` and symlink traversal rejection

## 3. Migration from the row store

- [ ] 3.1 Implement row-to-file conversion: body verbatim, slugged `name` with collision suffix, `description` from the leading sentence, `type` omitted, provenance and `migrated_from` preserved
- [ ] 3.2 Wire migration into startup so it runs once when the directory is uninitialized, off the UI thread, with per-row failure isolation
- [ ] 3.3 Add tests for idempotence via `migrated_from`, for a second run not overwriting an edited file, and for one unconvertible row not aborting the batch

## 4. Write paths

- [ ] 4.1 Extend the `remember` tool's argument schema to name, description, type, and content; keep its catalog name and position unchanged
- [ ] 4.2 Reimplement `remember` to write the memory file and its index line as one operation, replacing an existing file when the name matches
- [ ] 4.3 Confirm the catalog-length assertions and provider tool-declaration tests still pass with the name and position unchanged
- [ ] 4.4 Add the memory directory as an auto-approved read and write scope for the generic file tools in the tool permission mapping
- [ ] 4.5 Add a permission test asserting that a write outside the memory directory keeps its previous approval behavior, and that a memory-directory write is rejected while the memory enablement toggle is off

## 5. Extraction returns actions

- [ ] 5.1 Define the extraction action schema (`create` / `update` / `delete` with name, description, type, body) and its validator
- [ ] 5.2 Change `memory_extraction_gateway.rs` to request structured output and return a validated action list instead of `Option<String>`
- [ ] 5.3 Include the existing-memory manifest and the bodies of the most relevant existing memories in the extraction prompt so update actions are expressible
- [ ] 5.4 Apply surviving actions to the directory; reject per action, not per call, and log each rejection without failing the generation
- [ ] 5.5 Repoint OnePiece's compaction-triggered extraction and the CLI post-turn extraction at the directory sink, leaving both trigger conditions unchanged
- [ ] 5.6 Add tests for an action naming a path outside the memory root, a partially invalid action list, an empty action list, and an unparseable response

## 6. Read paths

- [ ] 6.1 Change `format_memory_section` to enumerate the directory, ordered by last modification time, keeping the existing character budget and section shape
- [ ] 6.2 Repoint `list_agent_memories`, `delete_agent_memory`, and `reset_agent_memories` at the directory, keeping command names unchanged
- [ ] 6.3 Extend the memory record payload with name, description, and type
- [ ] 6.4 Add tests for a memory updated in-session sorting ahead of untouched older memories, and for the listing reflecting an out-of-band edit without a restart

## 7. Retrieval index

- [ ] 7.1 Change the `agent_memory` document identity from a row id to a directory-relative path
- [ ] 7.2 Switch `agent_memory` reconciliation to take the directory scan as its authoritative snapshot
- [ ] 7.3 Resolve search hits by reading the memory file, omitting a hit whose file is gone rather than returning indexed text
- [ ] 7.4 Add tests for out-of-band file deletion revoking recall, and for `workspace_file` documents being untouched by memory reconciliation

## 8. Frontend service boundary

- [ ] 8.1 Extend the memory record type in `src/services/agent-service.ts` with name, description, and type
- [ ] 8.2 Update `tauri-agent-client.ts` for the richer payload without introducing `invoke()` outside the adapter
- [ ] 8.3 Update `web-agent-client.ts` to back the same shape with an in-process list, keeping mock memory events and toggle gating intact
- [ ] 8.4 Update the memory management UI to surface name, description, and type
- [ ] 8.5 Add component tests for the management view rendering the new fields and for an untyped memory rendering without error

## 9. Verification

- [ ] 9.1 `npm run lint:ci`
- [ ] 9.2 `npm run test`
- [ ] 9.3 `npm run build`
- [ ] 9.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 9.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 9.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 9.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] 9.8 `openspec validate migrate-agent-memory-to-file-store --strict` and `openspec validate --specs --strict`
- [ ] 9.9 `npm run test:coverage` and `npm run contracts:check`, since the service contract changed
- [ ] 9.10 `npx playwright test` for the memory management UI change
- [ ] 9.11 Record implementation verification results for the archive gate
