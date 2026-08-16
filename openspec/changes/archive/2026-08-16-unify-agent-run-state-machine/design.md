## Context

VaneHub currently has three useful but different foundations: `execution_observability` owns UUID run/trace/span identity and metadata timelines; `operations` owns observable task lifecycle, cancellation flags, logs, and operation-to-run correlation; Session, Plan, Loop, Goal, approval, question, and group-chat contexts own their business state. Their independent enums are intentional, but there is no guarded, persisted state machine that all long-running execution can project onto. This causes ambiguous waiting states, inconsistent cancellation/recovery, and no common UI contract.

The architecture fitness baseline is already present. The change must preserve DDD direction, existing Tauri commands and data, React-to-service isolation, Web/Tauri parity, unified logging, and metadata-only observability. Active Skill changes contain lifecycle concepts but do not provide or own the canonical substrate.

## Goals / Non-Goals

**Goals:**

- Make `operations` own a reusable canonical Run aggregate and published application facade.
- Reuse `ExecutionRunId` identity and correlate, rather than merge, execution telemetry and lifecycle persistence.
- Express all required states, guarded transitions, waiting reasons, retry limits, verification, recovery, cancellation, hierarchy, timestamps, and safe events.
- Add additive persistence and conservative restart reconciliation.
- Project the existing Session generation, Plan, Loop, Goal, approval, question, and group-chat execution boundaries without replacing their domain models.
- Provide additive frontend contracts and a compact localized status component with Web/mock parity.

**Non-Goals:**

- Mission Control, evaluation scoring, background runners, or provider plugin APIs from roadmap items 05 and later.
- Combining existing Session, Plan, Loop, Goal, message, operation, or observability tables.
- Replaying external processes, tool calls, approvals, questions, or destructive actions after restart.
- Breaking existing commands, DTO fields, service calls, or stored records.

## Decisions

### 1. `operations` owns canonical lifecycle; observability owns telemetry

`operations` already publishes lifecycle/cancellation/logging contracts across contexts, so it gains `AgentRun`, `RunState`, `RunEvent`, repository ports, and `AgentRunsApi`. `execution_observability` remains the owner of trace topology and capture/export. Both use the same canonical UUID run id through a narrow published identity contract; lifecycle events are projected to telemetry asynchronously.

Alternative: expand `execution_observability` into an orchestration engine. Rejected because exporter/timeline health must not control business execution. Alternative: add a `runs` context. Rejected as a duplicate of `operations` ownership.

### 2. Canonical states and guarded transition commands

The persisted states are `created`, `preparing`, `running`, `waiting_approval`, `waiting_user`, `paused`, `retrying`, `blocked`, `stuck`, `verifying`, `completed`, `failed`, and `cancelled`. Completed, failed, and cancelled are terminal. State changes accept an explicit trigger, timestamp, bounded reason code, and optional retry count; direct status mutation is private.

The transition table is closed and table-tested. Waiting approval leaves only through approval, rejection/failure, or cancellation. Waiting user leaves only through answer, cancellation, or interruption. Verification may complete, fail, cancel, or enter a bounded repair retry. Terminal transitions are idempotent only when the requested terminal state and event witness match; other terminal mutations fail.

Alternative: reuse `OperationStatus`. Rejected because it cannot distinguish waiting, pause, retry, blocked, stuck, or verification and changing it would break broad settings operations.

### 3. Owners project; canonical Run does not orchestrate owners

Each Run stores `owner_type`, `owner_id`, optional session/plan/loop/goal references, optional parent id, recovery policy, retry policy, and safe timestamps/reason. Owning application services call `AgentRunsApi` at meaningful boundaries and retain their existing invariants. The Run service does not import owner repositories or private aggregates.

PlanRun maps to a parent Run, SubTaskRun/Attempt to children. LoopRun maps to one Run and keeps its phases. Agent generations map to Runs linked to messages and operations. Group-chat delegated execution maps to children while Seat/turn routing remains unchanged. Goal links are informational and do not alter goal acceptance rules.

### 4. Cancellation is monotonic and hierarchy-aware

