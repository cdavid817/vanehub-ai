## Context

See `proposal.md` — Why. The state the two previous changes left:

- `SemanticMethod` is an enum with four variants, and `NegotiatedCapabilities` carries one `bool` field per variant plus `position_encoding` and `document_sync`.
- `NegotiatedCapabilities::supports(SemanticMethod)` is a match from variant to field — the compiler enforces that the two stay aligned, which is worth keeping.
- `initialize_negotiation.rs` reads each capability out of the initialize result by hand.
- `semantic_query_coordinator.rs` has one `async fn` per method, each ~60 lines of the same shape: prepare, request, record response or failure, release, normalize.
- Nothing in the frontend names a language any more; the status card still names all four capabilities.
- The whole-object DTO fixtures added in `extend-lsp-language-registry` pin the serialized shape of every command result.

## Goals / Non-Goals

**Goals:**

- Adding a semantic method costs its protocol handling and its normalization, and nothing else. No DTO field, no TypeScript field, no status row, no contract branch.
- Call hierarchy behaves as one bounded operation rather than three requests that happen to run in sequence.
- The read-only invariant is untouched. Nothing here sends a mutating request or accepts `workspace/applyEdit`.

**Non-Goals:**

- Making `SemanticMethod` data-driven. A method is not data — each one needs protocol handling and a result shape that only code can supply. What becomes data-driven is how the negotiated result is *represented and carried*, not what the set is.
- Write methods. `rename`, `codeAction`, and `formatting` break the read-only invariant and need the permission and audit path; mixing that into a capability expansion would make both hard to review.
- `completion` and `semanticTokens`. The Agent is not typing and does not highlight.

## Decisions

### 1. The method set stays an enum; the negotiated record becomes a list over it

`SemanticMethod` gains five variants and a `const ALL` in a fixed order. `NegotiatedCapabilities` replaces its four booleans with one entry per variant of `ALL`, each carrying the method and whether the server advertised it.

That gives the two distinctions the spec asks for without a second vocabulary. A method the client does not implement is not a variant, so it cannot appear. A method the client implements but the server omits is a variant with `supported: false`, which is different from absent. And because the list is built by iterating `ALL`, its order is the enum's declaration order on every server — a renderer never has to sort.

Rejected: a `HashSet<SemanticMethod>` of supported methods. It collapses "unsupported" into "absent", so the status surface could not show a method as unsupported without knowing the full set independently — which is the compiled-in list this change exists to delete.

Rejected: keeping the booleans and adding five more. Nine places would each go from four lines to nine, and the tenth method would cost the same again.

`supports()` keeps its signature and becomes a lookup. The exhaustive match disappears, so a variant added without a capability mapping would silently report unsupported rather than failing to compile; a test asserting every `ALL` variant maps to a real LSP capability path replaces what the compiler was doing.

### 2. Position encoding and sync mode stay fields

They are not methods and have no supported/unsupported axis. Folding them into the list to make it uniform would mean inventing a `supported` value for a setting, which is worse than two extra fields.

### 3. Call hierarchy owns one deadline for the whole exchange

`prepareCallHierarchy` then `incomingCalls` or `outgoingCalls` is one operation from the Agent's view, so it gets one budget rather than each step carrying the single-request deadline. Two steps at the single-request budget would let a slow server take twice as long as any other tool while every individual request looked healthy.

Preparation returning several items resolves to the first in the server's order, with the rest reported as not followed. Following all of them would multiply the request count by an amount the server chooses, which is exactly the kind of unbounded fan-out the rest of this context refuses.

### 4. The five coordinator methods reuse one shape

`find_type_definition` and `find_implementations` differ from `find_definition` only in the method string, so they share its normalization and cap. `find_workspace_symbols` is the one method that is not position-based — it takes a query string and is not scoped to a document, so it skips document admission and the lease entirely, which also means it is the only new tool that can run without opening a file.

`get_document_symbols` flattens rather than returning a tree. A tree would make the result shape recursive and unbounded in depth, and an Agent reading an outline wants the names, not the nesting structure; each entry carries its enclosing symbol's name so the nesting is still recoverable.

### 5. Tools are appended, and the catalog order is now a spec requirement

New tools go after every existing one. Inserting among them changes the tool-definition prefix a provider caches, which silently costs every session its prompt cache. This was a comment in the previous change's task list; it is a requirement now because it is a property a reviewer cannot see in a diff.

## Risks / Trade-offs

- **Losing the exhaustive match in `supports()`** → a new variant with no capability mapping would report unsupported everywhere and look like a server problem. Replaced by a test over `SemanticMethod::ALL` asserting each maps to a capability path, which fails the build the same way the match did.
- **The whole-object DTO fixtures will fail** → that is them working. The negotiated-capability shape is exactly what they pin. Update them to the new shape deliberately; loosening them to stop the failure would remove the only thing that makes a contract change reviewable.
- **Seven hard-coded tool-count assertions** → `assert_eq!(tools.len(), 10)` twice in `providers/tests.rs` plus several `resolve_tool_catalog_*` tests. They fail as soon as the first tool is added. Update them to the new count rather than deriving it, so a tool added by accident still fails something.
- **The desktop layer is the only one exercising the raw IPC payload** → `add-lsp-go-python-cpp` proved that: a required-field change passed 4551 Rust and 1660 frontend tests and failed only there. A capability shape change is the same class of risk, so run the desktop layer before believing the refactor is behavior-preserving, and remember the layer scripts do not rebuild.
- **Nine new tools where there were four** → more tool definitions means a longer system prompt for every session with LSP available. The tools are only offered for a trusted local workspace with a discoverable server, so the cost falls on sessions that can use them, but it is a real cost and worth measuring rather than assuming.
- **`semantic_query_coordinator.rs` grows past a reviewable size** → five more methods of the same ~60-line shape would roughly double it. The shared parts (prepare, request, record, release) should be factored before the fifth method rather than after, or the file becomes the thing nobody reads.

## Migration Plan

No database migration. Negotiated capabilities are runtime state recomputed at every initialize, never persisted, so there is nothing stored in the old shape to convert.

Rollback is code-only. A downgraded build renegotiates capabilities on its next initialize and produces the old shape; nothing on disk carries the new one.

## Open Questions

- Whether `find_workspace_symbols` should accept a result-kind filter. Servers vary in how much they return for a short query, and a filter would let the Agent ask for types only. Nothing in this change depends on the answer, and adding it later needs no shape change because the tool already carries a bounded input object.
