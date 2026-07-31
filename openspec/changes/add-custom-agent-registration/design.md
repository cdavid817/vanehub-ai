## Context

Today every registered agent has `launch_kind = "cli"` (`src-tauri/src/contexts/agent_runtime/infrastructure/schema.rs`) and chat generation for such agents is explicitly scoped to `interaction mode == cli` (chat-experience: "Desktop CLI chat streams provider runtime output"). That path spawns a provider CLI process and normalizes its stdout into `ProviderOutputEvent` (`agent_runtime/infrastructure/providers/output.rs`: `Token`, `Thinking`, `ToolLifecycle`, `RichBlock`, `SessionId`, `Completed`, `Failed`, `Empty`), which the frontend already renders as `started` / `token` / `thinking` / `tool_use` / `completed` / `failed` chat events with full persistence and Rich Block support. Interactive terminal sessions (`agent-terminal-runtime`) and CLI launch-parameter profiles (`cli-parameter-management`) are a separate, CLI-only concern layered on top of this and are not touched by this change.

VaneHub's Rust backend has no official Anthropic SDK to depend on (the official SDKs cover Python/TypeScript/Java/Go/Ruby/C#/PHP, not Rust), so the new provider integration calls the Anthropic Messages API directly over HTTP with streaming (SSE).

This is Phase 1 of a larger, explicitly staged effort (agreed with the user) to give VaneHub a first-party, self-implemented agent. Later phases — tool execution and a permission/approval system, context compaction, a Skill system for these agents, cross-session memory — are out of scope here and will be separate proposals building on the foundation this change lays.

**Scope revision (still Phase 1):** after the Anthropic-only adapter shipped, the user asked to also support arbitrary OpenAI Chat Completions-compatible endpoints — not OpenAI's own hosted API specifically, but the wire protocol that the large majority of third-party and relay LLM providers (DeepSeek, Moonshot/Kimi, Zhipu GLM, Qwen, most self-hosted inference servers, most aggregator gateways) speak, since it lets one endpoint type reach far more real-world providers than an Anthropic-only client can. This is added to Phase 1 rather than deferred to a later phase because Decision 3 already shaped the port for exactly this (`AgentProcessGateway` reuse + a per-provider pure-function translation module) — implementing it now is additive to the existing shape, not a redesign. See Decision 7.

## Goals / Non-Goals

**Goals:**
- Register an agent that is configured with a provider, an API key, and a model — not a local executable.
- Run chat message generation for such an agent by calling the Anthropic Messages API directly (streaming) and translating the response into the existing `ProviderOutputEvent` vocabulary, so the existing chat UI, persistence, and Rich Block rendering work with no changes.
- Define the new execution path behind an application port shaped so more providers can be added later as new infrastructure implementations, without changing application/domain code or the port's public shape in a breaking way.
- Also register and run agents against any OpenAI Chat Completions-compatible endpoint (a user-supplied `base_url`, not a fixed host) — covering the large class of third-party/relay providers that speak this protocol, alongside the Anthropic-native path. Both formats coexist; a registered agent picks one via `interface_format`.
- Preserve full Web/mock parity for registration and generation.

