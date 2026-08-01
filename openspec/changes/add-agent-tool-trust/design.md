## Context

**The mandatory approval gate today.** `risk_tier_for(tool_name, input) -> ToolRiskTier` (`agent_runtime::application::tool_catalog.rs`) is a pure, static classification: `SHELL_TOOL_NAME` always `RequiresApproval`; `FILE_TOOL_NAME` is `AutoApprove` only for `operation: "read"`, `RequiresApproval` for everything else (including `"write"`); `REMEMBER_TOOL_NAME` always `AutoApprove`; every other name (including MCP-prefixed, `add-agent-mcp-tools`) falls through to `RequiresApproval` by the catch-all default. `execute()`'s tool round-trip loop (`infrastructure/api_process_adapter.rs`) calls this once per requested tool call, and only proceeds straight to `execute_tool_call` without prompting when the result is `AutoApprove` — there is no other lever anywhere in this call graph.

**CLI agents already have the lever this change adds for API agents.** `RuntimeAgentCliProfileAdapter::load` (`infrastructure/cli_profile.rs:22-54`) resolves a CLI session's actual launch parameters by: (1) `self.parameters.load_selections(agent_id)` — that agent's own **persisted** CLI Parameter defaults, configured once via the CLI Parameters settings UI, independent of any single session; (2) `apply_configuration_overrides(agent_id, selections, configuration)` — the **per-turn** chat-configuration choice (`permission_mode: "plan"/"agent"/"auto"`) layered on top, only inserting an override when the per-turn value is one of those three literals. Confirmed by reading `providers/invocation.rs:193-278` directly: `permission_mode == "default"` matches none of the match arms, so nothing is inserted and step (1)'s persisted value survives untouched. `agent_runtime::application::service.rs:108-118`'s `start_loop_role_generation` (used by every Loop worker/verifier role generation) deliberately sets `permission_mode: "default".to_string()` — not `"plan"`, not `"agent"`/`"auto"` — meaning Loop does not itself grant any special permission; it simply defers to whatever persisted CLI Profile posture the assigned agent already has. This is the exact mechanism native API agents have no equivalent of today.

**Where a new setting would need to be read.** `execute()` already calls `config.provider_config(agent_id)` (`ApiAgentGateway::provider_config`, `ports.rs:628`) once per generation to read `ApiProviderConfig { model_id: String, interface_format: String, base_url: Option<String> }` (`application/models.rs:780-784`) — the established, already-wired read path for "this API agent's own persisted settings."

**Schema precedent.** `agents.model_id`/`interface_format`/`base_url` were each added by their own additive `ALTER TABLE agents ADD COLUMN ...` migration in `agent_runtime::infrastructure::schema.rs` (`apply_api_agent_schema`, `apply_openai_compatible_schema`), registered in `platform::database::migrations.rs`'s versioned list — the highest version currently applied is **31** (`"agent-cross-session-memory"`, line 175-180). Boolean-shaped columns elsewhere in this schema are consistently `INTEGER NOT NULL DEFAULT <0|1>` (`skills.enabled`, `mcp` servers' `active` — SQLite has no native boolean type).

**Plan mode's enforcement is already unconditional and independent of any approval decision.** `add-agent-chat-configuration`'s `execute_tool_call` gates `shell`/MCP-prefixed names/file-`write` operations with a hard rejection *whenever `plan_mode` is true*, before any tool-specific dispatch — this check does not consult `risk_tier_for` or any approval outcome at all; it runs regardless of whether the call would otherwise have been auto-approved or human-approved. This means plan mode already wins over any auto-approve setting **by construction**, not because of anything this change needs to add.

**Lifecycle-management precedent for API-agent-only mutations.** `update_api_agent`/`delete_api_agent` (`add-agent-lifecycle-management`) both use `UPDATE/DELETE ... WHERE id = ?1 AND launch_kind = 'api'`, treating "0 rows changed" as `AgentNotFound` — deliberately not distinguishing "no such id" from "id belongs to a non-API agent," since CLI agents were never meant to reach these operations at all.

## Goals / Non-Goals

