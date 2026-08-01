## Why

Phase 1 (`add-custom-agent-registration`) gave VaneHub a first-party agent that can call an LLM provider's API directly, but it is purely conversational: `RuntimeAgentApiAdapter::execute()` sends one request, streams text/thinking back, and returns — there is no agentic loop, no tools, and no way for the agent to act on the user's machine. Every CLI-based agent VaneHub manages (Claude Code, Codex CLI, etc.) already has its own tool-use loop and permission prompting baked into the external binary; VaneHub's own backend has never had to implement one, because it never drove tool execution itself. This change is Phase 2 of the native-agent effort: give the native agent an actual tool-use loop — shell and file-editing tools, gated by a permission/approval step — using Claude (Anthropic Messages API) and any OpenAI Chat Completions-compatible endpoint (both already shipped in Phase 1) as the reference providers.

## What Changes

- Affects the **desktop runtime only** for real tool execution (shell commands and file access require a local OS process/filesystem, which the Web/mock runtime does not have); the Web/mock runtime gets deterministic simulated tool-call events so the frontend contract stays identical across runtimes.
- Add a tool-use loop to `RuntimeAgentApiAdapter`: after a generation turn ends with the model requesting one or more tool calls (Anthropic `tool_use` content blocks / OpenAI `tool_calls`), the adapter executes them and sends the results back as a new turn, repeating until the model produces a final response with no further tool calls.
- Add exactly two tools in this phase: a shell/bash tool and a file read/write tool, each executed through existing `platform::process` (bounded, timeout, cancellable subprocess execution) and `platform::filesystem` (boundary-enforced sandboxed path resolution) primitives rather than new ad hoc process/path-handling code.
- Add a risk-tiered permission/approval gate: read-only tool calls (file reads) execute immediately with no prompt; tool calls with side effects (file writes, shell execution) pause the loop and require an explicit user approve/deny decision, surfaced through a new Tauri command and frontend UI, before running.
- Translate both the tool-definition request shape and the tool-result reply shape for both already-shipped interface formats (Anthropic Messages `tools`/`tool_use`/`tool_result`; OpenAI Chat Completions `tools`/`tool_calls`/`role: "tool"`) into the existing, already-frontend-rendered `GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent)` / `ToolUseBlock` vocabulary — no new event vocabulary, no new chat-rendering code.
- Add Web/mock parity: `web-agent-client.ts` simulates a deterministic tool-call/approval/result sequence for API-based agent sessions, without real process or filesystem access.

## Capabilities

### New Capabilities
- `agent-tool-execution`: the native API-based agent's tool-use loop, its shell and file-read/write tools, and the permission/approval gate that authorizes each tool call before it runs.

### Modified Capabilities
- None. `api-agent-runtime` (introduced by the still-unarchived `add-custom-agent-registration` change) is extended in behavior — a message can now involve multiple provider round-trips instead of exactly one — but since that capability has no archived baseline yet to diff against, this change describes the tool-loop behavior entirely within the new `agent-tool-execution` capability rather than as a delta against a not-yet-merged spec.

## Impact

- **`agent_runtime` (Rust, primary)**: `RuntimeAgentApiAdapter::execute()` gains a loop instead of a single request; `anthropic_provider.rs` and `openai_compatible_provider.rs` gain tool-definition request-building and tool-call/tool-result translation; a new tool-execution module (shell + file tools) built on `platform::process`/`platform::filesystem`; a new Tauri command to resolve a pending approval. No SQLite changes — pending approvals and in-loop tool turns are in-memory state on the already-existing managed-generation bookkeeping, not persisted (see design.md Decisions 2 and 4).
- **`platform` (Rust)**: no new primitives expected — `platform::process::ProcessRequest`/`ProcessAdapter` and `platform::filesystem::BoundedFilesystem` are reused as-is; confirm during design whether either needs a narrow extension (e.g. an additional bounded-output ceiling for tool output specifically) rather than assuming none is needed.
- **Frontend**: new approval UI (a chat-inline prompt: tool name, input, approve/deny) wired through `agent-service.ts` → `tauri-agent-client.ts`/`web-agent-client.ts`; no changes to existing message list/streaming rendering, since `ToolLifecycleEvent` rendering already exists for CLI agents.
- **Unaffected**: `agent-terminal-runtime`, `cli-parameter-management`, `prompt-hook-management`, the existing CLI process adapter and its 4 managed CLIs, `skill-management` (the unrelated CLI-skill-mount-path concept).
- No breaking changes: purely additive to the native agent's execution path; agents without tool calls in a turn behave exactly as they do today.
