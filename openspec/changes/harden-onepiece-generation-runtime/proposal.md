## Why

A source audit of the OnePiece native-agent runtime (baseline `main`, 2026-09-04) confirmed five generation-integrity gaps that no current spec or change covers:

- **No immutable generation snapshot** — `run_generation` and its neighbors take many independently-injected ports (`#[allow(clippy::too_many_arguments)]` across `api_process_adapter`), so the configuration a generation runs under is assembled piecemeal and can drift mid-turn instead of being frozen once.
- **A fixed 32K context budget path** — the context-engine evidence budget is hardcoded (`generation.rs`: `model_capacity: Some(32_768)`, `total: 32_768`) rather than derived from the active profile/endpoint/model capacity, so evidence selection is blind to the actual context window even though the compaction trigger already resolves real capacity from the model catalog.
- **Retrieval evidence is spliced into the user prompt** — selected evidence is appended to `request.effective_prompt` inside a `<context-evidence>` block (`generation.rs`), mixing untrusted retrieved content into the user's own text and blurring the prompt-injection boundary.
- **The synthetic compaction summary rides as `role: "user"`** — both compaction paths insert the summary as a user turn; the optimizer path at least carries an identifying marker, the compatibility path carries none, so a synthetic summary can be mistaken for a direct user statement (documented in the compaction chapter).
- **No durable tool-call journal** — effectful tool executions have no persisted effect log or unknown-effect recovery protocol; after a crash mid-tool-call, the runtime cannot tell whether the side effect happened, and a replay could repeat file writes, commands, or external calls. Observability spans record what was seen, but they are not a recovery contract.

The source study also proposed memory-side changes (audience/ACL semantics, extraction decoupling, multi-agent episodes); those are already carried by `strengthen-governed-cross-session-memory` and are out of scope here. Two of the study's gap statements were corrected during verification: recall does not search the whole shared pool (it searches the active + global + all-Agents compatibility view, so narrowed memories are not recallable at all), and automatic extraction is not uniformly "attached to compaction" (OnePiece extracts only on the compatibility fallback path; CLI extraction runs after turn delivery).

## What Changes

- Freeze one **immutable generation snapshot** per OnePiece generation/seat turn — provider profile and model, resolved capacity, tool catalog, personalization/skill/memory views, and permission revision — assembled once at turn start and carried as a single value; mid-turn configuration edits affect the next turn only.
- Derive the context-engine budget from the **actual capacity of the active endpoint/model** (with the model catalog and explicit profile overrides as sources), removing the fixed 32K path; character-only fallback remains for models without capacity metadata.
- Carry retrieved evidence in a **typed evidence envelope** separate from the user's text — evidence reaches the provider as its own clearly-attributed content part or system-side section, never appended inside the user's message.
- Give the synthetic compaction summary a **provider-neutral carrier** so it is never mistakable for user speech: both compaction paths must emit an identifiable synthetic-summary form (the exact mapping per interface format is a design decision recorded in this change).
- Introduce a **durable tool-call journal** for effectful tools: journal-before-execute, record outcome, and a recovery protocol that marks unknown-effect entries for review instead of auto-replaying them.

## Impact

- Affected specs: `onepiece-native-agent`, `agent-context-engine`, `agent-context-compaction`, `agent-tool-execution`.
- Affected code (indicative): `contexts/agent_runtime/infrastructure/api_process_adapter/` (snapshot assembly, prompt/evidence assembly, summary carrier), `application/context_engine.rs` and the capacity catalog (budget derivation), tool dispatch and a new journal store, plus recovery wiring.
- Developer docs: the compaction and OnePiece chapters gain known-gap notes referencing this change; fuller doc updates land with implementation.
- No behavior changes ship with this proposal; the source study's remaining target material (runtime-subject permissions, MCP leases, skill subject binding, event envelopes, and the rest of its seventeen-way split) stays future work and is intentionally not promised here.

## Capabilities

### Modified Capabilities

- `onepiece-native-agent`: the immutable generation snapshot requirement.
- `agent-context-engine`: model-aware budget derivation and the typed evidence envelope.
- `agent-context-compaction`: the provider-neutral synthetic-summary carrier.
- `agent-tool-execution`: the durable tool-call journal and unknown-effect recovery.
