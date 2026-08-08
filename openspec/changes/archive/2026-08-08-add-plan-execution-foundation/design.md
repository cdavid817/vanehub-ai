## Context

Loop Engineering Runtime already provides durable run state, role sessions, verification commands, cancellation, and one isolated worktree, but its phases are fixed runtime lifecycle stages rather than user-defined tasks. GroupChat provides persistent, serially routed Seats and therefore is not an execution scheduler or an ephemeral Worker pool. OnePiece can create API sessions and execute tools, but it has no durable Plan aggregate or dependency scheduler.

This change introduces task orchestration as a separate domain. It must preserve the Rust modular-monolith boundaries, keep SQLite and Git operations native, expose all frontend behavior through matching adapters, and obey unified logging and execution-observability redaction rules.

The foundation intentionally executes only one SubTask at a time. It nevertheless persists a real dependency graph and immutable run snapshot so later parallel scheduling does not require a model rewrite.

## Goals / Non-Goals

**Goals:**

- Generate a bounded, editable Plan draft from a user goal through OnePiece.
- Require explicit approval before any Agent session or worktree is created.
- Persist a versioned Plan definition separately from immutable PlanRun execution snapshots.
- Execute eligible SubTasks in deterministic topological order with concurrency fixed to one.
- Give every attempt a separate OnePiece session while sharing one isolated Plan integration worktree.
- Persist verification evidence, bounded predecessor summaries, resource usage, controls, and restart-safe state.
- Present equivalent typed contracts in desktop and Web/mock runtimes without pretending that Web/mock performs native Git or provider work.

**Non-Goals:**

- Parallel Worker execution, per-Worker worktrees, result commits, candidate integration, or conflict resolution.
- Reusing GroupChat Seats as Workers or mapping Loop lifecycle phases to SubTasks.
- Guard LLM evaluation, periodic checkpoints, mid-stream correction, rollback automation, or dynamic replanning.
- Automatically committing, merging, pushing, removing, or applying the Plan result to a target branch.
- Supporting planners other than the configured built-in OnePiece Agent in this first change.

## Decisions

### 1. Own Plan execution in a new `task_orchestration` bounded context

The native layer will add a `task_orchestration` context that owns Plan definitions, versions, dependencies, run snapshots, SubTask runs, attempts, scheduling, and status projection. It will consume only published APIs from Agent runtime, sessions, workspaces, operations, observability, and unified logging.

This keeps Plan lifecycle invariants out of `agent_runtime`, whose responsibility remains provider execution, and avoids overloading Loop or GroupChat with incompatible meanings. A direct Plan-to-LoopPhase mapping was rejected because Loop phases describe the engine lifecycle and are not arbitrary dependency nodes.

### 2. Separate editable Plan versions from immutable execution snapshots

The persistence model will use these logical records:

```text
Plan
 └─ PlanVersion
     ├─ SubTaskSpec
     └─ SubTaskDependency

PlanRun (approved PlanVersion snapshot)
 └─ SubTaskRun
     └─ SubTaskAttempt
```

`Plan` owns identity and draft lifecycle. Each accepted planner result or user edit creates or updates a draft `PlanVersion`; dependency edges exist only in a normalized dependency table. Approval atomically freezes the selected version into a `PlanRun` snapshot. Later edits cannot mutate a running snapshot.

This is more data than storing a JSON blob, but it makes graph validation, recovery, status queries, and future amendments explicit. Duplicating both plan-level edges and `depends_on` arrays was rejected because the copies can diverge.

### 3. Validate planner output before persistence or execution

The planner will call the active OnePiece Profile with a versioned system instruction containing the goal, available execution-tool descriptions, maximum SubTask count of ten, single-session granularity guidance, and the required structured response schema. Planner generations execute no tools.

The native boundary will strictly parse and validate the response: stable local IDs, 1–10 SubTasks, one to three acceptance criteria per SubTask, references to existing nodes, no self-edge, no duplicate edge, and an acyclic graph. Invalid output remains a failed draft-generation attempt with an actionable error; it never starts execution.

OnePiece credentials stay in the existing Profile credential store. Planner and Worker requests capture the active Profile reference and generation configuration; credentials are not copied into Plan tables, prompts, sessions, logs, or telemetry.

### 4. Make the native scheduler the sole execution authority

The LLM decides what tasks and dependencies to propose. The native scheduler decides whether a node is eligible and when it may run. A node is eligible only when every predecessor has succeeded and its evidence is persisted.

Eligible nodes are ordered by topological rank, Plan ordinal, then stable SubTask ID. The first release sets both per-run and effective Worker concurrency to one. A transactional compare-and-set claim changes one node from `pending` to `dispatching`, preventing duplicate dispatch after overlapping ticks or restart recovery.

SubTaskRun states are:

```text
pending -> ready -> dispatching -> running -> verifying -> succeeded
                                  |             |
                                  +-----------> failed

Exceptional terminal/projection states: cancelled, interrupted, blocked, skipped
```

PlanRun states are:

```text
draft-free execution lifecycle:
queued -> preparing -> running -> awaiting_acceptance -> completed
                       |   |              |
                       |   +-> failed     +-> failed
                       +-> pause_requested -> paused
                       +-> cancel_requested -> cancelled
restart ambiguity ------------------------> recovery_required
```

Independent branches may continue after one node fails; only descendants of an unreplaced failed node become blocked. The PlanRun becomes `failed` when no runnable work remains and at least one required node failed or is blocked. It becomes `awaiting_acceptance` only when all required nodes succeeded.

