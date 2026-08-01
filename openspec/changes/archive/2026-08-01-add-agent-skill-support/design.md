## Context

`skill-management` (`src-tauri/src/contexts/tooling/skills/`) already owns Skill content: a `skills` table keyed by `(id, scope, workspace_path)`, `SkillMetadata{id, name, description, category, version, triggers}`, 6 built-ins, and a `skill_agent_bindings` table that binds a Skill to an agent id **for CLI filesystem mounting** — that table carries `mounted_path`/`status` columns that only make sense for a CLI process reading its own skill directory. `agent_id` is already a plain, unconstrained `String` there (checked against the real `agents` table's registered ids, not a fixed enum), and CLI + API agents already share that same `agents` table (confirmed in Phase 1, `add-custom-agent-registration`).

The native API-based agent (`agent_runtime`) has no "system prompt" concept anywhere today — `GenerationProcessRequest.effective_prompt` is one flat string, and neither `anthropic_provider.rs` nor `openai_compatible_provider.rs` builds anything beyond a plain `messages`/turns array. `RuntimeAgentApiAdapter::execute()` (`api_process_adapter.rs`) already has two established extension precedents from Phases 2-3: a `WireFormat` function-pointer struct that both provider modules populate identically, and a per-round-trip loop (`turns: Vec<Value>`) that Phase 3's compaction reads/rewrites by character count.

## Goals / Non-Goals

**Goals:**
- Let a registered API agent bind to existing Skills (the same registry `skill-management` already manages) and have bound + enabled Skills' content injected as a provider-native system prompt on every generation request for that agent.
- Keep exactly one "Skill" concept in the product — no parallel content model.
- Add the minimum new schema/ports needed: one new binding table, one new cross-context port, no changes to `skill-management`'s existing schema or requirements.
- Guarantee the injected system prompt is never treated as a compactable/summarizable "turn" by Phase 3's compaction mechanism.

**Non-Goals:**
- Tool-based progressive Skill loading (a `read_skill`/`list_skills` tool) — explicitly rejected this phase in favor of simple system-prompt-prefix injection (see Decision 2); Skills here are short prompt-sized text, so the extra round-trip/complexity isn't justified yet.
- Per-Skill ordering/staging/versioning controls — `prompt-hook-management`'s governance (categories, draft/publish/rollback, execution trace/evaluation) is explicitly not being extended to Skills or to API agents this phase.
- Any change to CLI agents' existing mount-path Skill binding, drift detection, or built-in seed management.
- A separate Skill authoring surface for API agents — they bind to Skills authored through the existing `skills-page.tsx` UI.

## Decisions

### 1. New binding table lives in `tooling::skills`, not `agent_runtime`

`skill_api_agent_bindings(skill_id, scope, workspace_path, agent_id, enabled, created_at, updated_at)`, `PRIMARY KEY (skill_id, scope, workspace_path, agent_id)`, `FOREIGN KEY (skill_id, scope, workspace_path) REFERENCES skills(id, scope, workspace_path) ON DELETE CASCADE`, `FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE`. No `mounted_path`/`status` columns — presence of a row means "bound"; `enabled` (matching `skills.enabled`'s own on/off precedent) lets a user disable a binding without deleting it. `agent_runtime` depends on this data through a new port, not a direct table reference — matching how `agent_runtime` already depends on `ConversationHistoryPort`/`ApiCredentialPort`-style ports for everything outside its own context.

**Why:** the existing `skill_agent_bindings` table already lives inside `tooling::skills`, not wherever CLI-agent launching lives — bindings are Skill-context-owned data regardless of which kind of agent is on the other side. Keeping the new table there too is consistent, and keeps `agent_runtime` from needing to know anything about Skill scopes/metadata beyond "give me the enabled system-prompt text for this agent id."

**New port** (`agent_runtime::application::ports`): `AgentSkillPort::bound_skill_prompts(&self, agent_id: &str) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError>` where `BoundSkillPrompt{name: String, body: String}`, ordered by skill id for determinism (no user-configurable ordering this phase — see Non-Goals). Implemented by a new adapter in `tooling::skills::infrastructure` that joins the new binding table (`enabled = 1`) against `skills`, and is wired into `agent_runtime`'s ports the same way every other cross-context dependency already is in `bootstrap/agent_runtime.rs`.

**Alternative considered:** put the binding table in `agent_runtime` instead, with `agent_runtime` reading Skill content directly from `tooling::skills`'s repository. Rejected — this would make `agent_runtime` own a piece of "what Skills apply to what" data while `tooling::skills` owns the rest of it, splitting one concept across two contexts for no benefit.

### 2. Injection: system-prompt-prefix, assembled once per generation, threaded as a value separate from `turns`

`execute()` calls `agent_skill_port.bound_skill_prompts(agent_id)` once, near the top (alongside resolving `provider_config`/`wire_format`), and formats bound skills into one string:
```
## <skill 1 name>
<skill 1 body>

## <skill 2 name>
<skill 2 body>
```
(empty list → `None`, no system prompt at all — a Skill-less API agent behaves exactly as it does today). This `Option<String>` is passed alongside `turns`/`tools` into `(wire_format.build_request_body)`, whose signature gains a fourth parameter: `fn(&str, &[Value], &[ToolDefinition], Option<&str>) -> Value`.

- **Anthropic** (`anthropic_provider.rs`): `Some(system)` sets a top-level `body["system"] = json!(system)` string field — Anthropic's Messages API keeps `system` separate from `messages` natively, so this requires no restructuring of `turns`.
- **OpenAI-compatible** (`openai_compatible_provider.rs`): `Some(system)` prepends `{"role": "system", "content": system}` as the first element **when constructing `body["messages"]` from `messages: &[Value]`** — i.e. synthesized at request-build time, not stored back into the `turns: Vec<Value>` the caller owns.

**Why the system prompt is never written into `turns`:** Phase 3's compaction (`maybe_compact`/`turns_character_count`) operates on `execute()`'s local `turns: Vec<Value>`, treating everything in it as fair game to summarize away once the recent-window boundary is crossed. If the system prompt were stored as `turns[0]` (the natural-looking approach for OpenAI-compatible, whose wire format has no separate system field), compaction could summarize it into a lossy "conversation recap" and it would stop being reliably present on every request — exactly the failure mode a system prompt must never have. Keeping `system: Option<String>` as its own value, computed once and passed unchanged into every `build_request_body` call for the lifetime of the generation (including the two `maybe_compact`-triggered summarization calls), guarantees it survives compaction untouched and is never mistaken for a compactable turn.

**Alternative considered:** tool-based progressive loading (`read_skill` tool, Non-Goals). Rejected this phase — built-in Skills are 1-3 sentence instructions and nothing in the registry enforces a size limit on user-authored ones either, so today's realistic Skill sizes don't justify the extra round-trip and reliability risk (the model must choose to call the tool) that progressive loading would add.

### 3. Skill-fetch failure degrades gracefully, same philosophy as Phase 3's summarization fallback

If `bound_skill_prompts` errors (DB unavailable, etc.), `execute()` logs via the existing `AgentLoggingPort` and proceeds with `system = None` rather than failing the generation.

**Why:** consistent with Phase 3's established precedent (a failed summarization call falls back to sending uncompacted rather than breaking the generation) — Skills are an enhancement to a request, not a precondition for it to be valid.

### 4. Frontend: extend the existing Skill settings UI and service boundary, not a new page

`src/settings/pages/skills-page.tsx` already renders CLI-agent mount bindings via `agent-service.ts`'s `bindSkillToAgent`/`unbindSkillFromAgent`. This phase adds parallel methods (`bindSkillToApiAgent`/`unbindSkillFromApiAgent`/`listApiAgentSkillBindings` or equivalent naming resolved at implementation time) to the same service interface, `tauri-agent-client.ts`, and `web-agent-client.ts`, and a new section/control on the same page listing registered API agents a Skill can additionally bind to — no new settings page, no new component tree.

**Why:** matches the "one Skill concept, two binding paths" framing from the proposal; a separate page would visually fragment something that's conceptually the same Skill list with a second kind of target.

## Risks / Trade-offs

- **[Risk]** Concatenating multiple bound Skills' full bodies into every request adds fixed token cost to every turn of a session, whether or not the Skill is relevant to what the user is currently asking. → **Mitigation:** Skills are opt-in per agent (a user only binds what they want always-available); Non-Goal for now, revisit progressive loading if adopted Skills grow larger or more numerous in practice.
- **[Risk]** A system prompt with no explicit ordering/versioning could drift confusingly if a user rebinds Skills mid-session (does the *next* request reflect the change, given `system` is recomputed once per generation, not cached across generations?). → **Mitigation:** by design, `bound_skill_prompts` is called fresh at the start of every `execute()` call (once per generation, i.e. once per user turn) — a rebind takes effect on the very next message, which is the expected, unsurprising behavior; nothing is cached across turns.
- **[Trade-off]** No per-Skill ordering control (Decision 1, ordered by skill id) — acceptable for a first version; revisit only if users need to control which Skill's instructions "win" in case of conflicting guidance.

## Migration Plan

Purely additive: one new SQLite table (`skill_api_agent_bindings`) in `tooling::skills`'s existing migration sequence, one new port + adapter, a fourth parameter on an existing function-pointer type in `WireFormat`. No changes to existing tables or existing requirements. API agents with no bound Skills, and all CLI agents, are completely unaffected — identical behavior to today.

## Open Questions

- Exact frontend copy/placement for the new API-agent binding control on `skills-page.tsx` — a UI detail for implementation time, not an architectural decision.
- Exact naming of the new service-boundary methods (`bindSkillToApiAgent` vs. a more generic name that could later cover other non-mount binding targets) — pick the clearer name at implementation time; no forward-compatibility requirement being designed for here.
