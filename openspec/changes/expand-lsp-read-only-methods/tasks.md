## 1. Baseline

- [x] 1.1 Record the pre-change pass state of `cargo test --workspace code_intelligence`, `cargo test --workspace native_lsp`, the frontend LSP vitest files, and `npx playwright test tests/e2e/lsp-settings.spec.ts`. Unset `all_proxy`/`ALL_PROXY` and pin `PLAYWRIGHT_PORT` for the last one
  - Baseline on `11458275`: `code_intelligence` **190**, `native_lsp` **1**, frontend LSP suites **746**.
- [x] 1.2 Record the current native tool count and where it is asserted, so the catalog growth is a deliberate diff rather than a surprise
  - **10**, asserted at `providers/tests.rs:768` (`fixtures.len()`) and `:1005` (`tools.len()`), plus the `resolve_tool_catalog_*` family.
- [ ] 1.3 Measure the serialized size of the LSP tool definitions as declared to a provider. Nine tools where there were four lengthens every eligible session's system prompt, and the cost should be known rather than assumed

## 2. Capability representation (refactor, no behavior change)

- [x] 2.1 Write failing tests for the negotiated record: a method the client implements but the server omits reports unsupported rather than being absent, a capability the client does not implement never appears, and two servers negotiating the same set list them in the same order
- [x] 2.2 Add `SemanticMethod::ALL` in a fixed order and a mapping from each variant to its LSP capability path, with a test asserting every variant maps to a non-empty path
- [x] 2.3 Replace the four booleans in `NegotiatedCapabilities` with one entry per `ALL` variant; keep `position_encoding` and `document_sync` as fields, since neither has a supported/unsupported axis
- [x] 2.4 Reduce `supports()` to a lookup over that list, keeping its signature so no caller changes
- [x] 2.5 Rewrite `initialize_negotiation.rs` to build the list by iterating `ALL` rather than reading four capabilities by hand
- [x] 2.6 Update the command DTO, the TypeScript type, and the contract validator to carry the list, with the validator checking method-identifier shape and rejecting duplicates
- [x] 2.7 Replace the status card's hardcoded array with iteration over the reported list, falling back to the raw method identifier when a locale lacks the key
- [x] 2.8 Update the Web/mock adapter to report the same list deterministically
- [x] 2.9 Update the whole-object DTO fixtures to the new shape. They fail here because they are working; do not loosen them
- [x] 2.10 **Acceptance for this group before any method is added:** every suite from task 1.1 passes with the same counts. The refactor is behavior-preserving or it is wrong
  - **Held.** Frontend **746 -> 746**, unchanged. `code_intelligence` **190 -> 192**, and the +2 are the two new domain tests this group added; no existing test changed outcome.
  - `initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` failed twice and then passed three times on the same code, including under the identical full filter. It asserts a 2s initialize timeout and is load-sensitive; it was green on the baseline. Recorded rather than investigated, because on this one an isolated pass proves nothing either — it passes under load too.
  - The compiler guarantee turned out to survive the refactor, contrary to what the design predicted: `advertised()` matches exhaustively on `SemanticMethod`, so a variant added without deciding how it is advertised still fails to compile. What the compiler cannot check is `ALL` completeness, and `all_lists_every_semantic_method` covers that with an exhaustive match that forces a human back to the list.

## 3. Shared query shape

- [x] 3.1 Factor the repeated prepare/request/record/release sequence out of the four existing coordinator methods, keeping their behavior identical
  - Four pieces: `PositionRequest` (the six values every position query needs from its caller), `position_query` (prepare -> position -> one request, returning the prepared query beside the response), `request` (owns record-then-release so a later method cannot leak the slot), and `located_query` (the whole shape of a method answering with locations, parameterised by endpoint and normalization).
  - `wire_request` derives the endpoint and any extra parameters from `SemanticMethod` and matches exhaustively, so a variant added without an endpoint fails to compile. The `Diagnostics` arm exists only to keep that match exhaustive; diagnostics arrive as a notification and never route through a request.
  - `located_outcome` dropped its cap argument: the normalizer has already truncated to the method's own cap, so the returned count is the vector's length. Passing the cap in again was only a chance to pair the wrong one.
  - Behavior unchanged: `cargo test --workspace code_intelligence` **192 passed**, same as the group 2 checkpoint. The first run failed on `initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` (the known load-sensitive test, 191+1 = the same 192) and a re-run was clean.
