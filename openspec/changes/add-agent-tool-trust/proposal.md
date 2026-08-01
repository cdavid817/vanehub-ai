## Why

Native API agents (`launch_kind = "api"`) require an explicit human approval click for every `shell` call and every `file` write, with no way to configure otherwise — confirmed by direct code reading: `risk_tier_for` classifies both as `ToolRiskTier::RequiresApproval` unconditionally, and nothing in the native tool-use loop reads any persisted, agent-level setting the way CLI-based agents already can. CLI-based agents, by contrast, already have exactly this kind of persistent lever: `RuntimeAgentCliProfileAdapter::load` layers the per-turn chat configuration on top of that agent's own persisted CLI Parameter selections (`cli_profile.rs`), and a user who has separately configured a CLI agent's parameters to auto-accept edits gets that behavior in every session with that agent, including unattended ones (e.g. `add-agent-lifecycle-management`'s spec already anticipates Loop worker/verifier roles being assigned to API agents, and Loop's own role-generation deliberately passes `permission_mode: "default"` specifically so it defers to whatever persisted posture the agent already has — a mechanism native API agents don't yet have an equivalent of). This gap is what currently makes it impractical to use a native API agent anywhere a human isn't watching every single tool call, including — but not limited to — a future Loop worker role.

## What Changes

- Affects the **desktop runtime only** (native API agents don't exist in the Web/mock runtime's real sense — `web-agent-client.ts`'s existing simulated approval flow is extended to match, see Impact).
- Add a persistent, per-agent boolean setting, `autoApproveTools`, stored on the `agents` row (mirrors `model_id`/`interface_format`/`base_url`'s existing additive-migration pattern) — off by default for every agent, existing and newly registered.
- When `autoApproveTools` is enabled for an agent, `shell` and `file`-tool `write` calls for that agent skip the mandatory approval prompt and execute immediately, exactly as if a human had clicked "approve." **MCP-sourced tool calls are explicitly excluded from this setting and continue to require approval unconditionally, with no auto-approve path this phase** — MCP tools are arbitrary, third-party-defined, and VaneHub has no way to know what any given one actually does, matching this native-agent effort's existing fail-closed treatment of MCP calls (`add-agent-mcp-tools`).
- Plan mode (`add-agent-chat-configuration`) continues to override this setting unconditionally: a plan-mode generation still hard-rejects `shell` and file writes at the execution boundary regardless of `autoApproveTools`, with no exception.
- Enabling the setting requires a dedicated, explicit confirmation step — a distinct action from ordinary agent editing (`update_api_agent`), with UI copy that states plainly what is being granted ("this agent can run shell commands and modify files without asking each time, in every session"). Disabling it is a plain, unconfirmed action, since it only restores the existing stricter default.
- Add a new Tauri command (and matching Web/mock method) to read and set this flag, separate from `update_api_agent`/`get_api_agent_provider_config`.

## Capabilities

### New Capabilities
- `agent-tool-trust`: A native API agent can be granted a persistent, explicitly-confirmed exemption from per-call approval for shell execution and file writes, with MCP calls and plan mode both remaining unaffected by the grant.

### Modified Capabilities
(none — additive on top of `agent-tool-execution`'s risk-tier classification and `agent-mcp-tools`'/`agent-chat-configuration`'s existing dispatch/plan-mode gates; no existing requirement changes.)

## Impact

- **Affected code (desktop runtime only)**: `contexts::agent_runtime` schema (new `agents.auto_approve_tools` column), `application::models::ApiProviderConfig` (new field, read alongside the agent's existing model/interface/base-url settings), the tool-use loop's approval-tier decision in `infrastructure::api_process_adapter.rs` (a new composing function alongside `risk_tier_for`, not a change to `risk_tier_for` itself), a new Tauri command pair (get/set), and the agent settings UI (a new confirmation surface, separate from the existing `AgentEditDialog`).
- **Web/mock runtime**: `web-agent-client.ts`'s existing simulated shell-approval sequence gains a mock trust flag so the simulation stays representative.
- **No impact** to `risk_tier_for`'s existing pure classification function (kept unchanged and independently testable), to MCP tool approval (still unconditional), to plan mode's own enforcement (still unconditional and always wins), or to CLI-based agents (they already have an equivalent, untouched mechanism via CLI Parameter profiles).
- Directly unblocks a follow-up change (not part of this one): removing Loop's hardcoded `InteractionMode::Cli` requirement for worker/verifier role assignment, which today would otherwise make an API-agent Loop worker request approval on every single tool call of every single iteration.