### 5. Use distinct sessions and attempts in one retained integration worktree

After approval, the workspaces context creates one collision-safe Plan branch and sibling worktree from a recorded base OID. All SubTask attempts use that canonical worktree as their bounded session root. Because concurrency is one, no two attempts mutate it simultaneously.

Each dispatch creates a new `SubTaskAttempt` and a distinct OnePiece API Agent session. Retry creates another attempt and session rather than rewriting prior evidence. Sessions and attempts remain queryable for audit even after the provider process is released.

The runtime does not create commits, merge the Plan branch, push it, remove its worktree, or modify the user's target branch. The retained worktree is the reviewable output. Parallel changes will later replace shared mutation with result commits and candidate integration without changing Plan identity or DAG semantics.

### 6. Transfer bounded structured context, not transcripts

Each successful SubTask stores a bounded `result_summary`, changed-file summary, validation summary, and usage metadata. A dependent attempt receives only its task specification plus the summaries of direct predecessors, ordered deterministically and truncated to its context budget.

Raw predecessor transcripts, prompts, tool arguments, tool results, and secrets are excluded. When summaries exceed the budget, the runtime preserves task IDs, outcomes, acceptance evidence, and truncation metadata before optional descriptive detail.

### 7. Verify every SubTask before releasing dependants

After the Worker generation stops successfully, the runtime executes the SubTask's declared validation commands through the guarded operation boundary. Exit codes, bounded output summaries, changed-file metadata, and timestamps become verification evidence.

A SubTask reaches `succeeded` only when every required acceptance check passes. Dependants are not released merely because the Agent session completed. Command execution errors, failed checks, timeout, cancellation, and incomplete provider output create explicit attempt outcomes rather than being collapsed into one generic failure.

### 8. Persist controls and recover conservatively

Pause and cancel requests are durable before an in-memory signal is delivered. Cancellation stops starting new nodes and requests cancellation of the active generation/operation. Pause waits for the active attempt to reach a safe terminal boundary and then prevents another claim.

On restart, the scheduler reconciles persisted attempts with sessions and operations. It never assumes an ambiguous in-flight mutation succeeded. An active attempt without conclusive terminal evidence becomes `interrupted`, and the PlanRun enters `recovery_required` until the user retries, cancels, or accepts another explicitly supported recovery action. The runtime never resets or discards the retained worktree automatically.

### 9. Expose summary/detail contracts through runtime adapters

Shared TypeScript contracts will separate list summaries, Plan detail/version graphs, PlanRun detail, and control operations. React components use a Plan service interface. The Tauri adapter alone invokes declared native commands; the Web/mock adapter implements the same methods with deterministic in-memory transitions and an explicit simulated-runtime marker.

The UI will support goal entry, draft generation, graph/list review, SubTask editing, dependency editing, approval, run progress, pause/cancel/retry controls, evidence inspection, and final worktree location. Polling will fetch bounded run projections rather than repeatedly returning full Agent transcripts.

### 10. Correlate execution without expanding sensitive telemetry

PlanRun, SubTaskRun, and Attempt IDs will be attached as redacted correlation attributes to session, operation, and execution-run records. Plan orchestration events use the unified logging service with `error`, `warn`, `info`, and `debug` semantics.

Diagnostic records may contain stable IDs, state transitions, durations, counts, safe filenames, exit classifications, and hashes/fingerprints. They must not contain the user goal, generated descriptions, prompts, credentials, raw tool arguments/results, or unredacted command output by default. User-facing task output remains available through the bounded session/operation presentation path.

## Risks / Trade-offs

- **[Shared worktree can retain partial changes after a failed task]** → Stop scheduling descendants, preserve the worktree for review, expose the failed attempt, and require explicit retry/cancel decisions; do not attempt automatic rollback in this phase.
- **[Serial execution underuses independent DAG branches]** → Persist real dependencies and deterministic eligibility now; add parallel claims and result integration in a separate change.
- **[Planner output can be plausible but unsafe or too coarse]** → Apply strict structural validation, bounded task count, explicit acceptance criteria, and mandatory user approval.
- **[A process crash can leave uncertain filesystem effects]** → Persist transitions before side effects, correlate operations, classify ambiguity as recovery-required, and never silently redispatch.
- **[Context summaries may omit useful details]** → Preserve direct-predecessor evidence and truncation metadata first, while retaining source sessions for user inspection.
- **[Plan UI can become expensive as history grows]** → Use paginated summaries and bounded detail projections rather than embedding full histories in list responses.
- **[New orchestration tables increase migration surface]** → Use additive, idempotent SQLite migrations and keep existing Loop, GroupChat, Agent, and session records unchanged.

## Migration Plan

1. Add the `task_orchestration` context and additive SQLite schema with no backfill from Loop definitions or runs.
2. Add native repositories, domain validation, planner, scheduler, controls, recovery, and published command API behind feature entry points.
3. Add shared frontend contracts plus Tauri and Web/mock adapters before connecting React views.
4. Add Plan review/execution routes and tests, then enable the UI entry point.
5. On rollback, hide/disable the entry point and stop the scheduler; retained Plan tables and worktrees remain reviewable and existing domains continue unchanged.

## Open Questions

No blocking design questions remain for the foundation. Retry cleanup, checkpoint rollback, per-Worker worktrees, integration commits, Guard policy, and dynamic Plan amendments are deliberately deferred to their dedicated changes.