**Goals:**
- A user can grant a specific native API agent a persistent exemption from per-call approval for `shell` and file `write` calls, in a way that applies to every future session with that agent (including ones nobody is watching).
- Granting this exemption requires a deliberate, explicit, clearly-worded confirmation — distinct from ordinary agent editing.
- MCP-sourced tool calls and plan mode are both completely unaffected by this setting, in both directions (the setting can never loosen either, and neither one ever needs to loosen the setting).

**Non-Goals:**
- Any change to MCP tool call approval — stays unconditional `RequiresApproval`, no carve-out, this phase or any planned future one.
- Any change to plan mode's own enforcement.
- Per-project/per-folder scoping of the trust grant — matches CLI Profile's own agent-global scope exactly (see Decision 1).
- Any change to CLI-based agents' own permission mechanism (`cli_profile.rs`, `providers/invocation.rs`) — untouched; they already have this capability.
- Opening Loop to API-agent worker/verifier roles — this change only removes the practical blocker (every tool call needing a click); the Loop-specific `InteractionMode::Cli` gates are out of scope here and tracked as a distinct follow-up change.

## Decisions

### 1. The trust grant is scoped per-agent, globally — not per-project/folder

A single `agents.auto_approve_tools` column, read the same way regardless of which project/session the agent is currently working in.

**Why:** deliberately mirrors CLI Profile's own scope exactly — a user's persisted CLI Parameter selections for an agent already apply to every project that agent touches, not per-project. Matching that precedent keeps the two mechanisms conceptually identical (same question — "do I trust this agent generally?" — answered the same way for both agent kinds), and per-folder scoping can be added later as a refinement if a real need for it shows up, without this phase needing to guess at that shape now.

**Alternative considered:** scope to `(agent_id, folder)`, matching how memories/Skill bindings are already scoped. Rejected for this phase — it answers a different, narrower question ("do I trust this agent *for this project*?") than what was asked for, and CLI Profile — the mechanism this is explicitly modeled on — doesn't do this either.

### 2. Coverage is `shell` and `file`-`write` only; MCP stays unconditionally gated; the check lives in a new function alongside `risk_tier_for`, not inside it

New function (name illustrative), e.g. `fn requires_approval(tool_name: &str, input: &Value, auto_approve_tools: bool) -> bool`, used at `execute()`'s round-trip-loop approval-gate call site in place of the current direct `risk_tier_for(...) == RequiresApproval` comparison:

```rust
fn requires_approval(tool_name: &str, input: &Value, auto_approve_tools: bool) -> bool {
    if auto_approve_tools && (tool_name == SHELL_TOOL_NAME || tool_name == FILE_TOOL_NAME) {
        return false;
    }
    risk_tier_for(tool_name, input) == ToolRiskTier::RequiresApproval
}
```

`risk_tier_for` itself is untouched — it keeps its existing pure, agent-trust-unaware contract and existing test suite exactly as-is.

**Why:** MCP tools are arbitrary and third-party-defined; VaneHub has no way to evaluate what any given one actually does, which is exactly why `add-agent-mcp-tools` classified them `RequiresApproval` unconditionally with "no auto-approve carve-out" stated explicitly in that phase's own proposal. Extending trust to a category this codebase has already deliberately refused to loosen once would contradict that decision without a dedicated conversation about MCP specifically — matching this change's own proposal.md, which names this exclusion as a deliberate scope boundary, not a deferred TODO. Keeping the composition in a separate function (rather than adding an `auto_approve_tools` parameter to `risk_tier_for` itself) keeps `risk_tier_for`'s "pure classification by name+input, no ambient state" contract intact, and its existing unit tests need no changes.

**Alternative considered:** also cover MCP calls, gated by a second, separate flag. Rejected for this phase — no clear need has surfaced yet, and it would need its own security conversation given MCP tools' arbitrary/unknown nature; easy to add later as a distinct, explicitly-named setting if it does.

### 3. Plan mode needs no new interaction code — its existing unconditional gate already wins

