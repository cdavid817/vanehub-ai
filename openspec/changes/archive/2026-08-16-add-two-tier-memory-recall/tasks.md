## 1. Index injection

- [x] 1.1 Split `format_memory_section` into index assembly and body assembly, with the index built from `MEMORY.md` reconciled against the directory scan
- [x] 1.2 Implement the paired line-and-byte cap: truncate at an entry boundary, never mid-entry, and append a line naming which cap fired
- [x] 1.3 Order index entries by last modification time so truncation drops the least recently modified first
- [x] 1.4 Apply the OnePiece caps (200 lines, 12,000 bytes) to the system-prompt index
- [x] 1.5 Apply the separate CLI caps (40 lines, 3,000 bytes) at the Prompt Hook injection point, leaving its position and ordering unchanged
- [x] 1.6 Add tests for line-cap truncation, byte-cap truncation within the line cap, entry-boundary cutting, the truncation notice text, and the two surfaces truncating independently

## 2. Relevance selection

- [x] 2.1 Add the selector gateway, resolving OnePiece's credentials and provider the way `memory_extraction_gateway.rs` does
- [x] 2.2 Build the selection manifest from names, types, descriptions, and ages only, never bodies
- [x] 2.3 Request structured output bounded to a small token cap, and discard returned names absent from the manifest
- [x] 2.4 Instruct the selector to return an empty list when nothing is clearly useful rather than its best guess
- [x] 2.5 Enforce the selection bound of five memories
- [x] 2.6 Add tests for an empty selection, an over-bound selection, a hallucinated name, and an unparseable response

## 3. Generation-scoped assembly

- [x] 3.1 Run selection once at generation start, not per provider round-trip
- [x] 3.2 Place the selected bodies in the system prompt after the index, with Skills and the index ahead of them, so the volatile section sits at the tail of the cached prefix
- [x] 3.3 Assert the system prompt is byte-identical across every round-trip of one generation, including after compaction triggers
- [x] 3.4 Degrade to index-only injection on any selector error, timeout, or unusable result, without failing the generation
- [x] 3.5 Skip the selector call entirely when the memory enablement toggle is off

## 4. Already-surfaced exclusion

- [x] 4.1 Add session-scoped state holding `(path, mtime-at-surface)` for every memory whose body has been injected
- [x] 4.2 Filter surfaced memories out of the candidate manifest before the selector call
- [x] 4.3 Make a memory eligible again once its mtime changes
- [x] 4.4 Clear the state when a session ends, and do not persist it
  - Not persisted, and reclaimed by a session cap rather than an explicit end-of-session hook: session lifecycle lives in the `sessions` context, and signalling into `agent_runtime` from there would cross a boundary the architecture keeps closed. A finished session is never consulted again, so the cap is what retires it.
- [x] 4.5 Add tests for exclusion across two generations in one session, re-eligibility after an update, and a fresh session seeing everything

## 5. Freshness annotation

- [x] 5.1 Compute elapsed age from mtime and render it in words rather than as a timestamp
- [x] 5.2 Attach the verify-before-asserting caveat only to memories older than one day
- [x] 5.3 Add tests for a fresh memory carrying age without a caveat, and a stale memory carrying both

## 6. Runtime parity

- [x] 6.1 Update `web-agent-client.ts` to simulate index injection and body selection through the existing memory events, with no provider call
- [x] 6.2 Gate the simulated selection event on the memory enablement toggle, matching the desktop runtime
- [x] 6.3 Confirm index injection and selection still operate with no embedding source configured, and that `recall` remains absent from the catalog in that state
- [x] 6.4 Add tests for Web-runtime selection parity and for the no-embedding-configured path

## 7. Verification

- [x] 7.1 `npm run lint:ci`
- [x] 7.2 `npm run test`
- [x] 7.3 `npm run build`
- [x] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 7.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 7.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 7.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 7.8 `openspec validate add-two-tier-memory-recall --strict` and `openspec validate --specs --strict`
- [x] 7.9 `npm run test:coverage`
- [x] 7.10 Record implementation verification results for the archive gate

## Verification results

Run on Windows 11, branch `worktree-context`. Every `cargo test` invocation carried
`no_proxy`/`NO_PROXY` for loopback; without it the local-HTTP fixtures hang against this machine's
system proxy rather than failing.

| Command | Result |
|---|---|
| `npm run lint:ci` | clean |
| `npm run test` | 261 files, 1195 tests passed |
| `npm run build` | built, 16 lazy chunks, 127.0 KiB gzip static closure |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 2882 lib + 47 integration passed, 0 failed, 15 ignored |
| `cargo check` | clean |
| `openspec validate add-two-tier-memory-recall --strict` | valid |
| `openspec validate --specs --strict` | 119 passed, 0 failed |
| `npm run test:coverage` | 1195 passed; statements 71.09%, branches 67.17%, functions 67.02%, lines 75.13% |

Two notes on what these runs cost to get right.

`npm run build` caught a type error `npx vitest run` did not: a test read `bodyMarkdown` off a
`RichBlock` union whose audio variant has no such field. Vitest does not typecheck, so the frontend
test suite passing is not evidence the build will.

One earlier `npm run test` reported a single failure that did not reproduce across two subsequent
full runs, with a concurrent cargo build competing for the machine at the time. It is recorded as
an unreproduced flake rather than as a clean first pass; the suite above is a later, quiet run.

Playwright was not run: this change adds no UI. The memory management surface it feeds was covered
by `migrate-agent-memory-to-file-store`'s own e2e run. Desktop smoke was not run either — no Tauri
startup, IPC, or desktop runtime behavior changes here, and CI runs that job on all three
platforms.
