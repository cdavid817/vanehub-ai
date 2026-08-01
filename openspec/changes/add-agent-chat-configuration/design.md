## Context

**The value is already flowing to `execute()`, just unread.** `GenerationProcessRequest.configuration: AgentChatConfiguration` (`application/models.rs:329-339`: `permission_mode: String`, `reasoning_depth: Option<String>`, `thinking: bool`, plus `agent_id`/`interaction_mode`/`provider_id`/`model_id`/`streaming`/`long_context`) is already part of every request `execute()` receives (`request: &GenerationProcessRequest`, `infrastructure/api_process_adapter.rs:345`) — no new port, no new threading through `RuntimeAgentApiAdapter`/`run_generation`/`monitor_generation` is needed to read it. The gap is entirely that nothing inside `execute()` looks at `request.configuration` at all today.

**CLI agents' own translation, read directly** (`infrastructure/providers/invocation.rs::apply_configuration_overrides`, lines 180-278): `reasoning_depth` becomes claude-code's `effort` / codex-cli's `reasoningEffort` (with `"max"` folded to `"xhigh"` for codex-cli specifically — `"max"` is a VaneHub-only tier that doesn't exist in either target CLI's own vocabulary). `permission_mode` maps `"plan"` to a read-only/no-side-effects configuration on every CLI (claude-code `permissionMode: "plan"`, codex-cli `sandbox: read-only` + `approvalPolicy: on-request`, gemini-cli `approvalMode: plan`, opencode `agent: plan`) and `"agent"`/`"auto"` to more autonomous, less-gated configurations (claude-code `acceptEdits`, codex-cli `workspace-write`, opencode `build`/`autoApprove: true`). `thinking` is only read by the opencode branch (`selections.insert("thinking", ...)`) — not universally wired even across CLI agents.

**The receive-side pipeline for thinking/reasoning content already exists in full** — confirmed by reading both provider modules directly:
- `anthropic_provider.rs::translate_content_block_delta` (lines 156-186) already matches `"thinking_delta"` → `GenerationProcessEvent::Thinking(text)`.
- `openai_compatible_provider.rs::translate_delta` (lines 135-161) already matches a `reasoning_content` delta field → the same `GenerationProcessEvent::Thinking(text)`.

Neither needs any change. Only the *request* side — actually asking the model to think, or restricting which tools are offered — is missing.

**`WireFormat.build_request_body` is a plain function pointer, not a closure**, currently `fn(&str, &[Value], &[ToolDefinition], Option<&str>) -> Value` (`api_process_adapter.rs:298`, selected per-`interface_format` at lines 319/331). Both provider modules' `build_request_body` share this exact signature today; `execute()` itself stays format-agnostic by never branching on `interface_format` directly, only ever calling through `wire_format.*` function pointers.

**Two call sites build a request body**, not one: the main generation turn (`execute()`, line 416) and context compaction's own internal summarization sub-request (`maybe_compact`, line 872) — a separate, auxiliary call that asks the model to concisely summarize the conversation so far, unrelated to the user's own turn.

**The tool catalog is already built by a single function**, `resolve_tool_catalog(request, mcp, logging, clock) -> Vec<ToolDefinition>` (added by `add-agent-mcp-tools`, `api_process_adapter.rs:626-657`), which starts from the fixed 3-tool `tool_catalog()` and merges in MCP-sourced entries. This is the natural point to swap in a different fixed catalog for plan mode.

**The tool-approval gate does not currently know about `permission_mode` at all.** `risk_tier_for(tool_name, input) -> AutoApprove | RequiresApproval` (`application/tool_catalog.rs`) is a static classification by tool name/operation only. `execute_tool_call` (`api_process_adapter.rs:1016`) is the actual execution boundary, already special-casing `remember` (no workspace-folder dependency) and MCP-prefixed names (`add-agent-mcp-tools`) before the general dispatch.

## Goals / Non-Goals

**Goals:**
- `thinking` and `reasoning_depth` produce a real behavior difference in the request sent to the provider, for the interface format each one meaningfully applies to.
- `permission_mode = "plan"` makes a native API agent's tool access genuinely read-only — both what the model is told it can do (catalog) and what it can actually get away with (execution boundary), matching every CLI agent's own "plan mode" meaning.
- Context compaction's own internal summarization call is unaffected by the user's turn-level `thinking`/`reasoning_depth`/`permission_mode` settings — it is not the user-facing turn.

**Non-Goals:**
- `permission_mode = "agent"` / `"auto"` — not implemented this phase; both remain behaviorally identical to `"default"`. See Decision 4.
- Any change to CLI-based agents' own configuration-to-flag translation (`providers/invocation.rs`) — untouched.
- Any change to the tool-approval *UI*/event flow itself — plan mode changes which tools are offered and enforced, not how the approval prompt looks or behaves for tools that are still offered.
- A `reasoning_depth`-driven Anthropic behavior beyond the on/off `thinking` boolean — adaptive thinking has no separate depth parameter on current-generation models.

