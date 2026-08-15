## Why

Once `migrate-agent-memory-to-file-store` lands, memories are addressable files with names and descriptions, but injection still works the way the row store forced it to: take the pool newest-first and stop at a character budget. That strategy has a fixed ceiling. Everything past the budget is invisible no matter how relevant it is, and the ceiling is reached quickly because full memory bodies are being spent on the budget.

Splitting the surfaces removes the ceiling. A one-line index entry is cheap enough to always carry; a full body is expensive and only worth carrying when the current turn actually calls for it. With that split the pool can grow without bound while the always-on cost stays flat, and the `recall` tool stops guessing — it can see what exists before searching for it.

The pool is also getting older. Memories are point-in-time observations, and a stale one asserted as current fact is worse than no memory, because a specific file path or function name makes a wrong claim sound authoritative. The model needs to be told how old a memory is, in a form it will actually reason about.

## What Changes

- The always-injected memory surface becomes the `MEMORY.md` index rather than memory bodies. Every generation carries one line per memory — a pointer plus a short hook — so the model knows what exists.
- Full memory bodies are injected only when selected as relevant to the current turn. Selection runs over a manifest of names, types, descriptions, and ages rather than over content, so its cost is proportional to the number of memories rather than to their size.
- Selection is deliberately conservative: it returns a bounded number of memories, is instructed to prefer returning nothing over returning a maybe, and excludes memories already surfaced earlier in the same session so its budget goes to fresh candidates.
- Injected memories carry a human-readable age rather than a raw timestamp, and memories past a staleness threshold carry an explicit caveat that they are point-in-time observations to be verified before being asserted as fact.
- Index injection is bounded by both a line cap and a byte cap. Either cap alone is insufficient: a small number of very long index lines slips past a line cap while still flooding the prompt. Truncation is signposted in the injected text so the model can see the index is incomplete and act on it.
- OnePiece and CLI-wrapped agents stop sharing one injection budget. **BREAKING** relative to the current requirement that the two surfaces behave identically: OnePiece's index is injected once into a cached system prompt, while a CLI-wrapped agent's is prepended to every message, so the same limit cannot be right for both.
- The `recall` tool keeps its name, position, and schema. It remains the content-driven search path and is unchanged by this proposal; the index simply gives the model something to aim it at.

Affects both runtimes. The Web/mock runtime keeps event and contract parity for the new selection step without issuing a model call, as it already does for injection and extraction.

## Capabilities

### New Capabilities

None. This change replaces the injection strategy of an existing capability.

### Modified Capabilities

- `agent-cross-session-memory`: system-prompt injection carries the index rather than bodies, with relevance-selected bodies added per turn; injected memories carry age and staleness caveats; the index is bounded by paired line and byte caps with signposted truncation; the OnePiece and CLI injection budgets separate.
- `retrieval-vector-search`: the requirement that recency-based memory injection continues unchanged when no embedding source is configured no longer describes reality, since injection is no longer recency-based; index injection must remain functional without any embedding configuration.

## Impact

- `src-tauri/src/contexts/agent_runtime/application/models.rs`: `format_memory_section` splits into index assembly and relevance-selected body assembly, with separate budgets per surface.
- A per-turn relevance selection call is added to the OnePiece generation path. It is bounded, non-agentic, and its failure degrades to index-only injection rather than failing the generation.
- Session-scoped state is added to track which memories have already been surfaced, so repeat selections do not re-spend the budget on the same files.
- The CLI Prompt Hook injection point is unchanged in position and ordering; only the content and the budget change.
- `src-tauri/src/contexts/retrieval/`: no behavior change, one spec sentence corrected.
- Frontend service boundary is unchanged. No new Tauri command is introduced; both the Tauri adapter and the Web/mock adapter change together for event parity.
- Depends on `migrate-agent-memory-to-file-store`, which supplies the names, descriptions, types, and index this proposal consumes.
