## Context

Loop's Worker/Verifier eligibility check is a single hardcoded `agent.ensure_selectable(InteractionMode::Cli)` call in `loop_service.rs::validate_agent`, called from both `validate_definition_environment` (definition save) and `validate_start` (run start). Two more call sites independently hardcode `InteractionMode::Cli` when actually launching a role: `service.rs::start_loop_role_generation` (builds the `AgentChatConfiguration` passed to `send_message`) and `sessions_gateway.rs::create_worker_session`/`create_verifier_session` (builds the session-creation request passed into the `sessions` context).

Everything else Loop does — Verifier result parsing (`loop_verifier.rs::parse_result`, which just parses the final message's text as `{recommendation, findings}` JSON), git worktree isolation and diffing (`LoopGitStatePort`), tool workspace scoping (shell/file tools already scope to `session.folder`, which Loop sets to the worktree path), the run/iteration state machine, and the SQLite schema — was confirmed by direct reading to already be agent-kind-agnostic. This change is scoped narrowly to the three hardcodes plus one new eligibility rule; it is not a rearchitecture.

The critical constraint shaping this design: native API agents execute their own tool calls (shell/file) through VaneHub's own `execute_tool_call`, which blocks on human approval unless the agent is marked trusted (`add-agent-tool-trust`, already merged). CLI agents never touch this gate at all — their tool calls happen inside their own PTY, governed by the CLI's own native config (CLI Profile). Loop runs are meant to be unattended. An untrusted API agent used as a Worker/Verifier would stall on its first shell/file call with nobody watching to approve it. This asymmetry — CLI agents were always "safe by construction" for unattended use, API agents are not — is why this change bundles a new eligibility rule (trust required) alongside the mechanical mode-plumbing fixes, rather than shipping the plumbing alone.

## Goals / Non-Goals

**Goals:**
- Allow a trusted API agent (`auto_approve_tools == true`) to be selected and to actually run as a Loop Worker or Verifier.
- Reject an untrusted API agent up front — at definition save and at run start — with a clear, specific error, never a silent runtime stall.
- Keep CLI agent behavior completely unchanged (same eligibility path, same session creation behavior).

**Non-Goals:**
- No change to MCP tool-call approval. MCP calls remain unconditionally gated even for a trusted agent (existing `add-agent-tool-trust` invariant, reaffirmed, not touched). A Loop-role API agent with MCP tools bound can still stall on an MCP call mid-run — accepted as a known, documented limitation, not solved here.
- No new Loop-definition-editor UI filtering, graying-out, or annotation of ineligible agents in the Worker/Verifier picker. It keeps listing every registered agent unfiltered, exactly as today; an ineligible selection surfaces through the existing generic validation-error display when the definition is saved.
- No continuous re-validation of trust during a run. Trust is checked at definition-save and run-start (both go through `validate_agent`); it is not re-checked before every individual iteration. See Risks below.
- No changes to the unrelated `launch_workflow`/`coordination_executor.rs` "workflow launch" concept, which also hardcodes CLI and also rejects `InteractionMode::Api` — confirmed via grep that Loop's own session creation never calls into it. Out of scope.

## Decisions

### Decision 1: `validate_agent` branches on the agent's actual supported mode, not a single hardcoded one

```rust
fn validate_agent(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
    let agent = self.ports.registry.find(agent_id)?
        .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))?;
    if agent.supports(InteractionMode::Cli) {
        agent.ensure_selectable(InteractionMode::Cli)?;
        return Ok(());
    }
    agent.ensure_selectable(InteractionMode::Api)?;
    let trusted = self.ports.api_agents.provider_config(agent_id)?
        .map(|config| config.auto_approve_tools)
        .unwrap_or(false);
    if !trusted {
        return Err(loop_validation(
            "API agent must have tool trust enabled before it can be used as a Loop Worker or Verifier.",
        ));
    }
    Ok(())
}
```

An agent's `supported_interaction_modes` is set once at registration and is exclusively either a CLI-family set (`[Cli]`, or `[Cli, Browser]`, etc.) or exactly `[Api]` — `launch_kind` is a hard, mutually-exclusive registration-time choice, so `supports(Cli)` and `supports(Api)` cannot both be true in practice. The `Cli`-first branch preserves 100% of existing behavior (same call, same error) for every CLI agent; the `Api` branch is entirely new code, touched only when a non-CLI agent is evaluated.

**Alternative considered**: check `agent.launch.kind == "api"` directly instead of `supports(InteractionMode::Api)`. Rejected — `supports`/`ensure_selectable` is the domain type's own existing vocabulary for this exact question (mode eligibility, including the availability/authentication checks `ensure_selectable` already performs), and every other eligibility check in this codebase goes through it. Branching on `launch_kind` would introduce a second, parallel way to ask the same question.

### Decision 2: `LoopApplicationPorts` gains an `api_agents: Arc<dyn ApiAgentGateway>` field

`ApiAgentGateway::provider_config(agent_id) -> Result<Option<ApiProviderConfig>, _>` (added in `add-agent-tool-trust`) is the only existing accessor for `auto_approve_tools`; it is not on the domain `AgentDefinition` returned by `AgentRegistryRepository::find` (deliberately — `add-agent-tool-trust` kept it on the application-layer `ApiProviderConfig`, not the domain type). `LoopApplicationPorts` has no such dependency today.

At `bootstrap/agent_runtime.rs:90`, `repository = Arc::new(SqliteAgentRuntimeRepository::new(...))` already implements both `AgentRegistryRepository` and `ApiAgentGateway` — it's the same concrete type already handed to several other port structs as different trait objects. Wiring this in is one more `.clone()` at the `LoopApplicationPorts { ... }` construction site (`bootstrap/agent_runtime.rs:215-221`), not a new adapter.

`provider_config` returning `None` (agent not found as an API-agent config row) is treated as untrusted (`unwrap_or(false)`), fail-closed — this should be unreachable given `ensure_selectable(Api)` already confirmed the agent is a registered API agent, but there is no reason to treat an unexpected `None` as trusted.

### Decision 3: `LoopRoleSessionRequest` gains an `interaction_mode: InteractionMode` field, resolved by the caller before session creation

`SessionsAgentRuntimeAdapter::create_worker_session`/`create_verifier_session` (`sessions_gateway.rs`) has no registry access and receives only `agent_id` — it cannot resolve the correct mode itself without a new dependency at the infrastructure layer. Instead, the resolution happens one layer up, where a registry dependency is cheap to add and where the result is a natural, typed field on the existing request:

- `LoopWorkerApplicationPorts` (`loop_worker.rs`) and `LoopVerifierApplicationPorts` (`loop_verifier.rs`) each gain a `registry: Arc<dyn AgentRegistryRepository>` field.
- Before constructing `LoopRoleSessionRequest`, `loop_worker.rs::launch_iteration` / the equivalent in `loop_verifier.rs` look up the target agent and set `interaction_mode` using the same "prefer `Cli` if supported, else `Api`" rule as Decision 1 (for a `Cli`-family agent, this always resolves to `Cli` specifically — Loop launches these agents as terminal sessions, never as `Browser`, even if an agent's `supported_interaction_modes` happens to also list `Browser`).
- `sessions_gateway.rs` converts the typed `InteractionMode` to `.as_str().to_string()` only at the point it crosses into the `sessions` context's own `SessionLoopRoleRequest` — mirroring how every other agent_runtime → sessions boundary conversion in this file already works.
- `bootstrap/agent_runtime.rs:217`'s `registry: repository` (currently a move, the last use of `repository` in the function at that point) becomes `registry: repository.clone()`, since `repository` is now needed again later for the two new port structs (constructed further down, around lines 235 and 249).

`sessions::create_loop_role_session`'s own `ensure_agent_supports(&request.agent_id, &request.interaction_mode)` check is unchanged — it already independently re-validates mode support server-side; it will simply start passing for `Api` once the correct value flows through instead of always `"cli"`. This existing check is not redundant with Decision 1 — it protects the `sessions` context's own invariants regardless of what `agent_runtime` sends it, and needs no changes to do so correctly.

### Decision 4: `service.rs::start_loop_role_generation` resolves mode via the registry it already has

```rust
let agent = self.ports.registry.find(&session.agent_id)?
    .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(session.agent_id.clone()))?;
let interaction_mode = if agent.supports(InteractionMode::Cli) {
    InteractionMode::Cli
} else {
    InteractionMode::Api
};
```

`AgentRuntimeApplicationService` already holds `self.ports.registry` (used elsewhere in the same file, e.g. `set_auto_approve_tools`) — no new dependency needed here, unlike Decisions 2 and 3.

### Decision 5: trust is validated at definition-save and run-start, not re-validated per iteration

`validate_agent` runs once when a Loop definition is saved and once when a run starts (`validate_start`). It does not run again before each individual Worker/Verifier iteration within an already-started run. A user who disables an agent's trust flag mid-run (a deliberate, active action taken while a run happens to be in flight) could cause a *later* iteration to stall exactly like the pre-existing untrusted-agent stall case, rather than being caught up front.

**Alternative considered**: re-check trust inside `loop_worker.rs`/`loop_verifier.rs` on every iteration launch, which would require also giving those services `ApiAgentGateway` access (on top of the `AgentRegistryRepository` access Decision 3 already adds) — a second new dependency per service, for a narrow edge case (mid-run trust toggling) that requires deliberate user action to trigger, is already recoverable (the existing `cancelled`-responsive `await_approval` wait means the run can still be cancelled, same as any other stall), and has a real precedent for the chosen behavior: CLI Profile's own approval configuration is likewise read at session/turn start, not continuously re-polled during execution. Rejected as disproportionate to the risk for this phase.

## Risks / Trade-offs

- **[Risk] Mid-run trust revocation stalls a later iteration** (Decision 5) → Mitigation: this requires a deliberate, active user action (turning off trust) while a run happens to be in flight; the resulting stall is recoverable the same way any approval stall is today (cancel the run). Not solved further in this phase.
- **[Risk] MCP-bound Loop-role agents stall on MCP calls even when trusted** (unconditional MCP gating, unchanged from `add-agent-tool-trust`) → Mitigation: documented as a known limitation (proposal Non-Goals, and user-facing copy in tasks.md's UI-facing task if one is warranted); no enforcement added to actively block this configuration, per explicit decision — it's a real but narrow gap, not silently hidden.
- **[Risk] Four port structs (`LoopApplicationPorts`, `LoopWorkerApplicationPorts`, `LoopVerifierApplicationPorts`, plus the `bootstrap` wiring) change shape** → Mitigation: every affected test file already uses this codebase's established single-`FakeWorld`-implements-every-port-trait convention (confirmed in `loop_service_tests.rs`); adding `ApiAgentGateway`/`AgentRegistryRepository` impls to the relevant `FakeWorld`s is mechanical and follows an existing pattern, not a new one.
- **[Risk] Reordering the `bootstrap/agent_runtime.rs` `repository` move to a clone (Decision 2/3)** → Mitigation: `Arc::clone` is cheap and this file already clones the same `repository` value into multiple port structs elsewhere; the only change is making the currently-last usage (line 217) a clone like its siblings instead of a move, since it's no longer the last usage once the two new dependents are added further down the same function.
