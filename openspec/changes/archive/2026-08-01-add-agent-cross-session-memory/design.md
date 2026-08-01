## Context

Nothing in this codebase persists information across separate sessions today. `ConversationHistoryPort::recent_messages` is scoped to one session's own message table; Phase 3's compaction only affects what one generation sends, not what survives after it; Phase 4's Skill-driven system prompt is static, bound content — the agent and user have no way to write to it from a conversation.

The only field available to group sessions together over time is `AgentSession.folder: Option<String>` (populated from the `sessions` context's `SessionWorkspace.folder`) — an unconstrained, non-unique string. No "Project" entity exists anywhere to attach to instead. `folder` can be `None` for non-project or remote sessions.

Phase 3 (`add-agent-context-compaction`) already established the precedent for a best-effort, non-streamed extra model call (`summarize_turns`, reusing `wire_format.build_request_body`/`translate_sse_data` without declaring tools), triggered when a generation's turns cross `COMPACTION_TRIGGER_CHARACTERS`, operating on exactly the older-turns slice (`&turns[..split_at]`) about to be replaced. Phase 4 (`add-agent-skill-support`) established the precedent for resolving content once per generation into a `system: Option<String>` (`resolve_system_prompt`/`format_system_prompt`) that is threaded into every request but deliberately never written into `turns`, so compaction can never summarize it away.

## Goals / Non-Goals

**Goals:**
- Let an agent's conversations build up persistent memory, scoped to that agent and (when available) its workspace folder, that carries into later, separate sessions.
- Two ways memory gets written: an explicit `remember` tool call, and best-effort automatic extraction riding on Phase 3's existing compaction trigger — both writing into the same store.
- Inject stored memory into generation requests the same way Phase 4 injects Skills: as part of the system prompt, bounded, never written into `turns`.
- Give users a way to see and delete what's been remembered.
- Preserve Web/mock parity.

**Non-Goals:**
- Cross-*agent* memory sharing — memories stay scoped to the single agent that produced them; a different agent (even on the same folder) does not see them.
- Vector search, embeddings, or semantic relevance ranking over memories — start with recency-ordered inclusion under a character budget (mirroring `COMPACTION_TRIGGER_CHARACTERS`'s precedent of a simple, non-semantic heuristic); revisit only if that proves inadequate in practice.
- Any change to CLI-based agents.
- Automatic extraction on every single generation — only when compaction already triggers (see Decision 3).

## Decisions

### 1. Scoping key: `(agent_id, folder)`, with an empty-string sentinel for "no folder"

New table `agent_memories(id, agent_id, folder, content, source, created_at, updated_at)`, `folder TEXT NOT NULL DEFAULT ''`. A session with `AgentSession.folder: None` reads/writes memory under `folder = ''` — an "agent-global, not project-specific" bucket, not an error and not a reason to disable memory.

**Why an empty-string sentinel instead of a nullable column:** matches the exact convention `skill-management`'s own schema already uses (`skills.workspace_path TEXT NOT NULL DEFAULT ''` for its global scope) and avoids `NULL`-vs-`NULL` SQL comparison pitfalls (`WHERE folder = ?` works uniformly; a nullable column would need `WHERE folder IS ? OR (folder IS NULL AND ? IS NULL)`-style handling).

### 2. Storage lives in `agent_runtime` itself — one port, one adapter, two consumers

Unlike Phase 4's Skill bindings (which had to live in `tooling::skills`, a different context, requiring a new cross-context port), memory is a concept `agent_runtime` owns outright. One new trait, `AgentMemoryPort` (`save`, `list`, `delete`), implemented by one new `SqliteAgentMemoryRepository` in `agent_runtime::infrastructure`, constructed once in bootstrap and shared via `Arc` clone into two consumers:
- `RuntimeAgentApiAdapter` (generation execution): `save` (from the `remember` tool and automatic extraction), `list` (for system-prompt injection).
- New Tauri commands on `AgentRuntimeApi`'s existing facade (management UI): `list`, `delete`.

**Why one port instead of splitting like Phase 4 did:** Phase 4 split `SkillApiBindingRepository` out from `SkillRepository` specifically to avoid bloating an already-large *existing* 10-method trait with an unrelated concern. `AgentMemoryPort` is built fresh with exactly the operations memory needs — there's no existing trait to protect from bloat, so one small, cohesive trait is simpler than three narrower ones.

### 3. Automatic extraction piggybacks on the existing compaction trigger — no new trigger logic

`maybe_compact` (Phase 3) already identifies `&turns[..split_at]` as "the older turns about to be replaced" when it fires. Extraction runs on exactly that slice, at exactly that moment — a second best-effort model call alongside the existing summarization call, not a new heuristic for "when is a session substantial enough to extract from." The extraction instruction explicitly allows "nothing worth remembering" as a valid, cheap, empty response; each non-empty returned line becomes one `agent_memories` row with `source = "automatic"`. Extraction failure is logged and otherwise ignored — it must never affect compaction's own success/failure, matching Phase 3's summarization-failure fallback philosophy exactly.

**Why not extract after every generation:** the natural alternative — running an extraction call at the end of every single `execute()` — adds latency and cost to every short exchange, most of which contain nothing worth remembering long-term. Tying extraction to the same signal that already means "this session has gotten substantial" is a cost/latency control that reuses existing infrastructure instead of inventing a new one.

### 4. Explicit path: a `remember` tool, auto-approved

`tool_catalog()` (Phase 2) gains a `remember` tool (`{content: string}` input). `risk_tier_for` classifies it `AutoApprove`. `execute_tool_call` dispatches it to `AgentMemoryPort::save` using the executing generation's `agent_id`/`folder`, returning a short confirmation string as the tool's output.

**Why `AutoApprove`:** unlike shell execution or file writes, this tool only ever writes to the app's own internal SQLite storage — it cannot touch the user's filesystem, run a process, or affect anything outside the app. The worst case is a wrong or low-value memory, which the management view (Decision 6) lets the user delete.

### 5. Injection: `resolve_system_prompt` grows a second source, still never written into `turns`

`resolve_system_prompt` (Phase 4) is extended to also accept an `AgentMemoryPort`, fetch memories for `(agent_id, folder)`, and fold a `## Memory` section (recency-ordered, greedily included up to `MEMORY_INJECTION_CHARACTER_BUDGET`) alongside the existing `## <Skill name>` sections into the same `system: Option<String>` — returning `None` only when both sources are empty. The combined value is threaded into every `build_request_body` call exactly as Skill content already is, and is never appended to `turns`, for the identical reason established in Phase 4 (compaction must never be able to summarize it away).

### 6. Minimal management view: list + delete

A small settings view lists an agent's stored memories (content, source, created date) with a delete action per row, backed by the two new Tauri commands from Decision 2. No editing, no manual "add memory" affordance this phase — creation is exclusively through the `remember` tool and automatic extraction; deletion is the only manual control needed for correctness/privacy.

## Risks / Trade-offs

- **[Risk]** Character-budget-bounded, recency-ordered injection can silently drop older memories once the budget fills, with no ranking by relevance. → **Mitigation:** matches the same simple-heuristic trade-off already accepted for compaction and Skill injection in this codebase; revisit only if this proves inadequate in practice.
- **[Risk]** Automatic extraction is lossy and non-deterministic (an LLM call), and only fires when compaction fires — a session that never crosses the compaction threshold gets no automatic extraction at all. → **Mitigation:** the explicit `remember` tool remains available regardless of session length; automatic extraction is an additive convenience, not the only way to save a memory.
- **[Risk]** A wrong or stale memory persists indefinitely and silently influences every future session until removed. → **Mitigation:** the management view (Decision 6) is in scope for this phase specifically because of this risk, not deferred.
- **[Trade-off]** No cross-agent sharing (Non-Goals) — acceptable; revisit only if a real use case for it emerges.

## Migration Plan

Purely additive: one new table, one new port/adapter fully owned by `agent_runtime`, one new tool in the existing catalog, an extension to an existing system-prompt-resolution function, two new Tauri commands, one small settings view. No changes to any existing table or existing requirement. Agents that never call `remember` and never cross the compaction threshold behave exactly as they do today.

## Open Questions

- Exact `MEMORY_INJECTION_CHARACTER_BUDGET` value — pick a conservative default at implementation time (comfortably smaller than `COMPACTION_TRIGGER_CHARACTERS`, since memory is meant to be a concise fact list, not a full conversation).
- Exact settings-view placement (new sub-page vs. attached to the existing agents page) — an implementation-time UI detail, not an architectural decision.
