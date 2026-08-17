# Loop and Plan runtimes

VaneHub runs two durable native execution runtimes for autonomous work: the **Loop** runtime (goal + acceptance-criteria-driven iteration against a Git project) and the **Plan** runtime (topology-aware subtask scheduling). Both persist state in SQLite and treat persisted state as authoritative over in-memory scheduler state. The user-facing Loop/Plan workflow is in the user guide; this chapter covers the native design.

## Loop runtime

A Loop definition is persisted with a stable id, name, enabled state, local Git project path, base branch, goal, acceptance criteria, allowed and protected paths, stable Worker and Verifier Agent ids, structured verification commands, stop limits, version, and timestamps. Loop definitions preserve **stable Agent ids** rather than matching display names.

First-phase scope is constrained: a definition targeting a non-Git project, a remote workspace, a missing Agent, an unsafe path scope, or an invalid limit is rejected without starting an Agent or creating a worktree. The Worker and Verifier roles accept either a CLI-launched Agent or an API Agent with tool-use trust enabled; an API Agent without tool-use trust is rejected.

## Plan runtime

A dedicated task-orchestration boundary persists `PlanRun`, `SubTaskRun`, `SubTaskAttempt`, verification evidence, control request, and correlation records. On approval of a valid Plan version, the runtime creates one `PlanRun` snapshot and one pending `SubTaskRun` for every snapshotted SubTask in a single consistent operation. Overlapping scheduler ticks claiming the same ready SubTask are serialized by a transactional compare-and-set transition — at most one dispatch attempt is created.

### Deterministic topology-aware serial scheduling

The scheduler dispatches only SubTasks whose dependencies succeeded, orders eligible work by topological rank, Plan ordinal, and stable SubTask ID, and runs at most one SubTask attempt per `PlanRun` at a time in this foundation release. A pending SubTask whose predecessor has not reached verified success is not dispatched. When multiple independent SubTasks are eligible, only the deterministic first is dispatched. A failed required SubTask blocks only its transitive descendants, not independent branches.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/loop-engineering-runtime](../../../openspec/specs/loop-engineering-runtime/spec.md) — durable Loop definitions and the Worker/Verifier trust contract.
- [openspec/specs/plan-execution-runtime](../../../openspec/specs/plan-execution-runtime/spec.md) — the durable execution aggregate and serial scheduler.
- [openspec/specs/plan-management](../../../openspec/specs/plan-management/spec.md) — Plan definition lifecycle.

Loop and Plan execution live in the `agent_runtime` bounded context; see [Native bounded contexts](native-contexts.md).
