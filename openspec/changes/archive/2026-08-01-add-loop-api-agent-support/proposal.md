## Why

Loop (multi-agent Worker/Verifier collaboration) only accepts CLI-launched agents today, purely because that was the only agent kind that existed when Loop was built — the eligibility check is a hardcoded `InteractionMode::Cli` requirement, not a deliberate design constraint. Native API agents (`launch_kind = "api"`) can now run tool-using, multi-turn sessions on their own (shell/file execution, MCP tools, cross-session memory, Skills), and — as of the already-merged `add-agent-tool-trust` change — can also be marked trusted so their shell/file calls don't block on a human approval click. That trust mechanism was built specifically to unblock unattended automation like Loop, so this is a natural next step: let a trusted API agent serve as a Loop Worker or Verifier, expanding Loop beyond whichever CLI tools a user happens to have installed.

## What Changes

- Loop definition validation (`validate_agent`, shared by definition save and run start) accepts agents that support either `Cli` or `Api` interaction mode, instead of requiring `Cli` unconditionally.
- When the resolved agent is an API agent, validation additionally requires the agent's tool-trust flag (`auto_approve_tools`, from `add-agent-tool-trust`) to be enabled. An untrusted API agent is rejected up front with a clear error, both when saving a Loop definition and when starting a run — never silently stalled mid-run waiting for an approval nobody is watching for.
- Loop Worker/Verifier role-session creation (`SessionsAgentRuntimeAdapter::create_worker_session`/`create_verifier_session`) resolves and passes the target agent's real interaction mode instead of hardcoding `Cli`, so the underlying session is created with the correct mode and downstream routing (`AgentProcessGateway`) dispatches to the API-agent HTTP path instead of the CLI subprocess path.
- Loop role generation (`start_loop_role_generation`) resolves the real interaction mode from the agent registry instead of hardcoding `Cli`, so the `AgentChatConfiguration` passed to `send_message` is correct for API agents.
- No changes to Verifier result parsing, git worktree diffing, tool workspace scoping, Loop's state machine, or Loop's SQLite schema — all confirmed already agent-kind-agnostic.
- MCP tool calls remain unconditionally gated behind human approval even for a trusted agent (unchanged from `add-agent-tool-trust`). A Loop-role API agent with MCP tools bound can still stall on an MCP call during an unattended run — this is a known, documented limitation for this phase, not something this change attempts to solve.
- No new Loop-definition-editor UI filtering or annotation. The agent picker keeps listing every registered agent unfiltered, same as today; an ineligible selection surfaces through the existing generic validation-error path when the definition is saved.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `loop-engineering-runtime`: the Loop definition/start eligibility requirement ("Reject unsupported first-phase scope") is extended to define which agents are eligible for a Worker/Verifier role — CLI agents (unchanged) or trusted API agents (new) — and to reject untrusted API agents with a clear error at both definition-save and run-start time.

## Impact

- **Affected code (Rust, desktop runtime only — no Web/mock-adapter behavior to change, since Loop's Web adapter already simulates its own agent list without a CLI-only restriction)**:
  - `src-tauri/src/contexts/agent_runtime/application/loop_service.rs` — `validate_agent`'s eligibility check, plus a new dependency on agent tool-trust state.
  - `src-tauri/src/contexts/agent_runtime/application/service.rs` — `start_loop_role_generation`'s hardcoded `AgentChatConfiguration.interaction_mode`.
  - `src-tauri/src/contexts/agent_runtime/infrastructure/sessions_gateway.rs` — `create_worker_session`/`create_verifier_session`'s hardcoded `interaction_mode`.
  - `src-tauri/src/contexts/agent_runtime/application/loop_models.rs` — `LoopRoleSessionRequest` gains an `interaction_mode` field.
  - `src-tauri/src/contexts/agent_runtime/application/loop_worker.rs` / `loop_verifier.rs` — resolve the target agent's mode before building `LoopRoleSessionRequest`; their `*ApplicationPorts` structs gain a registry-shaped dependency.
  - `src-tauri/src/bootstrap/agent_runtime.rs` — thread the existing `repository` (already implements both the agent-registry and `ApiAgentGateway` ports) into the newly-dependent port structs.
- **Not affected**: `src-tauri/src/contexts/sessions/application/service.rs`'s `create_loop_role_session` (its `ensure_agent_supports` check already works correctly once the right mode is passed in — no code change needed there), Verifier result parsing, git worktree/diff logic, Loop's frontend (`src/loop-center/`), SQLite schema.
- **Dependency**: builds directly on the already-merged `add-agent-tool-trust` change's `auto_approve_tools` flag and `ApiAgentGateway::provider_config` accessor.