Cancellation sources are user, parent, timeout, and shutdown. The Run application service atomically records cancellation intent and terminal state before signaling the existing cooperative cancellation handle. Parent cancellation cascades to all non-terminal descendants; already terminal children remain unchanged. Late completions, approvals, questions, and tool starts are rejected. Repeated cancel/terminal delivery returns the existing outcome without duplicating an event.

Resume is allowed only for paused, blocked, stuck, or an owner-approved interrupted Run, and requires the owner runtime to confirm it can resume. Waiting approval/user and retrying are not general manual-resume states.

### 5. Additive forward-only SQLite persistence

New `agent_runs` and `agent_run_events` tables store bounded lifecycle metadata, optimistic version, owner links, recovery policy, retry counters, and event witnesses. Foreign keys do not target every legacy owner table because owners have different retention rules; indexed typed links are validated by applications. The migration does not rewrite legacy Session/Plan/Loop/Goal rows.

The migration is transactional, idempotent, and forward-only. Rolling back the binary leaves new tables inert and preserves all legacy behavior; re-upgrading reuses them. No down migration deletes execution evidence.

### 6. Conservative recovery

Startup scans only non-terminal Runs. Terminal Runs remain unchanged. Recoverable owners receive a bounded reconciliation request and must produce resumed, paused, blocked, failed, or cancelled evidence. External CLI/API generations without a durable resumable handle become failed with reason `interrupted_restart`; pending approvals/questions are invalidated; no external or destructive action is replayed. Recovery uses optimistic version/event witnesses so repeated startup is idempotent.

### 7. Additive service and UI contract

`AgentService` gains get/list/cancel/resume Run methods and typed Run views. `tauri-agent-client.ts` alone invokes additive commands. `web-agent-client.ts` uses the same transition reducer and deterministic mock clock/fixtures without claiming native recovery or process effects. Adapter contract tests compare state/reason/action semantics.

`AgentRunStatus` is a reusable presentational component using semantic tokens and i18n. It renders badge, elapsed time, explicit waiting reason, retry count, and only allowed actions. Existing chat/Plan/Loop surfaces embed it at their current execution summary; no new dashboard is added. Visual tests cover futuristic/minimal at desktop/narrow widths.

### 8. Safe events and bounded performance

Events are `run_created`, `run_started`, `run_waiting`, `run_resumed`, `run_retrying`, `run_verifying`, `run_completed`, `run_failed`, `run_cancelled`, and `run_stuck`. They contain ids, enum classifications, timestamps, sequence/version, and bounded reason codes only. Application diagnostics go through the operations logging port; raw prompts, outputs, tool payloads, credentials, paths, and errors are excluded.

Indexes support owner, parent, status/update time, and ordered event lookup. Deterministic benchmarks assert bounded query counts, pagination, event/reason sizes, and transition-table complexity rather than fragile shared-runner milliseconds.

## Risks / Trade-offs

- [Dual lifecycle projections drift] → Keep owning state authoritative, update canonical Run through one application boundary, test mapping tables, and surface reconciliation failures without inventing success.
- [Cancellation races with completion] → Use optimistic versions, one atomic terminal write, idempotency witnesses, and reject late effects.
- [Recovery accidentally replays unsafe work] → Resume only from owner-provided durable evidence; never infer resumability from a pre-crash running state.
- [Cross-context coupling grows] → Publish narrow immutable Run contracts from `operations`; owners never access Run infrastructure and Run code never accesses owner repositories.
- [Large rollout destabilizes existing paths] → Add tables, commands, fields, and projections incrementally; preserve legacy contracts and enable owner integrations in task order.
- [UI becomes an early Mission Control] → Limit the component to one Run summary and actions already supported by the owning surface.

## Migration Plan

1. Land domain/application contracts and deterministic tests with no runtime integration.
2. Add transactional schema and repository/recovery tests; legacy data remains untouched.
3. Add native API/commands/composition and Web/Tauri frontend contracts.
4. Integrate generation, approval/question, Plan/Loop, hierarchy, and Goal projections in that order with compatibility tests after each owner.
5. Add minimal UI and complete visual/desktop verification.
6. Rollback by deploying the previous binary; it ignores additive tables. Preserve new tables/events for later re-upgrade and never replay or delete them automatically.

## Open Questions

None. Provider-specific resumability remains an owner policy and future background-runner/evaluation details remain explicitly deferred.
