## 1. Index injection

- [ ] 1.1 Split `format_memory_section` into index assembly and body assembly, with the index built from `MEMORY.md` reconciled against the directory scan
- [ ] 1.2 Implement the paired line-and-byte cap: truncate at an entry boundary, never mid-entry, and append a line naming which cap fired
- [ ] 1.3 Order index entries by last modification time so truncation drops the least recently modified first
- [ ] 1.4 Apply the OnePiece caps (200 lines, 12,000 bytes) to the system-prompt index
- [ ] 1.5 Apply the separate CLI caps (40 lines, 3,000 bytes) at the Prompt Hook injection point, leaving its position and ordering unchanged
- [ ] 1.6 Add tests for line-cap truncation, byte-cap truncation within the line cap, entry-boundary cutting, the truncation notice text, and the two surfaces truncating independently

## 2. Relevance selection

- [ ] 2.1 Add the selector gateway, resolving OnePiece's credentials and provider the way `memory_extraction_gateway.rs` does
- [ ] 2.2 Build the selection manifest from names, types, descriptions, and ages only, never bodies
- [ ] 2.3 Request structured output bounded to a small token cap, and discard returned names absent from the manifest
- [ ] 2.4 Instruct the selector to return an empty list when nothing is clearly useful rather than its best guess
- [ ] 2.5 Enforce the selection bound of five memories
- [ ] 2.6 Add tests for an empty selection, an over-bound selection, a hallucinated name, and an unparseable response

## 3. Generation-scoped assembly

- [ ] 3.1 Run selection once at generation start, not per provider round-trip
- [ ] 3.2 Place the selected bodies in the system prompt after the index, with Skills and the index ahead of them, so the volatile section sits at the tail of the cached prefix
- [ ] 3.3 Assert the system prompt is byte-identical across every round-trip of one generation, including after compaction triggers
- [ ] 3.4 Degrade to index-only injection on any selector error, timeout, or unusable result, without failing the generation
- [ ] 3.5 Skip the selector call entirely when the memory enablement toggle is off

## 4. Already-surfaced exclusion

- [ ] 4.1 Add session-scoped state holding `(path, mtime-at-surface)` for every memory whose body has been injected
- [ ] 4.2 Filter surfaced memories out of the candidate manifest before the selector call
- [ ] 4.3 Make a memory eligible again once its mtime changes
- [ ] 4.4 Clear the state when a session ends, and do not persist it
- [ ] 4.5 Add tests for exclusion across two generations in one session, re-eligibility after an update, and a fresh session seeing everything

## 5. Freshness annotation

- [ ] 5.1 Compute elapsed age from mtime and render it in words rather than as a timestamp
- [ ] 5.2 Attach the verify-before-asserting caveat only to memories older than one day
- [ ] 5.3 Add tests for a fresh memory carrying age without a caveat, and a stale memory carrying both

## 6. Runtime parity

- [ ] 6.1 Update `web-agent-client.ts` to simulate index injection and body selection through the existing memory events, with no provider call
- [ ] 6.2 Gate the simulated selection event on the memory enablement toggle, matching the desktop runtime
- [ ] 6.3 Confirm index injection and selection still operate with no embedding source configured, and that `recall` remains absent from the catalog in that state
- [ ] 6.4 Add tests for Web-runtime selection parity and for the no-embedding-configured path

## 7. Verification

- [ ] 7.1 `npm run lint:ci`
- [ ] 7.2 `npm run test`
- [ ] 7.3 `npm run build`
- [ ] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 7.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 7.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 7.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] 7.8 `openspec validate add-two-tier-memory-recall --strict` and `openspec validate --specs --strict`
- [ ] 7.9 `npm run test:coverage`
- [ ] 7.10 Record implementation verification results for the archive gate
