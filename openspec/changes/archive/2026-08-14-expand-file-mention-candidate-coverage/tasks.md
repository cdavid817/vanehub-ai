## 1. Native search core

- [x] 1.1 Add the mention-candidate extension allowlist and the vendored/generated directory exclusion list as named constants in the workspaces context, alongside the existing traversal constants
- [x] 1.2 Implement the bounded candidate walk: depth-limited, skips dot-prefixed entries and excluded directory names, canonicalizes and enforces session-root containment, tracks visited directories against symlink loops
- [x] 1.3 Implement match scoring — exact filename 100, filename prefix 80, filename substring 60, ordered path-segment match 40, no match excluded — with ties broken by shallower depth then case-insensitive path order
- [x] 1.4 Match a query containing a path separator against the relative path instead of the filename alone
- [x] 1.5 Cap the returned result count at the caller-supplied limit and stop traversal early once the budget is satisfiable
- [x] 1.6 Return an empty result set with a structured error for a session with no resolvable root, without leaking raw native diagnostics

## 2. Tauri command surface

- [x] 2.1 Add the search command module under `src-tauri/src/commands/workspaces/` with its DTO and mapper, taking session id, query, and result cap
- [x] 2.2 Register the command in the core command registry
- [x] 2.3 Confirm `list_session_documents` and `collect_documents` are byte-for-byte unchanged, so the Documents tab requirement in `session-workspace-tabs` needs no re-verification

## 3. Frontend service boundary

- [x] 3.1 Add the search method to the `agent-service.ts` interface with its result type
- [x] 3.2 Implement it in `tauri-agent-client.ts` by invoking the new command
- [x] 3.3 Implement it in `web-agent-client.ts` over the mock workspace, applying the same ranking and caps so browser mode stays usable
- [x] 3.4 Register the command in `src/contracts/` so `npm run contracts:check` covers it

## 4. Composer wiring

- [x] 4.1 Extract mention parsing, candidate querying, debounce, out-of-order response rejection, and selection into a composer mention hook, moving participant-mention completion with it
- [x] 4.2 Reduce `ChatInputBox.tsx` to consuming that hook; confirm it is comfortably under the 300-line cap with headroom for the follow-up changes
- [x] 4.3 Stop deriving `fileReferenceCandidates` from `documentsQuery` in `use-main-layout-model.ts`; issue a query-scoped request through the service boundary instead
- [x] 4.4 Verify the Documents tab still consumes `listSessionDocuments` and is unaffected

## 5. Tests

- [x] 5.1 Rust: allowlist admits representative extensions per language family and rejects binary/generated ones
- [x] 5.2 Rust: excluded directory trees are not descended into, and their contents never appear in results
- [x] 5.3 Rust: ranking order across all four tiers, deterministic tie-breaking, and path-separator queries
- [x] 5.4 Rust: result cap, depth limit, root containment, symlink loop safety, and unresolvable-root handling
- [x] 5.5 Rust: a regression test asserting `list_session_documents` still returns only Markdown and text documents
- [x] 5.6 Frontend: migrate composer completion tests off the documents-listing stub onto the new service method — leaving them on the old stub would keep them green against a source the composer no longer uses
- [x] 5.7 Frontend: hook-level tests for debounce and out-of-order response rejection
- [x] 5.8 Frontend: web-adapter test asserting the mock implementation returns the same ordering contract as the native one

## 6. Verification

- [x] 6.1 `npm run lint:ci`
- [x] 6.2 `npm run test`
- [x] 6.3 `npm run build`
- [x] 6.4 `npm run contracts:check`
- [x] 6.5 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.6 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.7 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 6.8 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.9 `npx playwright test` — composer completion is a UI behavior change
- [x] 6.10 `openspec validate expand-file-mention-candidate-coverage --strict`
- [x] 6.11 `openspec validate --specs --strict`
- [x] 6.12 Manual check against a real project root: `@` resolves a source file that is not Markdown or text, and no dependency-directory file appears in results