- [x] 3.2 Confirm `semantic_query_coordinator.rs` is smaller after the factoring than before, not larger. Five more methods of the unfactored shape would roughly double it
  - **The prediction was wrong and the file grew: 562 -> 594 physical lines (+32).** Reported rather than engineered away; hitting the number would have meant deleting the doc comments that state the invariants, or pushing the request construction out into `api.rs`, which moves lines instead of removing them.
  - What the checkpoint was actually protecting is marginal cost, and that did move: a new location method now costs ~27 lines (signature + request + one `located_query` call + a `wire_request` arm) against ~50 unfactored. The +32 overhead is repaid by the second added method, so group 4 alone clears it.
  - rustfmt sets the floor here: `struct_lit_width` is 18, so the six-field `PositionRequest` literal expands to 8 lines at each of the three call sites no matter how it is written. Grouping fewer values instead pushes the helpers past clippy's 7-argument limit.
  - Re-measure after group 6, when the file holds nine methods, against the ~50-lines-per-method unfactored projection.

## 4. Position-based methods

- [x] 4.1 Add `find_type_definition` and `find_implementations` over the factored shape, reusing the definition normalization, the cap of 20, and the truncation metadata
  - Both go through `located_query` with `normalizer.definitions`, so the cap of 20 and the truncation metadata come from the same place `find_definition` gets them and cannot drift.
  - `SemanticMethod` gained two variants, appended to `ALL` rather than inserted: that order is what the settings card renders. `advertised()` reads `type_definition_provider` and `implementation_provider`, which are their own provider types rather than `OneOf`, so they get their own arms.
  - `build_initialize_params` now declares `typeDefinition` and `implementation` client capabilities. A server reads that list to decide what to index; asking for something we never send is as wrong as not asking for something we do.
  - `api.rs` gained matching entry points and a shared `resolve_query`, so a new entry point cannot reach a server picked for the wrong language. Both are marked `expect(dead_code)` until group 7 wires the catalog — `expect`, not `allow`, so the attribute fails the build once it is wired.
  - Measured cost of the two methods, against the ~50-lines-each the unfactored shape would have taken: coordinator 594 -> 658 (+64). Ten of those are the temporary dead-code annotations group 7 deletes, so ~27 each is the standing cost — the marginal figure recorded at 3.2. Group 3's +32 overhead is repaid.
- [x] 4.2 Add tests: location links normalize into the common shape, an empty result is `ready` rather than unavailable, and an unadvertised method returns unavailable without a request being sent
  - The fixture server answers `typeDefinition` in LocationLink form with a wide `targetRange` and a narrow `targetSelectionRange`; the test asserts columns 4..9, so a fallback to the enclosing range would fail rather than pass silently.
  - `implementation` answers with an empty array and the test asserts `ready` with `total: 0`. `unavailable` would say the server could not answer, which sends the agent looking again.
  - The unadvertised case uses a new `lsp-unadvertised` fixture mode that **exits** on either request. Asserting only the outcome would also pass if the request went out and the server declined it, so the evidence is a definition query still being answered afterwards.
  - `all_lists_every_semantic_method` stopped compiling on the new variants, which is the guard from group 2 working. Suite: `code_intelligence` **192 -> 195** (+2 coordinator, +1 negotiation).

## 5. Symbol methods

- [x] 5.1 Add `find_workspace_symbols`: bounded query string, cap of 50, workspace filtering with a reported filtered count, and an invalid-input status for an empty query. This is the one method with no document, so it skips admission and the lease
  - `prepare` split into `admit` (acquire + advertise check) and the document half, so this query gets a server without getting a lease. It returns `ready_without_document`: there is no version to report and inventing one would let a caller compare it against a real one.
  - An empty or whitespace query is refused before anything is sent. The servers that answer it answer with the whole index, and the ones that do not disagree about what it means.
  - `relative_path` stays in the signature as an **anchor**, not a scope. Recorded as a decision in design.md: LSP has no "the repository", a server indexes one project root, and a repository can hold several.
- [x] 5.2 Add `get_document_symbols`: flattened to a bounded depth, each entry carrying its enclosing symbol's name, with symbols past the bound counted as truncated
  - `document_query` is the sibling of `position_query` for a method that names a document but no position inside it; `PositionRequest` became `QueryRequest` and the position travels separately.
  - The depth bound stops the recursion, not just the emission — a server-controlled response walked to arbitrary depth is a stack overflow waiting for a malformed one. Unwalked subtrees set `truncated` and are deliberately not counted in `total`; that reversal is recorded in design.md.
  - `normalize_location` split so the half that runs after the file is known workspace-relative is reusable: nested document symbols carry no URI, so they start there.