## Decisions

### 1. `build_request_body`'s function-pointer signature widens with a small, provider-agnostic options struct; each provider reads only the field(s) it understands

```rust
/// Provider-agnostic knobs from `AgentChatConfiguration` that map onto a single generation
/// request. Each provider module reads only the field(s) meaningful to its own wire format —
/// mirrors how `WireFormat`'s other function pointers already share one signature across
/// providers with different per-provider bodies.
struct GenerationOptions<'a> {
    thinking: bool,
    reasoning_depth: Option<&'a str>,
}

impl GenerationOptions<'_> {
    /// Compaction's internal summarization sub-request never inherits the user's turn-level
    /// settings — see Decision 3.
    fn disabled() -> GenerationOptions<'static> {
        GenerationOptions { thinking: false, reasoning_depth: None }
    }
}
```

`build_request_body: fn(&str, &[Value], &[ToolDefinition], Option<&str>, &GenerationOptions) -> Value`. `anthropic_provider::build_request_body` reads only `.thinking`; `openai_compatible_provider::build_request_body` reads only `.reasoning_depth`. Every existing call site (2 production, both provider modules' own unit tests) passes an explicit `GenerationOptions`.

**Why:** keeps `execute()` itself format-agnostic — it builds one `GenerationOptions` from `request.configuration` and hands it to whichever provider was already selected, exactly like it already does for `tools`/`system`/`messages`. The alternative (mutating the built `Value` afterward based on `interface_format`) would leak wire-format-specific knowledge into `execute()` itself, which every existing doc comment in this file goes out of its way to avoid.

**Alternative considered:** pass the whole `&AgentChatConfiguration` through instead of a narrow options struct. Rejected — `build_request_body` would gain an implicit dependency on unrelated fields (`agent_id`, `provider_id`, `streaming`) it has no reason to know about, and the function's job is building one request body, not consuming the whole chat-configuration surface.

### 2. `thinking` → Anthropic only; `reasoning_depth` → OpenAI-compatible only

`thinking: true` on `interface_format = "anthropic"` adds `"thinking": {"type": "adaptive"}` to the request body — the modern, model-version-agnostic way to enable extended thinking (the older `budget_tokens` form is deprecated and rejected on current-generation models). `reasoning_depth: Some(depth)` on `interface_format = "openai-compatible"` adds `"reasoning_effort": "<low|medium|high>"`, folding VaneHub's `"max"` tier down to `"high"` — mirroring `providers/invocation.rs`'s own existing `"max"` → `"xhigh"` fold-down for codex-cli exactly, the same kind of "VaneHub's UI has one more tier than this particular target vocabulary" mapping already precedented in this codebase.

**Why:** these are the two request-side parameters each interface format actually defines for reasoning behavior. `thinking` has no equivalent generic OpenAI-compatible request flag (reasoning-capable models in that space largely reason automatically based on the chosen model id — e.g. a `deepseek-reasoner`-style model id — not a request-level toggle; inventing one here would be guesswork with real risk of a strict-schema endpoint rejecting an unrecognized field for no corresponding benefit). `reasoning_depth` has no Anthropic-side effect once using adaptive thinking — there is no separate depth/budget knob to set.

**Alternative considered:** also send `reasoning_effort` for Anthropic (best-effort, ignored if unsupported) or attempt some generic "enable reasoning" flag for openai-compatible. Rejected both — Anthropic's Messages API is fully known and controlled by VaneHub's own two provider modules, so guessing at an undocumented parameter has no upside; the openai-compatible case's real risk (strict-schema rejection) outweighs a speculative benefit for a vocabulary VaneHub does not control.

### 3. Compaction's internal summarization sub-request always uses `GenerationOptions::disabled()`

The `maybe_compact` call site (`api_process_adapter.rs:872`) passes `&GenerationOptions::disabled()` regardless of the user's own turn-level settings.

**Why:** summarizing the conversation so far is a mechanical, internal, auxiliary operation — not the user-facing turn the user configured `thinking`/`reasoning_depth` for. Enabling extended thinking or a high reasoning effort there would add latency and cost for a task that does not benefit from it, and the summary's own instruction (`SUMMARIZATION_INSTRUCTION`) already asks for a concise, mechanical response.

**Alternative considered:** inherit the user's settings. Rejected — no plausible benefit, real cost (latency, token spend) for an internal call the user never directly sees as a "turn."

### 4. Plan mode is enforced at both the tool-catalog level and the tool-execution level — not just one

New `plan_mode_tool_catalog()` in `tool_catalog.rs` (sibling to `tool_catalog()`, reusing a shared `remember_tool_definition()` helper factored out of both): excludes `shell` entirely; keeps `file` but with its `operation` schema narrowed to `enum: ["read"]` only; keeps `remember` unchanged. `resolve_tool_catalog` picks this instead of the normal fixed-catalog-plus-MCP-merge path when `request.configuration.permission_mode == "plan"` (and skips the MCP `catalog_entries` lookup entirely in that case — MCP tools would all be excluded anyway, so there's no reason to pay the lookup cost).

Independently, `execute_tool_call` gains a `plan_mode: bool` parameter (computed once at the `execute()` call site from `request.configuration.permission_mode == "plan"`) and hard-rejects, before any other dispatch: any MCP-prefixed name, `shell`, and `file` with `operation != "read"` — each returning `ToolExecutionOutcome{is_error: true, output: "<tool/operation> is disabled in plan mode."}`.

**Why:** the catalog restriction shapes what the model is *told* it can do (the primary, everyday defense — a well-behaved model simply never sees `shell` or a `write` option). The execution-level check is what actually *enforces* it, for the same reason MCP's call-time re-validation exists on top of catalog-time filtering (`add-agent-mcp-tools` Decision 4) and `execute_file` already fails closed on an unrecognized `operation` value: nothing prevents a model from emitting a tool name or operation value it was never offered — hallucination, or content from an earlier tool result attempting prompt injection. Relying on the catalog alone would mean "plan mode" is a suggestion, not a boundary.

**Alternative considered:** catalog-only (no execution-level check). Rejected — matches this codebase's own repeatedly-stated principle of never trusting a model's tool-call arguments as the sole gate, and the cost of the execution-level check is a handful of `if` conditions, not new infrastructure.

**Alternative considered:** intercept before the approval prompt is shown (so a plan-mode-blocked call never even reaches "awaiting approval" in the UI), not just at execution time. Deferred — the primary defense already prevents this in the non-adversarial case (the catalog never offers the tool/operation), so the only path that reaches this is hallucination or injection; catching it at execution is about correctness/safety, not everyday UX, and avoids threading `plan_mode` into the approval-prompt/event-sink code path as well. Noted as a trade-off below rather than built now.

### 5. `permission_mode = "agent"` / `"auto"` are explicitly out of scope this phase

Both are treated identically to `"default"` — no catalog change, no execution-level change, no risk-tier change.

**Why:** every CLI agent's mapping for these values reduces the amount of human-in-the-loop approval required (claude-code `acceptEdits`, codex-cli `workspace-write`, opencode `autoApprove: true`). The equivalent for a native API agent would mean this chat-configuration dropdown could silently disable `ToolRiskTier::RequiresApproval`'s mandatory approval gate for shell/file-write/MCP calls — a security-relevant behavior change to a boundary this native-agent effort has treated as deliberately fail-closed at every prior opportunity (shell always requires approval regardless of the command; unrecognized tool names/operations fail closed; MCP calls require approval unconditionally with, in `add-agent-mcp-tools`'s own words, "no auto-approve carve-out"). Implementing an approval bypass as a side effect of wiring up otherwise-inert config fields would contradict that standing pattern without a dedicated conversation about it.

**Alternative considered:** implement a narrower "auto-approve read-adjacent operations only" interpretation for `"agent"`/`"auto"`. Rejected for this phase specifically because the *right* narrower interpretation is itself the open design question — better decided deliberately in its own proposal than guessed at here.

## Risks / Trade-offs

- **[Risk]** A user-registered Anthropic agent pointing at an older model that only supports the deprecated `budget_tokens` thinking form would get a 400 when `thinking: {"type": "adaptive"}` is sent. **Mitigation:** surfaces as a normal generation failure through the existing `failure_from_http_status` classification (a clear API-side error message, not a crash); matches this codebase's general stance of not hardcoding a per-model compatibility matrix for agents the user registers with an arbitrary model id.
- **[Risk]** `reasoning_effort` is sent to *every* openai-compatible endpoint once `reasoning_depth` is set, even ones that don't support reasoning at all; a small number of strict-schema endpoints could reject an unrecognized field outright. **Mitigation:** accepted — this mirrors the same category of risk `providers/invocation.rs` already accepts for CLI agents' own flag translation, and the alternative (a per-vendor allowlist VaneHub has no way to keep current) is worse.
- **[Trade-off]** A plan-mode-blocked call is only reachable via model hallucination or prompt injection (see Decision 4's second alternative) and still shows an "awaiting approval" prompt before being rejected at execution time, rather than being intercepted earlier. **Mitigation:** accepted as noted in Decision 4 — revisit only if this proves to be an actual, observed problem rather than a theoretical one.

## Migration Plan

Purely additive: one new struct (`GenerationOptions`), one widened function-pointer signature (both provider modules updated, no external callers outside this file and its own tests), one new fixed-catalog function (`plan_mode_tool_catalog`) plus a small shared-helper refactor (`remember_tool_definition`) in `tool_catalog.rs`, one new `execute_tool_call` parameter. No schema changes, no new Tauri commands, no frontend changes (the configuration values already flow to the backend today).
