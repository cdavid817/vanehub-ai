## Why

The LSP foundation implements three request methods — `textDocument/definition`, `references`, `hover` — plus the pushed `publishDiagnostics` cache. The Agent therefore has no way to ask "where is the symbol named X anywhere in this workspace", "what does this file contain", or "who calls this function", and falls back to text search for questions a language server can answer precisely. Two existing specs already promise call relations conditionally: `lsp-code-intelligence` says definitions, references "and supported call relations" become Context Engine candidates, and `agent-context-engine` says the same. Because no call-hierarchy method is implemented, both clauses are currently vacuous.

## What Changes

**First, capabilities stop being fixed fields.** `NegotiatedCapabilities` carries one boolean per method — `definition`, `references`, `hover`, `diagnostics` — and that shape is restated in nine places: the domain struct, its `supports` match, the initialize parser, the command DTO, the TypeScript type, the contract validator, the runtime status card's hardcoded array, five locale bundles, and the Web mock. Adding one method means editing all nine; adding five means doing that five times.

This is the same shape of problem `extend-lsp-language-registry` solved for languages, on a different axis, and `add-lsp-go-python-cpp` just demonstrated what solving it is worth. So capabilities become a list of negotiated methods, and the nine places become one declaration plus consumers that iterate. The refactor lands first and changes no behavior: the existing suites passing unchanged are its acceptance.

Then the methods:

- Add `workspace/symbol` as a new Agent tool: workspace-wide symbol search by name, bounded and workspace-filtered like the existing location results.
- Add `textDocument/documentSymbol` as a new Agent tool: the symbol outline of one admitted file, with nesting flattened to a bounded depth.
- Add `textDocument/typeDefinition` and `textDocument/implementation`, normalized into the same result shape `find_definition` already produces.
- Add call hierarchy as one Agent tool over the three-step protocol (`textDocument/prepareCallHierarchy`, then `callHierarchy/incomingCalls` or `outgoingCalls`), with its own result cap and truncation metadata. This makes the "supported call relations" clause in `lsp-code-intelligence` and `agent-context-engine` real rather than conditional.
- Feed call relations into the Context Engine alongside the existing `LspDefinition` and `LspReference` candidate kinds.
- Gate every new method on the existing capability negotiation, so a server that does not advertise a method returns `unavailable` without a request being sent, exactly as hover does today.
- Reuse the existing status vocabulary (`ready`, `warming`, `timeout`, `unavailable`, `failed`), position normalization, workspace filtering, and document-lease machinery unchanged. No new safety mechanism is introduced because none is needed.

Deliberately excluded, with reasons:

- `textDocument/completion` — the Agent is not typing. Completion returns large, low-signal result sets; `workspace/symbol` is the tool it actually needs.
- `textDocument/rename`, `codeAction`, `formatting` — these are write operations. They would break the read-only invariant that `lsp-server-management` states explicitly ("the client SHALL reject the edit without changing any file") and would need the permission and file-mutation audit path. Mixing a security-model change into a capability expansion would make both hard to review. A separate change should carry them.
- `semanticTokens` — syntax highlighting has no Agent consumer.

## Capabilities

### New Capabilities

None. This extends the existing code-intelligence capability rather than adding a new one.

### Modified Capabilities

- `lsp-code-intelligence`: the conditional-tool-availability requirement covers the new read-only tools; new requirements define bounded, workspace-scoped, normalized results for workspace symbol search, document symbols, type definition, implementation, and call hierarchy, including per-method result caps and truncation metadata; the Context Engine candidate requirement stops treating call relations as hypothetical.
- `agent-tool-execution`: the native read-only LSP tool requirement stops enumerating exactly four tools and instead covers the full read-only set, preserving the same cancellation, hard-limit, and visible persisted tool-use lifecycle semantics.
- `lsp-server-management`: the capability-negotiation requirement stops describing a fixed set of negotiated flags and instead requires a negotiated method list, so a method the client does not implement and a method the server does not advertise stay distinguishable without the vocabulary being compiled in.
- `settings-center-ui`: the runtime status surface reports one row per negotiated method rather than a compiled-in list of four, so a method added later appears without a frontend change — the same property the language cards gained.

## Impact

**Runtimes affected: desktop and Web.** Each new tool needs a matching deterministic `unavailable` envelope in the Web/mock runtime to keep adapter conformance honest.

Frontend/backend isolation is unchanged. The frontend changes twice and then stops: once for the capability refactor, which replaces the status card's hardcoded array with iteration over the reported list, and not again for any of the five methods.

Affected code:

- `src-tauri/src/contexts/code_intelligence/infrastructure/{semantic_query_coordinator,semantic_results,initialize_negotiation}.rs`
- `src-tauri/src/contexts/agent_runtime/{application/tool_catalog.rs, infrastructure/code_intelligence_adapter.rs, infrastructure/context_sources.rs}`
- `src-tauri/src/contexts/code_intelligence/domain/models.rs` and `commands/code_intelligence/dto.rs` for the capability list
- `src/services/{lsp-contract,web-lsp-client}.ts`, `src/types/lsp.ts`, and `src/settings/pages/agents/lsp-runtime-status-card.tsx`

Known hazards this change must handle rather than discover late:

- New tools must be appended to the end of the native tool catalog. Inserting them earlier changes the system-prompt prefix and destroys prompt caching for every session.
- Two assertions hard-code the baseline tool count (`assert_eq!(tools.len(), 10)` in `contexts/agent_runtime/infrastructure/providers/tests.rs`), and several `resolve_tool_catalog_*` tests assert catalog contents. They fail as soon as a tool is added and must be updated deliberately, not silently relaxed.
- Call hierarchy is a three-request protocol, not a single round trip, so it needs its own deadline accounting rather than reusing the single-request budget.
- The capability refactor changes a serialized shape the status surface reads. The whole-object DTO fixtures added in `extend-lsp-language-registry` will fail, which is them working; update them deliberately rather than loosening them.
- `add-lsp-go-python-cpp` showed that the desktop layer is the only one that exercises the raw IPC payload. A capability shape change is exactly the kind of thing that passes every unit test and fails there, so run it before believing the refactor is behavior-preserving.

Dependency: none outstanding. `add-lsp-go-python-cpp` has landed, so this change starts from a registry whose per-language cost is already paid; the two touched the same context and were sequenced to avoid conflicting there.
