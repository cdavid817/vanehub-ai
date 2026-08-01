## Why

The native API-based agent (Phases 1-4: `add-custom-agent-registration`, `add-agent-tool-execution`, `add-agent-context-compaction`, `add-agent-skill-support`) has no memory beyond a single session — even Phase 4's Skill-driven system prompt is static, bound content, not something the agent or user can update from a conversation. Working with the same agent on the same project across many separate sessions means re-explaining context every time; nothing the agent learns in one session carries into the next.

## What Changes

- Add an explicit, tool-driven memory path: a new `remember` tool the model can call (e.g. when the user says "remember that...") to save a fact or decision to persistent storage scoped to the agent and its workspace folder.
- Add an automatic memory-extraction path: when a generation's turns cross Phase 3's existing context-compaction threshold, an additional best-effort model call extracts memorable facts from the turns being compacted away and saves them to the same store — reusing the compaction trigger as a "this session got substantial" signal rather than running extraction on every single turn.
- Add memory injection: stored memories for the current agent + workspace folder are folded into the same system-prompt mechanism Phase 4 built for Skills, bounded by a character budget — and, like Skill content, never written into the turns list Phase 3's compaction manipulates.
- Add a minimal memory management view: list and delete stored memories per agent, so users can correct or purge what's been remembered.
- Web/mock parity: deterministic simulated `remember` tool calls, extraction signal, and injection for mock sessions.

## Capabilities

### New Capabilities
- `agent-cross-session-memory`: persistent, agent + workspace-scoped memory for the native API-based agent — explicit tool-driven saves, automatic best-effort extraction on compaction, system-prompt injection, and a management view.

### Modified Capabilities
- None. `api-agent-runtime`, `agent-tool-execution`, `agent-context-compaction`, and `agent-skill-injection` (from Phases 1-4) are extended in behavior (a new tool in the catalog, an additional system-prompt source, an additional best-effort call alongside compaction) but none has an archived baseline yet, so this change describes the new behavior entirely within the new capability rather than as deltas against not-yet-merged specs.

## Impact

- **`agent_runtime` (Rust, primary)**: a new SQLite table owned by `agent_runtime` itself (unlike Phase 4's Skill bindings, no other context owns this concept, so no new cross-context port is needed for storage); a new `remember` tool added to the existing tool catalog/dispatch/risk-tier machinery from Phase 2; a new best-effort extraction call reusing Phase 3's summarization-call machinery, triggered at the same point compaction already triggers; `resolve_system_prompt` (Phase 4) extended to also fold in stored memories.
- **Frontend**: new Tauri commands for listing and deleting memories; a small settings view; `web-agent-client.ts` gains deterministic mock behavior for all of the above.
- **Unaffected**: CLI agents, cross-*agent* memory sharing (out of scope — memories stay scoped to the single agent that produced them), vector search/embeddings over memories (deferred — start with a simple bounded inclusion strategy, revisit only if that proves inadequate).
- No breaking changes: purely additive — an agent with no bound workspace folder and no saved memories behaves exactly as it does today.
