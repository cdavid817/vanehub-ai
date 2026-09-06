## 1. Generation snapshot

- [ ] 1.1 Define the `GenerationSnapshot` value (profile + revision, endpoint, model, resolved capacity, frozen tool catalog, effective view references, permission revision, generation options) and assemble it once at turn start.
- [ ] 1.2 Thread the snapshot through prompt assembly, the tool loop, compaction, and extraction, retiring the per-stage port fan-out that motivated `too_many_arguments`.
- [ ] 1.3 Add a test proving a mid-turn settings edit does not change the running turn's behavior and does change the next turn's.

## 2. Model-aware context budget

- [ ] 2.1 Derive `ContextBudget` from the snapshot's resolved capacity (profile override → model catalog → conservative default) with proportional reserves; remove the fixed `32_768` path.
- [ ] 2.2 Keep the character-only fallback for models without capacity metadata, surfaced as measurement quality rather than silently.
- [ ] 2.3 Add tests for a large-context model, a small-context model, and a capacity-lookup failure.

## 3. Typed evidence envelope

- [ ] 3.1 Introduce the evidence envelope type with provenance and redaction state; render it per interface format as a non-user section.
- [ ] 3.2 Stop appending `<context-evidence>` into `effective_prompt`; add a test asserting the user message reaches the provider verbatim.
- [ ] 3.3 Update the context-evidence manifest so the envelope's contents remain auditable.

## 4. Synthetic-summary carrier

- [ ] 4.1 Emit the `ContextSummary` carrier from both compaction paths; settle the per-format mapping decision recorded in the design.
- [ ] 4.2 Keep rendering historical marker-style summaries; add a compatibility test over an old transcript.

## 5. Durable tool-call journal

- [ ] 5.1 Add the journal store with Intent → Executing → Completed/Failed/UnknownEffect states and stable identities; journal-before-execute for effectful tools.
- [ ] 5.2 Wire recovery: UnknownEffect entries surface for review and are never auto-replayed; duplicate terminal delivery cannot double-execute.
- [ ] 5.3 Add crash-recovery tests around the execute boundary for an effectful tool and a read-only tool.

## 6. Documentation

- [ ] 6.1 Update the compaction, OnePiece, and tool-registry chapters (both languages) once behavior lands; remove the known-gap notes this change added.
