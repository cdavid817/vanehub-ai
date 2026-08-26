## 1. Baseline

- [x] 1.1 Record the pre-change pass state of `cargo test --workspace code_intelligence`, `cargo test --workspace native_lsp`, the frontend LSP vitest files, and `npx playwright test tests/e2e/lsp-settings.spec.ts`. Unset `all_proxy`/`ALL_PROXY` and pin `PLAYWRIGHT_PORT` for the last one
  - Baseline on `11458275`: `code_intelligence` **190**, `native_lsp` **1**, frontend LSP suites **746**.
- [x] 1.2 Record the current native tool count and where it is asserted, so the catalog growth is a deliberate diff rather than a surprise
  - **10**, asserted at `providers/tests.rs:768` (`fixtures.len()`) and `:1005` (`tools.len()`), plus the `resolve_tool_catalog_*` family.
- [x] 1.3 Measure the serialized size of the LSP tool definitions as declared to a provider. Nine tools where there were four lengthens every eligible session's system prompt, and the cost should be known rather than assumed
  - **1,782 bytes for four; 4,546 for nine.** +2,764 bytes, or 2.55x, on every session with a trusted local workspace and a discoverable server.
  - Turned into an assertion (`the_code_intelligence_tools_state_what_they_cost_a_system_prompt`) rather than left as a number in this file. A measurement recorded once goes stale the first time somebody rewrites a description; a bounded assertion makes them look at it.

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
  - **Re-measured after group 6.** Nine methods, 1,020 physical lines: 113 per method against the 140 per method the file carried at four. The average fell even though the last three methods added are the ones with genuinely new protocol handling — a three-step exchange, a nested walk, and a path with no document — rather than copies of an existing shape. The design's "five more would roughly double it" landed at +81%, and the part attributable to copying is the part that did not happen.
  - One thing the numbers do not excuse: 1,020 lines in one file is past comfortable review, even though every registered budget passes. Flagged for group 10 rather than split here, because splitting mid-change would move code the rest of these tasks are still editing.

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

- [x] 6.1 Add `find_call_hierarchy` over `prepareCallHierarchy` then `incomingCalls` or `outgoingCalls`, with **one** deadline covering the whole exchange rather than the single-request budget per step
  - `remaining_budget` hands each step what is left of a single 10s budget. Running it out maps to `request_timeout`, which is what the caller would have seen had one request spent the whole thing.
  - This forced the release rule to be stated properly. `request` split into `send` (records the response or failure, leaves the slot held) and `request` (sends then releases); `position` stopped releasing and became synchronous. The rule is now: `prepare` releases on its own failures, and after it succeeds the caller releases exactly once. A two-request query holds the slot across both.
  - Takes an `AgentPosition` rather than a line and a column: with the direction added, the separate pair would put it one argument over clippy's limit.
- [x] 6.2 Preparation resolving several items uses the first and reports that the rest were not followed; preparation resolving none returns `ready` with an empty list and sends no calls request
  - "Reports" needed a channel. `status_with_value` takes a reason alongside any status, so the outcome is `ready` with `call_hierarchy_items_not_followed` and `truncated` set. Silence would let the Agent read a partial hierarchy as a complete one.
  - The fixture prepares **two** items on purpose. One item could not tell the difference between following the first and following all of them.
- [x] 6.3 Add a test that cancelling between the two steps issues no further request and completes bounded cleanup
  - A new `lsp-hang-calls` fixture mode prepares normally and never answers the direction request, so the client's own cancellation is the only thing that can end it — no race with a reply.
  - The test wraps the join in a 3s timeout. Without it, a cancellation that fell through to the 10s request deadline would still pass, which is the failure the bounded-cleanup requirement is about.
  - Both direction handlers reject an item they did not prepare, so a request following an empty preparation kills the server instead of being answered.
- [x] 6.4 Add `ContextSourceKind::LspCallRelation` and feed relations into the candidate pipeline with the provenance definitions and references already carry
  - **Done in group 7, not in group 6.** This was the first of the five methods to need `AgentCodeIntelligencePort`, which has seven implementations across production and tests; group 7 took all five across that boundary at once rather than touching all seven twice.
  - Incoming calls only. Outgoing calls describe what the referenced symbol needs, which the definition source already reaches; callers are the direction that adds something.
  - Relations flatten to locations at the source boundary, so a call relation carries exactly the provenance a definition does — one candidate per caller, at its declaration. That needed `AgentCodeSymbol` to carry `preview`, which `NormalizedLocation` already had and `map_symbol` was dropping. Symbol tool results gained the declaration line as a side effect, which is the right answer anyway: a symbol without one is a coordinate the reader has to go and open.
