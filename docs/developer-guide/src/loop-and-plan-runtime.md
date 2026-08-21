# Loop runtime and session Plan mode

VaneHub has one durable native runtime for autonomous iterative work: the **Loop** runtime. **Plan** is a read-only execution mode inside an eligible OnePiece session; it is not a second durable task-orchestration runtime. The user-facing workflows are covered in the user guide, while this chapter describes the native ownership boundary.

## Loop runtime

A Loop definition is persisted with a stable id, name, enabled state, local Git project path, base branch, goal, acceptance criteria, allowed and protected paths, stable Worker and Verifier Agent ids, structured verification commands, stop limits, version, and timestamps. Loop definitions preserve **stable Agent ids** rather than matching display names.

First-phase scope is constrained: a definition targeting a non-Git project, a remote workspace, a missing Agent, an unsafe path scope, or an invalid limit is rejected without starting an Agent or creating a worktree. The Worker and Verifier roles accept either a CLI-launched Agent or an API Agent with tool-use trust enabled; an API Agent without tool-use trust is rejected.

## OnePiece session Plan mode

An eligible OnePiece session can switch its composer between Plan and Agent modes. Plan mode persists as `executionMode: "plan"` on the session chat configuration and resolves to a read-only effective policy. It keeps read-only exploration tools while excluding shell execution, file writes, effectful MCP tools, and delegated work.

The interactive `exit_plan_mode` request asks the user before a later turn can use Agent mode. Declining leaves the session in Plan mode. Approval changes only the session execution mode; it does not create a Plan definition, PlanRun, task graph, or worktree.

Historical Plan and PlanRun database rows remain available for migration compatibility and audit. The forward retirement migration terminalizes active legacy records and removes Plan-derived Work Board links without deleting recorded history or filesystem worktrees.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/loop-engineering-runtime](../../../openspec/specs/loop-engineering-runtime/spec.md) — durable Loop definitions and the Worker/Verifier trust contract.
- [openspec/specs/session-chat-configuration](../../../openspec/specs/session-chat-configuration/spec.md) — persisted OnePiece session Plan mode.
- [openspec/specs/agent-plan-exit-request](../../../openspec/specs/agent-plan-exit-request/spec.md) — interactive Plan-mode exit behavior.

Loop execution lives in the `agent_runtime` bounded context. OnePiece Plan mode is owned by the `sessions` and `agent_runtime` boundaries; see [Native bounded contexts](native-contexts.md).
