## Why

CLI agents can already bind reusable Skills (the existing `skill-management` capability: `SKILL.md`-backed content, 6 built-ins, filesystem mount into each CLI's own skill-discovery directory). The native API-based agent added in Phases 1-3 (`add-custom-agent-registration`, `add-agent-tool-execution`, `add-agent-context-compaction`) has no equivalent — it isn't a CLI process with its own filesystem-based skill convention, so the existing mount-path binding mechanism doesn't apply, and today there is no way to give an API agent reusable Skill/prompt-preset content at all.

## What Changes

- Add a new, non-mount binding mechanism: registered API agents can bind to the **same Skills** `skill-management` already manages (same `skills` table/registry, same 6 built-ins, same user-authored/imported Skills) via a new binding table that carries no filesystem `mounted_path`/`status` — those columns are meaningless for an agent that isn't a CLI process.
- When a generation runs for an API agent with bound + enabled Skills, their content is concatenated into the request's **system prompt**: Anthropic's top-level `system` field, OpenAI-compatible's leading `{role: "system"}` message. This is a new concept in the `agent_runtime` wire-format machinery — no "system prompt" exists there today (`GenerationProcessRequest.effective_prompt` is one flat string with no system/user split).
- Frontend: extend the existing Skill settings UI so a user can bind/unbind Skills to a registered API agent, alongside (not replacing) the existing CLI mount-path binding UI.
- Web/mock parity: `web-agent-client.ts` gets deterministic bind/unbind behavior and a visible signal that bound Skill content influenced the mock response.
- Does **not** change `skill-management`'s existing CLI mount-path binding, drift detection, built-in seed management, or import/authoring behavior — all of that keeps working exactly as today, unmodified.

## Capabilities

### New Capabilities
- `agent-skill-injection`: binding existing Skills to registered API agents via a non-mount binding table, and injecting bound + enabled Skill content into that agent's generation requests as a provider-native system prompt.

### Modified Capabilities
- None. `skill-management`'s existing requirements (scopes, metadata, built-ins, mount-path binding, import, drift detection/sync, service boundary) are unchanged — this change adds a new consumer of the same underlying Skill registry without altering any existing Skill requirement's behavior. `api-agent-runtime`/`agent-tool-execution`/`agent-context-compaction` (from `add-custom-agent-registration`, `add-agent-tool-execution`, `add-agent-context-compaction`) are extended (requests can now carry a system prompt) but have no archived baseline yet, so this is described entirely within the new capability rather than as a delta against not-yet-merged specs.

## Impact

- **`agent_runtime` (Rust)**: new SQLite table binding `(skill_id, scope, workspace_path, agent_id, enabled)`; new application service methods to bind/unbind/list Skills bound to an agent id; `WireFormat`/`execute()` in `api_process_adapter.rs` gains a system-prompt concept threaded through `build_request_body` for both `anthropic_provider.rs` and `openai_compatible_provider.rs`.
- **`tooling::skills` (Rust)**: read-only dependency — the new binding table's foreign key points at `skills(id, scope, workspace_path)`, and the new service reads Skill content through the existing repository. No changes to `skill-management`'s own domain, schema, or requirements.
- **Frontend**: extends the Skill settings UI and `agent-service.ts` boundary with bind/unbind-to-API-agent methods; `web-agent-client.ts` mirrors the behavior for parity.
- **Unaffected**: CLI agents and `skill-management`'s existing mount-path flow, `prompt-hook-management` (untouched, CLI-only), Phase 2's tool-use loop, and Phase 3's compaction mechanism — the system prompt is deliberately threaded as a value separate from the `turns` list compaction measures and rewrites (see design.md Decision 2), specifically so compaction can never summarize it away.
- No breaking changes: purely additive — API agents with no bound Skills behave exactly as they do today.