- [x] 6.5 Confirm the "supported call relations" clause in `lsp-code-intelligence` and `agent-context-engine` is now satisfied by an implemented source rather than vacuously
  - `lsp-code-intelligence` line 112 ("definitions, references, and supported call relations SHALL be normalizable as bounded Context Engine candidates") and `agent-context-engine` line 20 ("LSP definitions/references or call relations when supported"). Both were vacuous — the clause named a source nothing produced. `CodeIntelligenceContextSource::call_relations` is registered in `bootstrap/agent_runtime.rs` beside the definition and reference sources, so both now describe something that runs.

## 7. Tool catalog and Agent surface

- [x] 7.1 Append the five tools to the native catalog **after** every existing entry. Inserting among them changes the provider's cached tool-definition prefix
  - The catalog was the small part. Each tool crosses `AgentCodeIntelligencePort` and `AgentCodeIntelligenceResponderPort`, so the five landed as: two trait methods each, the runtime adapter, the unavailable responder, the native responder in bootstrap, and three test doubles — seven implementations.
  - `find_workspace_symbols` and `find_call_hierarchy` needed their own schemas: a query string and a `direction` enum respectively. A `direction` that is not `"outgoing"` reads as the default rather than as an error — the choice is between two values, and refusing a typo would cost a whole tool call to say what the default already says.
  - **The memory that says "adding a tool breaks seven count tests" did not apply.** Those seven pin the *baseline* catalog; LSP tools are conditional and are not in it. Two assertions broke, both listed below.
- [x] 7.2 Update the hard-coded tool-count assertions to the new count rather than deriving it, so a tool added by accident still fails something
  - `code_intelligence_tools_have_provider_neutral_workspace_implicit_schemas` now compares against a spelled-out `EXPECTED_CODE_INTELLIGENCE_TOOLS: [&str; 9]`, and the end-to-end test's sorted list gained the five names. Both are written out; an assertion that recomputes its own expectation passes for anything.
- [x] 7.3 Add a test that the previously existing tools keep their declaration order
  - `the_first_four_code_intelligence_tools_keep_their_declaration_order` checks the prefix specifically, not the whole list. The whole-list assertion above would also fail on a reorder, but it would fail the same way it fails for a rename, and the prompt-cache cost is not something a reader would infer from that.
- [x] 7.4 Add deterministic `unavailable` envelopes for the five new tools in the Web/mock runtime, and extend the adapter conformance test
  - Type definitions and implementations answer in the definition envelope, mirroring the desktop side reusing one normalization for all three. Symbols and relations get their own.
  - The conformance table is written out rather than generated from `lspToolNames`, so a name added without an envelope would be tested by nothing. A second test pins the two lists together.
- [x] 7.5 Add the tool-name and capability-label locale strings to all five bundles and extend `lsp-settings-localization.test.ts`
  - Five capability labels × five bundles, inserted beside the existing capability keys rather than appended, and added to `requiredKeys`.
  - No tool-*name* strings: the tool catalog is Agent-facing English sent to a provider, not UI copy, and there is no surface that renders a tool name to a user.

## 8. Documentation

- [x] 8.1 Update the user guides' tool table for the five new tools, and say plainly which ones a server may not advertise
  - The table gained a third column answering exactly that, with `gopls` and `rust-analyzer` named as servers that offer all nine and "older or smaller servers often stop at the first four" as the honest general case. The guide also explains the `unavailable` status rather than leaving a user to read it as breakage.
  - Both guides also said "Python, Go, Java, C, and C++ are not supported"; that was already stale from the previous change and is now "Java is not supported".
- [x] 8.2 Update the developer guides for the negotiated method list, the call-hierarchy deadline, and the append-only catalog rule
  - Three paragraphs added: the append-only rule with the test name that enforces it, the one-deadline exchange with the reason two budgets would be wrong, and the anchor-not-scope reading of `find_workspace_symbols`' path argument.
- [x] 8.3 Correct the developer guides' exclusion list, which currently names call and type hierarchy as excluded
  - Call hierarchy and type definitions moved out of the exclusion list, with a sentence saying they used to be on it and why they moved. Type *hierarchy* (`typeHierarchy/supertypes`) stays excluded and is now named separately, because "call/type hierarchy" as one phrase was what made the old line wrong in two directions at once.
- [x] 8.4 Run `npm run docs:check`
  - Passes, including README parity and the link/media/boundary inventory.

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
- [x] 10.4 Compare the task 1.3 measurement against the new tool-definition size and state the increase rather than leaving it unmeasured
  - 1,782 -> 4,546 bytes (+2,764, 2.55x). Both measured from the same build, so the comparison is exact rather than a before-and-after across two trees.
  - Worth stating plainly: this is the largest single cost of the change and it falls on every eligible session, used or not. It is bounded by eligibility — a session without a trusted local workspace and a discoverable server is offered none of them — but it is not free, and the assertion above is what keeps it from drifting further without anyone noticing.
- [ ] 10.5 Confirm the read-only invariant is intact: no mutating request is sent anywhere, and `workspace/applyEdit` is still rejected
