## Why

The chat configuration panel's reasoning-depth, extended-thinking, and permission-mode controls (`AgentChatConfiguration.reasoningDepth`/`thinking`/`permissionMode`) are already translated into real CLI flags for every CLI-based agent (`providers/invocation.rs::apply_configuration_overrides` — claude-code, codex-cli, gemini-cli, opencode) but are never read anywhere in the native API agent's generation path (`api_process_adapter.rs`). The UI does not hide or disable these controls for API-launch-kind agents, so a user can toggle "Extended Thinking" or pick a reasoning depth for a native Claude/OpenAI-compatible agent and see no effect at all, with no indication anything was ignored. This is the same class of problem as two correctness bugs already fixed earlier in this native-agent effort (Prompt Hook assembly and Scheduled Tasks both used to silently misbehave for non-CLI agents): a control that visibly exists but silently does nothing is worse than no control.

## What Changes

- Affects the **desktop runtime only** for real behavior change (only `launch_kind = "api"` agents are affected); the Web/mock runtime already simulates a `thinking` event gated on `config.thinking` and needs no change.
- When `configuration.thinking` is `true` and the agent's `interfaceFormat` is `anthropic`, the request body sent to the Messages API includes `"thinking": {"type": "adaptive"}`, enabling real extended thinking. The receiving/rendering pipeline for thinking content already exists and needs no change (`anthropic_provider::translate_content_block_delta` already turns a `thinking_delta` into `GenerationProcessEvent::Thinking`).
- When `configuration.reasoningDepth` is set and the agent's `interfaceFormat` is `openai-compatible`, the request body includes `"reasoning_effort": "<low|medium|high>"` (VaneHub's `"max"` tier folds down to `"high"`, the same way `providers/invocation.rs` already folds codex-cli's `"max"` down to `"xhigh"`). The receiving pipeline already exists too (`openai_compatible_provider::translate_delta` already turns a `reasoning_content` delta into the same `GenerationProcessEvent::Thinking`).
- When `configuration.permissionMode` is `"plan"`, the tool catalog offered to the model for that generation excludes `shell`, the `file` tool's `write` operation, and every MCP-sourced tool — matching every CLI agent's own "plan mode" meaning (explore and plan, cannot make edits or run commands). `remember` stays available (it only ever touches VaneHub's own storage, already auto-approved everywhere else). This restriction is provider-agnostic — it changes which tools are declared, not a wire-format-specific request field.
- `configuration.permissionMode` values `"agent"` and `"auto"` are explicitly **not** implemented this phase — treated identically to `"default"` (no behavior change). Doing so would mean a chat-configuration dropdown could silently bypass the mandatory tool-approval gate (`ToolRiskTier::RequiresApproval`) for shell/file-write/MCP calls, a security-relevant behavior change to a boundary this native-agent effort has treated as deliberately fail-closed throughout (shell always requires approval; unknown tools/operations fail closed; MCP calls require approval unconditionally with no auto-approve carve-out). That decision deserves its own dedicated proposal, not to be bundled quietly here.

## Capabilities

### New Capabilities
- `agent-chat-configuration`: Native API agents honor the chat configuration panel's extended-thinking, reasoning-depth, and plan-mode controls, to the extent each has a real, well-defined meaning for a direct-API agent.

### Modified Capabilities
(none — additive on top of the existing `agent-tool-execution`/`agent-mcp-tools` tool-catalog and request-building integration points; no existing requirement changes.)

## Impact

- **Affected code (desktop runtime only)**: `contexts::agent_runtime::infrastructure::api_process_adapter.rs` (tool catalog selection per generation), `anthropic_provider.rs`/`openai_compatible_provider.rs` (`build_request_body`).
- **No frontend changes** — `AgentChatConfiguration` and its UI controls already exist and are already sent to the backend for every session regardless of launch kind; this change only makes the native API agent path actually read and act on values it already receives.
- **No impact** to CLI-based agents' own configuration-to-flag translation (`providers/invocation.rs`, untouched), to the tool-approval UI/flow itself (only which tools are *offered* changes in plan mode, not how approval works), or to `permissionMode` values `"agent"`/`"auto"` (explicitly deferred, see above).