**Non-Goals:**
- Tool execution or a permission/approval system for it.
- Any interface format beyond Anthropic-native and OpenAI Chat Completions-compatible (e.g. Google Gemini's native format, or other provider-bespoke formats) — the port and translation-module pattern support adding these later the same way, just not in this change.
- Context compaction or any long-conversation management strategy.
- A Skill system for these agents (unrelated to the existing `tooling` Skill/mount-path concept, which this change does not touch).
- Cross-session memory.
- An explicit two-phase "plan then execute" flow — this is a normal single-turn/multi-turn conversational loop.
- Agent Terminal (interactive PTY session) integration, CLI Parameter Management, or Prompt Hook integration — all three are specified as CLI-only today and stay that way in this change.

## Decisions

### 1. Model this as a new interaction mode + launch kind, not a 5th CLI

Add `launch_kind = "api"` on the `agents` table (parallel to the existing `"cli"` value) and a new interaction mode (placeholder name `api` — confirm no collision with existing `InteractionMode` values before implementation) parallel to the existing `cli` / `browser` / `native-desktop` modes.

**Why:** `chat-experience`'s CLI streaming requirement is already conditioned on `interaction mode == cli` — that is the existing seam to mirror, not `agent_runtime/infrastructure/providers/invocation.rs`'s closed `match agent_id { ... }`, which is specifically CLI-argv construction and has no meaning for an HTTP call.

**Alternatives considered:**
- Add a 5th arm to `invocation.rs`'s match — rejected; that file builds subprocess command lines, a different execution model entirely.
- Reuse the existing `native-desktop` interaction mode — rejected; that mode means a local OS-window application, an unrelated concept.

### 2. Reuse `ProviderOutputEvent` as the adapter's output shape

The new adapter's only job is: given a conversation, produce a stream of `ProviderOutputEvent`s — translating Anthropic Messages API SSE events (`content_block_delta` text/thinking deltas, `message_stop`, error responses) into `Token`, `Thinking`, `Completed`, `Failed`. (`ToolLifecycle` and `RichBlock` are not produced in this phase — no tools yet.)

**Why:** this is the single mechanism that lets the existing chat message list, SQLite persistence, and Rich Block rendering work unmodified. No new frontend rendering path is needed.

**Alternative considered:** a dedicated UI/event model for API-based agents — rejected as unnecessary; the existing vocabulary already fits.

### 3. New application port; Claude-only infrastructure implementation now, shaped for multi-provider later

Define a narrow, behavior-oriented port in `agent_runtime::application::ports` (exact Rust type name is an implementation detail for tasks.md) with the shape "given conversation history, model id, and a credential reference, return a stream of `ProviderOutputEvent`." Implement it once, in a new `agent_runtime::infrastructure::providers` module, calling Anthropic's Messages API.

**Why:** per `openspec/project.md`, application ports must be narrow and must not depend on concrete network/credential adapters. Shaping the port around "conversation in, event stream out" — rather than around Claude's specific request/response shape — keeps adding OpenAI in Phase 3 a matter of a new infrastructure implementation, not a port redesign.

**Open question, not resolved here:** whether the existing CLI process adapter and this new API adapter should implement one shared port or two separate ports. A CLI generation naturally carries process-lifecycle concepts (PID, stdin/stdout) that an HTTP call does not; forcing a single shared interface risks leaking process semantics into a use case that has none. Left to whoever writes the port, constrained by "ports MUST be narrow and behavior-oriented" (project.md) — prefer two ports if unifying them would require optional/unused fields on either side.

### 4. Credentials through `platform::credentials`, never a plaintext column

The API key is stored through the existing credential store; the `agents` table (or a small new side table) stores a reference to the stored credential, not the raw key.

**Why:** `openspec/project.md` assigns credential storage to `platform::credentials`; IM connector credentials already follow this pattern (`native-runtime-architecture`: "secure credential access"). This change follows the same convention rather than inventing a new one.

### 5. The streaming API call runs through the existing non-blocking chat-generation path

CLI chat generation already streams token-by-token to the frontend without blocking the Tauri command boundary (`chat-experience`: "Desktop CLI chat streams provider runtime output" — "token events SHALL be emitted as output becomes available rather than only after process exit"). The new adapter's streaming HTTP call should be wired into that same non-blocking event-emission mechanism, not a newly invented one.

**Why:** `native-runtime-architecture`'s "General nonblocking native operations" and "Bounded native request response operations" requirements are explicit that variable-duration network work must not block the Tauri command boundary or main thread. Reusing the existing streaming plumbing avoids re-solving a problem the codebase already solved for CLI chat.

### 6. No Agent Terminal, CLI Parameter Management, or Prompt Hook integration

API-based agents are plain chat sessions in this phase; model is fixed at registration time (no per-message override, no launch-parameter catalog). All three referenced capabilities are formally scoped to CLI agents today (`agent-terminal-runtime`: "single-Agent CLI sessions"; `cli-parameter-management`: enumerates exactly the four CLI ids; Prompt Hook chat assembly is scoped to CLI chat invocations) and this change leaves that scoping as-is.

### 7. OpenAI Chat Completions-compatible format as a parallel, selectable infrastructure implementation

Add `interface_format` to agent registration, with exactly two values: `anthropic` (existing, default endpoint `https://api.anthropic.com/v1/messages`, unchanged) and `openai-compatible` (new, endpoint is `{base_url}/chat/completions` where `base_url` is a required, user-supplied field — e.g. `https://api.deepseek.com/v1`, `https://api.moonshot.cn/v1` — following the same "base URL up to and including `/v1`" convention the OpenAI SDK and most compatible tooling already use). `base_url` is required when `interface_format = openai-compatible` and unused/ignored for `anthropic`. The existing free-text `provider` field stays a human-readable label only (e.g. "DeepSeek") and does not select the wire format — `interface_format` does.

Implementation shape, mirroring the existing Anthropic path exactly:
- New `agent_runtime::infrastructure::openai_compatible_provider` module: pure, I/O-free functions with the same signatures as `anthropic_provider.rs` — `build_request_body(model, history) -> Value` (OpenAI shape: `{model, messages: [{role, content}], stream: true}`), `translate_sse_data(data) -> Option<GenerationProcessEvent>` (handles `choices[0].delta.content` → `Token`, the common `delta.reasoning_content` extension → `Thinking`, the literal `data: [DONE]` sentinel → `Completed`), `failure_from_http_status(status, body) -> GenerationProcessFailure`.
- `RuntimeAgentApiAdapter::execute()` (`api_process_adapter.rs`) branches on the agent's `interface_format` to pick the endpoint, auth header shape (`x-api-key` + `anthropic-version` vs. `Authorization: Bearer`), and which module's `build_request_body`/`translate_sse_data` to call. Everything else in `execute()` (credential fetch, history fetch, cancellation-aware SSE line loop, terminal-event handling) is format-agnostic and stays shared.
- `ApiAgentGateway::model_id(agent_id) -> Option<String>` is replaced by a slightly richer `ApiAgentGateway::provider_config(agent_id) -> Option<ApiProviderConfig { model_id, interface_format, base_url }>` — the three fields are always read together by `execute()`, so one query replaces three.
- Schema: two new nullable columns on `agents` — `interface_format TEXT`, `base_url TEXT` — additive migration, backfills existing `launch_kind = 'api'` rows (i.e. agents already registered by this same Phase 1 change) to `interface_format = 'anthropic'` so they keep working unchanged.
- Availability (`SqliteAgentRuntimeRepository::into_domain`): an `api` agent is available when it has a credential, a `model_id`, and — only when `interface_format = openai-compatible` — a non-empty `base_url`.
- Frontend: registration form gains an "接口格式" selector (Anthropic 原生 / OpenAI 兼容, defaulting to OpenAI 兼容 per the user's stated default) and a conditionally-shown Base URL field when OpenAI-compatible is selected. `RegisterApiAgentInput` gains `interface_format` and `base_url` (optional, required only for the openai-compatible branch — validated the same way client- and server-side as the existing four fields).

**Why:** this is additive to Decision 2/3's existing shape (`GenerationProcessEvent` vocabulary, `AgentProcessGateway` reuse, pure-function SSE translation) rather than a redesign — the same pattern that made the Anthropic adapter cheap to unit-test (task 4.3/4.6) applies unchanged to a second format. Consolidating `model_id` into `provider_config` avoids three separate SQLite round-trips per generation now that there are three related fields instead of one.

**Alternative considered:** keep `RuntimeAgentApiAdapter` Anthropic-only and add a second, fully separate adapter struct routed by `CompositeAgentProcessGateway` (the same pattern used to route CLI vs. API) — rejected; the two formats share 100% of the adapter's actual mechanics (credential/history fetch, cancellation, SSE line loop, thread spawn/registration bookkeeping) and differ only in request shape/auth header/endpoint/event translation, which the pure-function module split already isolates cleanly. A second full adapter struct would duplicate all of that shared mechanics for no benefit.

## Risks / Trade-offs

- **[Risk]** No official Anthropic Rust SDK means hand-rolled HTTP/SSE parsing against the Messages API, which carries more risk of subtle streaming-format bugs than an official SDK. → **Mitigation:** keep the SSE-event-to-`ProviderOutputEvent` translation a small, pure, unit-testable function, following the same fixture-driven test pattern already used for CLI output parsing (`agent_runtime/infrastructure/providers/tests.rs`).
- **[Risk]** Mishandled API keys could leak into diagnostics or crash output. → **Mitigation:** credential material flows only through `platform::credentials`; the new adapter's diagnostic logging redacts the key exactly like existing CLI command-argument redaction.
- **[Risk]** Designing the application port around the CLI adapter's existing shape would leak process semantics (PID, stdin) into a use case that has none. → **Mitigation:** derive the port from the "conversation in, event stream out" contract `chat-experience` already implies, not from the CLI adapter's current Rust signature.
- **[Risk]** Scope creep — it is easy to accidentally pull Phase 2+ items (basic tool support, per-message model override) into this change. → **Mitigation:** the proposal's explicit non-goals list; task review should push back on anything drifting past Phase 1's stated scope.
- **[Trade-off]** Model is a single value fixed at registration time, with no per-message override (unlike CLI parameter profiles). Acceptable for Phase 1; revisit only if a later phase needs per-message model switching for API agents.

## Migration Plan

Purely additive: new `launch_kind` / interaction-mode value, new SQLite migration (additive, preserves all existing `agents`/`sessions`/`messages` data per project convention), new adapter, new commands, new settings UI entry point. No existing agent, session, or message data changes. Rollback is simply not registering any API-based agent — existing CLI agents and their terminal/chat paths are entirely unaffected.

A second additive migration adds `interface_format` and `base_url` to `agents` and backfills `interface_format = 'anthropic'` for any pre-existing `launch_kind = 'api'` row, so agents registered before this scope revision keep behaving exactly as before.

## Open Questions

- Exact name for the new `launch_kind` / interaction-mode value (this document uses `api` as a placeholder) — confirm it does not collide with any existing `InteractionMode` variant.
- One shared generation port for CLI and API adapters, or two separate ports (see Decision 3) — resolve while writing the port, constrained by "narrow and behavior-oriented."
- Exact location of the existing non-blocking streaming/event-emission mechanism used by CLI chat generation, to be located and reused rather than reinvented (Decision 5).
- New table vs. new columns on `agents` for provider/model/credential-reference — left to schema design at task-execution time, constrained by "additive migration only."
- Which HTTP client crate `platform::network` (if any) already standardizes on, to reuse rather than introducing a new HTTP dependency without checking first.