No changes to `execute_tool_call`'s plan-mode checks. `requires_approval`'s `auto_approve_tools` short-circuit only ever affects whether a human is *prompted*; `execute_tool_call`'s plan-mode rejection is a completely separate, unconditional check that runs regardless of the approval outcome (auto-approved, human-approved, or denied all funnel into the same `execute_tool_call` call, which still hard-rejects `shell`/file-`write` in plan mode either way).

**Why:** worth stating explicitly as a decision (not just an incidental consequence) because it's the specific safety invariant this whole exploration was checking for — plan mode must always win over a trust grant, and it does, by construction of how the two checks are independently wired into two different points in the call graph rather than one depending on the other.

### 4. Enabling requires a dedicated confirmation action; disabling does not; both use a separate command from `update_api_agent`

New Tauri command (e.g. `set_agent_tool_trust(agent_id, enabled: bool) -> AgentView`), backed by `AgentRuntimeApplicationService::set_auto_approve_tools`, using the identical `UPDATE agents SET auto_approve_tools = ?1 WHERE id = ?2 AND launch_kind = 'api'` / 0-rows-changed-is-`AgentNotFound` shape `update_api_agent`/`delete_api_agent` already established. The current value is exposed by extending the *existing* `ApiProviderConfig`/`get_api_agent_provider_config` read path with one more field — no new read command needed. On the frontend, enabling calls `window.confirm(...)` with explicit warning copy (e.g. "This agent will be able to run shell commands and modify files without asking each time, in every session.") before calling the command at all; disabling calls it directly, no confirmation — it only restores the stricter default.

**Why:** `window.confirm`-gated is this codebase's own established pattern for consequential-but-not-catastrophic actions (agent deletion already uses it, per `add-agent-lifecycle-management`), so this doesn't need a new modal component — it needs the SAME weight of confirmation deletion already gets, with wording specific to what's being granted. Keeping this as its own command (not folded into `update_api_agent`, the way key rotation was) reflects that this is a categorically different kind of edit — `update_api_agent`'s existing fields (display name, model, base URL, key) are corrections/preferences; this one is a security-relevant grant, and deserves a call site that can't be casually reached as a side effect of an unrelated field edit.

**Alternative considered:** a plain checkbox inside `AgentEditDialog`, saved via the existing `update_api_agent` call. Rejected — exactly the "casual dropdown" shape the standing decision from `add-agent-chat-configuration` (not implementing `permission_mode: "agent"/"auto"`) was worried about; a dedicated, separately-confirmed action is the resolution to that earlier concern, not a repeat of it.

## Risks / Trade-offs

- **[Risk]** A trusted agent's `shell`/file-write calls now execute without any human in the loop at all for that specific call — the tool's own existing safeguards (sandboxed path resolution for `file`, bounded/cancellable subprocess execution for `shell`) are the only remaining backstop. **Mitigation:** this is an explicit, informed trade-off the user opts into per-agent with clear warning copy, not a default; the underlying execution primitives (`platform::process`, `platform::filesystem`) are unchanged and already bounded regardless of approval status.
- **[Risk]** A user might expect "trust this agent" to also loosen MCP approval, since MCP tools feel similar to shell/file in casual use. **Mitigation:** UI copy should name exactly what's covered ("shell commands and file changes") rather than a vague "tool calls," so the MCP exclusion is legible at grant time, not just in documentation.
- **[Trade-off]** Global per-agent scope (Decision 1) means a user can't trust an agent for one project but not another without registering two separate agent entries. **Mitigation:** matches CLI Profile's existing limitation exactly; revisit only if this friction proves real in practice, consistent with this native-agent effort's established "start simple" pattern.

## Migration Plan

Purely additive: one new schema migration (version 32) adding `agents.auto_approve_tools INTEGER NOT NULL DEFAULT 0`, one new field on `ApiProviderConfig` (and its DTO), one new composing function (`requires_approval`) used at exactly one call site in place of the current direct `risk_tier_for` comparison, one new application-service method + Tauri command + frontend method, one small UI addition. No changes to `risk_tier_for`, `execute_tool_call`'s existing dispatch or plan-mode gates, `update_api_agent`, or any MCP-related code path.
