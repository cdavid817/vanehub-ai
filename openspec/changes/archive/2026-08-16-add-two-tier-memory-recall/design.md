## Context

See `proposal.md` — Why for motivation. Depends on `migrate-agent-memory-to-file-store`, which supplies the names, descriptions, types, and `MEMORY.md` index this design consumes.

Two existing constraints shape everything below:

- `agent-cross-session-memory` requires that injected memory content never enter the turns list that compaction manipulates, and that it survive compaction present, complete, and unchanged for the rest of the generation. The system prompt is the only slot with that property, so every memory surface this change adds has to live there.
- Memory injection reaches two surfaces with very different economics. OnePiece's system prompt is assembled per generation and reused across that generation's whole tool loop. The CLI section is prepended to every message handed to a subprocess whose own context budget VaneHub neither controls nor measures.

`RuntimeAgentMemoryExtractionAdapter` already establishes the pattern for a bounded side call that resolves OnePiece's credentials itself; the selector reuses it rather than inventing a second one.

## Goals / Non-Goals

**Goals:**

- Always-on injection cost becomes flat in the number of memories and independent of their size.
- The pool can grow past the point where the old character budget made older memories unreachable.
- A stale memory arrives labelled as stale, in a form the model reasons about.
- A selector failure costs relevance, never the generation.

**Non-Goals:**

- Changing `recall`, the retrieval index, or embedding configuration. The selector is description-driven and automatic; `recall` stays content-driven and model-invoked. They coexist.
- Injecting selected bodies into CLI-wrapped agent prompts.
- Persisting already-surfaced state across sessions.
- A separate cheaper model tier for utility calls.

## Decisions

### D1. Selection runs once per generation, not once per provider round-trip

This is the decision the compaction-survival requirement forces. Selected bodies must live in the system prompt, and a system prompt that changes on every round-trip would invalidate the provider prompt cache on every round-trip inside a tool loop.

Running selection once at generation start makes the system prompt fixed for that generation's entire loop, which is exactly the invariant the requirement asks for and exactly what the cache wants. The prefix then changes between generations, which it can already do today whenever the pool changes.

Within the system prompt, the ordering is stable content first, volatile content last: Skills, then the memory index, then the generation's selected bodies. A prefix cache is a prefix, so putting the one part that changes per generation at the end preserves everything before it.

Alternatives considered. Injecting selected bodies into the turns list as a system-reminder block is what Claude Code does and would allow per-turn selection, but it puts memory content where compaction can delete it, contradicting an existing requirement this change is not reopening. Per-round-trip selection into the system prompt was rejected on cache cost alone.

### D2. Two surfaces, two budgets

| Surface | Line cap | Byte cap | Rationale |
|---|---|---|---|
| OnePiece system prompt | 200 | 12,000 | Assembled once per generation, reused across the loop; amortized over many round-trips |
| CLI effective prompt | 40 | 3,000 | Re-sent with every message, competing with a context budget VaneHub cannot see |

Both caps apply together because either alone is defeated by the other's failure mode: a line cap passes a handful of 2,000-character entries, and a byte cap passes a thousand short ones. Truncation cuts at an entry boundary and appends a line naming which cap fired, so the model can see the index is partial rather than treating it as the whole pool.

The previous requirement that both surfaces share one limit is what this change breaks. It was defensible when both injected the same bullet list; it stops being defensible once one is amortized and the other is not.

### D3. The selector reuses OnePiece's configured model

The selector resolves OnePiece's credentials and provider exactly as the extraction gateway does, sends the manifest plus the query, caps output tightly, and requires structured output — a list of memory names. Names not present in the manifest are discarded before use, so a hallucinated name costs nothing.

Its input is names, types, one-line descriptions, and ages. Never bodies. That is what makes the call's cost proportional to the number of memories rather than their total size, which is the entire point of the split.

The prompt instructs it to return nothing when nothing is clearly useful, rather than its best guess. A selector that always returns something turns the relevance budget into a random sample, which is worse than index-only injection because it looks authoritative.

Alternatives considered. A dedicated cheaper utility model is what Claude Code uses (`getDefaultSonnetModel` for a selection running on a possibly-Opus main loop). VaneHub has no utility-model tier — `model_category.rs` only distinguishes chat from embedding — so introducing one means a settings surface, a provider-config field, and a Web adapter change. That is a separate change; reusing the configured model keeps this one to the injection path.

### D4. Already-surfaced state is session-scoped and mtime-aware

An in-process, session-keyed set of `(path, mtime-at-surface)` pairs. A memory is excluded from the candidate manifest — before the selector call, not after — when its path is present and its mtime is unchanged. A memory whose mtime moved becomes eligible again, because its content is no longer the content the model already saw.

Filtering before the call rather than after it is what makes the bound meaningful: filtering afterwards spends selection slots on memories the caller is about to discard.

Not persisted. A new session should be able to see everything; carrying exclusions across sessions would make old memories progressively harder to surface, which is the failure this whole change exists to remove.

### D5. Age is rendered, not timestamped

Elapsed time is computed from mtime and rendered in words. A memory whose age exceeds one day additionally carries the caveat that it is a point-in-time observation, that file and symbol claims may be outdated, and that it should be verified before being asserted as fact.

The threshold exists because a caveat on something written an hour ago is noise that trains the model to skim past caveats generally. The rendering exists because a raw ISO timestamp requires date arithmetic to interpret, and the interpretation is the part that has to happen for staleness to affect behavior.

### D6. Failure degrades to index-only

Any selector failure — error, timeout, unparseable output, empty structured result — logs and proceeds with index injection alone. The index requires no model call and no embedding configuration, so the memory feature keeps working on an installation with neither.

### D7. Runtime boundary

| Layer | Change |
|---|---|
| React components | None |
| `agent-service.ts` | None. No new command |
| Tauri adapter | None beyond the events already carried for memory injection |
| Web/mock adapter | Simulates index injection and selection through the same events; no provider call, no model |
| Rust `agent_runtime` | Index assembly and body assembly split in `application/models.rs`; selector as a bounded gateway alongside the extraction gateway; session-scoped surfaced-set in session state |
| Rust `retrieval` | None. One spec sentence corrected |

## Risks / Trade-offs

- **A per-generation system prompt change invalidates the provider cache prefix from the memory section onward** → stable sections are ordered ahead of the volatile one, so the invalidation is confined to the tail rather than the whole prefix.
- **The selector adds one model call per generation** → it is bounded in output, sees no bodies, and reuses the credential path already exercised by extraction. It is also skippable: a failure or a disabled toggle costs nothing.
- **A conservative selector returns nothing and the model never sees a relevant memory it would have used** → the index is always present, so the model can still see that the memory exists and reach for it with `recall` or by reading the file directly. Index-only is a degraded state, not a blind one.
- **The CLI cap is much smaller than OnePiece's, so a large pool reaches CLI-wrapped agents heavily truncated** → truncation is signposted, and CLI-wrapped agents keep receiving the most recently modified entries first. Raising the cap is a config change, not a redesign.
- **Session-scoped exclusion state grows with a long session** → bounded by the number of memories, holding only a path and a timestamp per entry.
- **Ordering by mtime means a bulk edit or a filesystem copy reshuffles the index** → acceptable; the index is an orientation aid, and `recall` plus direct reads do not depend on its ordering.

## Open Questions

- Whether the CLI caps should differ per agent id, since these agents' own context budgets differ. Deferrable: it changes no spec and no interface, only the value a lookup returns.
- Whether a dedicated utility-model tier is worth introducing later for the selector and the extractor together. Deferrable: it is additive to both call sites and changes neither's contract.
