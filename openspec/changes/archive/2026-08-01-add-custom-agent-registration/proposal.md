## Why

VaneHub only supports agents that wrap a locally installed CLI process (Claude Code, Codex CLI, Gemini CLI, OpenCode) — every registered agent must have an executable VaneHub can spawn and manage. The user wants VaneHub to also host a first-party agent that talks to a provider's LLM API directly, as the foundation for a fully self-implemented agent (memory, skills, tool permissions, multi-provider support) to be layered on in later, separately proposed changes. This change is Phase 1 of that effort: prove the pipeline from "call a provider's API directly" through to "renders in VaneHub's existing chat UI," using Claude (Anthropic Messages API) as the reference provider, with no tool execution, no permission system yet.

Phase 1 was extended (still within this same change) to also support any OpenAI Chat Completions-compatible endpoint via a user-supplied `base_url`, rather than Anthropic only — most third-party and relay LLM providers speak this protocol, so one additional interface format reaches a much larger set of real-world providers than the Anthropic-only path alone.

## What Changes

- Add a new interaction mode for agents whose workflow launch path is a direct provider LLM API call over HTTP rather than a spawned CLI process, browser session, or native desktop app.
- Add Tauri commands to register this kind of agent: display name, provider (free-text label), API key (stored through `platform::credentials`, never in plaintext columns), model id, an `interface_format` (`anthropic` or `openai-compatible`), and — required for `openai-compatible` — a `base_url`.
- Add a new `agent_runtime` infrastructure adapter that calls either the Anthropic Messages API or an OpenAI Chat Completions-compatible endpoint (streaming in both cases) and translates the response into the same `started` / `token` / `thinking` / `tool_use` / `completed` / `failed` chat event vocabulary the existing CLI chat path already produces (backed by the existing `ProviderOutputEvent` enum), so the existing chat message list, persistence, and Rich Block rendering work unchanged.
- Route chat message generation to this new adapter when the active session's interaction mode is the new API mode, instead of spawning a CLI process.
- Add Web/mock parity for registration and generation in `web-agent-client.ts`.
- Clarify `agent-tool-registry`'s CLI-management requirement so it explicitly governs `launch_kind=cli` agents rather than reading as a cap on the entire agent catalog.
- **Explicitly out of scope for this change** (staged as separate future changes already agreed with the user): tool execution and the permission/approval system for it; interface formats other than Anthropic-native and OpenAI-compatible (e.g. Google Gemini's native format); context compaction / long-conversation management; a Skill system for API-based agents (this does not touch the existing, unrelated `Skill`/mount-path concept in `tooling`); cross-session memory; any explicit two-phase "plan then execute" flow. No **Agent Terminal** integration — API-based agents are chat-only in this change, not interactive terminal sessions, so `agent-terminal-runtime` and `cli-parameter-management` are unaffected.
- No breaking changes: this is additive. Existing CLI agents, Agent Terminal, and CLI chat behavior are unchanged.

## Capabilities

### New Capabilities
- `api-agent-runtime`: registration (provider, API key, model, interface format, and base URL for OpenAI-compatible agents) and chat-message generation for agents that call a provider's LLM API directly instead of spawning a local CLI process, against either the Anthropic Messages API or any OpenAI Chat Completions-compatible endpoint.

### Modified Capabilities
- `agent-tool-registry`: the "Supported CLI tool management catalog" requirement's fixed-order/CLI-only scenarios apply to `launch_kind=cli` agents specifically; it no longer describes the full universe of registrable agents (the generic "Registered agent catalog" requirement already did not cap the count).
- `interaction-modes`: add a new interaction mode, alongside the existing browser and native-desktop modes, for agents whose workflow launches through a direct provider API call.
- `chat-experience`: add a runtime execution path for sessions whose interaction mode is the new API mode — mirroring the existing "Desktop CLI chat streams provider runtime output" requirement — so streaming, persistence, and Rich Blocks flow through the same chat event pipeline as CLI sessions.

## Impact

- **`agent_runtime` (Rust, primary)**: new infrastructure adapter alongside `providers/process_adapter.rs` for direct API calls, with two interchangeable pure-function translation modules (Anthropic Messages, OpenAI Chat Completions-compatible) selected per-agent; `agents` table gains a new `launch_kind` value and new columns for model id, interface format, base URL, and provider credential reference (via `platform::credentials`, not ad hoc storage). `providers/invocation.rs` (CLI argv building) and `providers/output.rs`'s `ProviderOutputEvent` enum are reused, not replaced.
- **`sessions` (Rust)**: session creation and message generation already resolve the agent generically by id; the generation dispatch gains a branch for the new interaction mode.
- **Frontend**: `src/services/agent-service.ts`, `tauri-agent-client.ts`, `web-agent-client.ts` gain registration methods; `AgentRegistryEntry` (already `id: string`) needs no shape change. `ManagedCliAgentId` in `src/types/agent.ts` / `src/contracts/agent.ts` stays CLI-only and is **not** extended — CLI Parameters and Prompt Hooks remain CLI-only surfaces. New settings UI to register an API-based agent (name, provider, API key, model).
- **Unaffected**: `agent-terminal-runtime`, `cli-parameter-management`, `skill-management`, `prompt-hook-management`, `multi-agent-coordination` (coordination already resolves agents generically by id).
- Runs in both desktop (real API calls) and Web/mock (deterministic simulated events) runtimes per project convention.