- [x] 5.3 Add tests including a nested response and a flat response producing the same shape, and a rejected document never reaching the server
  - A new `lsp-flat-symbols` fixture mode answers `documentSymbol` in the flat `SymbolInformation` form with the same content as the nested one, and the test asserts the two normalize to an **equal `Vec<NormalizedSymbol>`**. Which form a server picks is not something the Agent should be able to tell.
  - Rejection is asserted on both an escaping path and an absent one; admission refuses before a lease exists, so nothing is sent.
  - Workspace symbols: 55 in-workspace matches plus one outside → 50 returned, total 55, truncated, filtered 1, and no document version.
  - Suite: `code_intelligence` **195 -> 198**. The known load-sensitive `initialize_timeout_...` test failed on the first run and passed on the re-run, as recorded at 2.10.

## 6. Call hierarchy

- [ ] 6.1 Add `find_call_hierarchy` over `prepareCallHierarchy` then `incomingCalls` or `outgoingCalls`, with **one** deadline covering the whole exchange rather than the single-request budget per step
- [ ] 6.2 Preparation resolving several items uses the first and reports that the rest were not followed; preparation resolving none returns `ready` with an empty list and sends no calls request
- [ ] 6.3 Add a test that cancelling between the two steps issues no further request and completes bounded cleanup
- [ ] 6.4 Add `ContextSourceKind::LspCallRelation` and feed relations into the candidate pipeline with the provenance definitions and references already carry
- [ ] 6.5 Confirm the "supported call relations" clause in `lsp-code-intelligence` and `agent-context-engine` is now satisfied by an implemented source rather than vacuously

## 7. Tool catalog and Agent surface

- [ ] 7.1 Append the five tools to the native catalog **after** every existing entry. Inserting among them changes the provider's cached tool-definition prefix
- [ ] 7.2 Update the hard-coded tool-count assertions to the new count rather than deriving it, so a tool added by accident still fails something
- [ ] 7.3 Add a test that the previously existing tools keep their declaration order
- [ ] 7.4 Add deterministic `unavailable` envelopes for the five new tools in the Web/mock runtime, and extend the adapter conformance test
- [ ] 7.5 Add the tool-name and capability-label locale strings to all five bundles and extend `lsp-settings-localization.test.ts`

## 8. Documentation

- [ ] 8.1 Update the user guides' tool table for the five new tools, and say plainly which ones a server may not advertise
- [ ] 8.2 Update the developer guides for the negotiated method list, the call-hierarchy deadline, and the append-only catalog rule
- [ ] 8.3 Correct the developer guides' exclusion list, which currently names call and type hierarchy as excluded
- [ ] 8.4 Run `npm run docs:check`

## 9. Verification

- [ ] 9.1 `npm run lint:ci`
- [ ] 9.2 `npm run test`
- [ ] 9.3 `npm run build`
- [ ] 9.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 9.5 `cargo check --workspace`
- [ ] 9.6 `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] 9.7 `npm run native:panic:check`
- [ ] 9.8 `cargo test --workspace`
- [ ] 9.9 `npm run architecture:check`, `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`
- [ ] 9.10 `npx playwright test`
- [ ] 9.11 `npm run desktop:unit:test`, then `npm run test:desktop` — which builds first. Never run a single layer script against a stale binary: it reports the pre-fix failure and reads like the fix did not work
- [ ] 9.12 `openspec validate expand-lsp-read-only-methods --strict` and `openspec validate --specs --strict`
- [ ] 9.13 Simulate the archive merge with `buildUpdatedSpec`

## 10. Acceptance

- [ ] 10.1 Confirm the task 2.10 checkpoint held: the refactor alone changed no test outcome
- [ ] 10.2 Confirm the frontend changed once, for the capability list, and not again for any of the five methods
- [ ] 10.3 Confirm no database migration was added and the highest migration number is unchanged
- [ ] 10.4 Compare the task 1.3 measurement against the new tool-definition size and state the increase rather than leaving it unmeasured
- [ ] 10.5 Confirm the read-only invariant is intact: no mutating request is sent anywhere, and `workspace/applyEdit` is still rejected
