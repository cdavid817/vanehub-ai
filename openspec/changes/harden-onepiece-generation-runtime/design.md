# Design: harden-onepiece-generation-runtime

Distilled from the OnePiece native-agent design study (baseline `main`, 2026-09-04). Every "current" claim below was re-verified against source; target material is marked as such and ships nothing by itself.

## Scope

Five generation-integrity mechanisms for the `launch_kind = api` runtime: the immutable generation snapshot, model-aware context budgeting, the typed evidence envelope, the provider-neutral synthetic-summary carrier, and the durable tool-call journal. The study's memory family is owned by `strengthen-governed-cross-session-memory`; its runtime-subject permission model, MCP runtime leases, skill subject binding, and event envelopes are future changes, deliberately excluded.

## The immutable generation snapshot

**Current**: generation inputs are gathered from many independently-injected ports at call time; nothing pins them together, so a settings edit mid-turn can be observed by later steps of the same turn (the personalization snapshot alone is already per-generation — the rest of the configuration is not).

**Target**: one value assembled at turn start and threaded through everything the turn does:

```text
GenerationSnapshot
├─ provider profile id + revision, endpoint, interface format, model id
├─ resolved model capacity (context window, max output)
├─ tool catalog snapshot (fixed + skill + MCP + extended, frozen for the turn)
├─ personalization / skill / memory effective views (existing snapshots, referenced)
├─ permission policy revision observed at start
└─ generation options (reasoning depth, streaming, compaction mode)
```

Freezing beats hot reload because a turn that changes rules midway is unexplainable and untestable; the trade — edits apply next turn — is the same rule the personalization snapshot and the compaction control state already follow. The snapshot also collapses the `too_many_arguments` orchestration surface: pipeline stages take the snapshot, not nine ports.

## Model-aware context budget

**Current**: the compaction trigger already resolves real capacity via the model context catalog, but the context-engine evidence budget is a fixed `total: 32_768` with hardcoded reserves (`generation.rs`). Evidence selection therefore under-uses large-context models and over-promises on small ones.

**Target**: derive `ContextBudget` from the snapshot's resolved capacity with a source priority of explicit profile override → model catalog → conservative default, keeping proportional reserves rather than fixed numbers, and keeping the character-only fallback for models without capacity metadata. The budget lands inside the generation snapshot so a capacity lookup failure is visible at turn start, not partway through evidence packing.

## The typed evidence envelope

**Current**: selected evidence is appended to `request.effective_prompt` inside `<context-evidence>` tags — untrusted retrieved content rides inside the user's own message.

**Target**: evidence travels as a typed envelope, rendered per interface format as a clearly-attributed non-user section (its own content part or system-side block), carrying provenance (source kind, id, redaction state) and never concatenated into user text. The injection boundary rule: retrieved content may inform the model, but nothing untrusted may masquerade as the user's words.

## The provider-neutral synthetic-summary carrier

**Current**: both compaction paths insert the summary as `role: "user"`; the optimizer path prefixes an identifying marker, the compatibility path does not.

**Target**: one `ContextSummary` carrier emitted by both paths, mapped per interface format (system, developer, or a dedicated content part — the mapping is an open decision below), always machine-identifiable and never bare user speech. Compatibility: transcripts already containing marker-style summaries keep rendering.

## The durable tool-call journal

**Current**: no persisted effect log for tool executions; crash recovery cannot distinguish "never ran" from "ran, outcome unknown", so a replay risks repeating side effects. Spans are observational, not a recovery contract.

**Target**:

```text
ToolCallJournal entry
├─ journal_id, generation_id, round, tool name, input digest
├─ effect class (read-only / effectful / external)
├─ state: Intent → Executing → Completed | Failed | UnknownEffect
└─ outcome digest + timestamps
```

Journal-before-execute for effectful tools; read-only tools may batch or skip journaling by class. Recovery never auto-replays an `UnknownEffect` entry — it surfaces the entry for review (the same conservatism session recovery already applies to uncertain CLI side effects). Idempotency: a journal entry's identity is stable across restarts, so duplicate delivery of one terminal event cannot double-execute.

## Open decisions (to settle in review, tracked here)

1. `ContextSummary` mapping per interface format: system vs developer vs dedicated content part.
2. Tool-catalog schema budget and the trimming algorithm when the frozen catalog exceeds it.
3. Whether approvals persist universally or only for effectful tools (the study argues universally).
4. Capacity source conflicts: catalog vs live discovery vs profile override precedence and cache invalidation.
5. Partial assistant messages after a provider stream break: display and manual-continue semantics.

## Non-goals

No second active generation profile, no per-purpose profiles (internal tasks keep the active profile with a distinct usage purpose), no parallel tool execution model, no artifact store, no runtime-subject permission expansion, and no attempt to make the CLI runtime share this kernel — `launch_kind = api` and `launch_kind = cli` keep separate executors under the shared governance contracts.
