## Context

The archived `add-plan-execution-foundation` change established versioned Plan drafts, immutable PlanRun snapshots, serial DAG scheduling, attempt-scoped OnePiece sessions, guarded command verification, retained integration worktrees, controls, recovery, and matching frontend adapters. The implementation currently exposes `execute_next_plan_attempt` as a synchronous command and the Plan UI calls it once per button press. Polling refreshes projections but does not advance execution.

Planning currently makes one tool-less OnePiece request containing the user goal and available tool names. Acceptance criteria are stored as strings beside optional validation commands. Verification considers all required commands successful when the command list is empty, and retry resets a failed SubTask without including prior failure evidence in the new attempt context.

The existing boundaries remain authoritative: `task_orchestration` owns Plan state, `agent_runtime` owns OnePiece provider/tool execution, workspaces own Git worktrees and status, operations own guarded validation, sessions own session identity, and React reaches them only through runtime-neutral services.

## Goals / Non-Goals

**Goals:**

- Give OnePiece a bounded read-only discovery phase that produces a project-aware Plan.
- Make Plan versus Agent capability state unmistakable without conflating permission mode with PlanRun phase.
- Let approved runs continue natively without a foreground React driver.
- Bind textual criteria to retained evidence and reject vacuous verification.
- Repair eligible failures with bounded evidence and immutable attempt history.
- Verify the integrated worktree before final user acceptance.
- Preserve restart safety, cancellation, adapter parity, privacy, and existing records.

**Non-Goals:**

- Dynamic mutation of an approved DAG, automatic replanning, or insertion of new SubTasks.
- Parallel SubTask execution, per-worker worktrees, commit-based integration, or conflict resolution.
- Automatic commit, merge, push, target-branch application, deployment, or worktree deletion.
- Using unrestricted shell, MCP, arbitrary network, or write tools during Plan discovery.
- Treating an LLM opinion alone as proof that deterministic checks passed.
- Changing non-OnePiece permission semantics or replacing the existing Loop engine.

## Decisions

### 1. Keep permission mode, planning artifact state, and execution phase separate

The shared chat configuration continues to own permission modes such as `plan`, `agent`, `default`, and `auto`. Task orchestration separately projects Plan lifecycle phases such as discovery, draft review, awaiting approval, preparing, running, verifying, repairing, paused, action required, final verification, and awaiting acceptance.

The UI composes those dimensions into accessible labels such as `Plan · read-only`, `Agent · running`, and `Agent · verifying`. The mode selector remains in the composer, while Plan Center remains the artifact and evidence view. Color is supplementary; icon, text, accessible name, and explanatory copy carry the semantics.

`verifying` and `repairing` are durable, first-class PlanRun states rather than labels inferred only from the latest SubTask. The driver enters `verifying` while guarded SubTask checks are active and `repairing` while an eligible follow-up OnePiece Attempt is dispatching or running. Recovery, pause, and cancellation therefore observe the same phase in SQLite, native projections, and both frontend adapters.

When a reviewed draft exists, leaving Plan mode through the Plan execution path opens an approval surface rather than silently changing a selector value. During an active attempt, a request to return to planning first persists pause intent and waits for the safe boundary. This avoids presenting a mode that disagrees with native capabilities.

Task orchestration also stores an optional originating OnePiece session id with the approved PlanRun snapshot. It is an opaque cross-context reference, not a foreign-key ownership transfer: sessions continue owning session lifecycle, while task orchestration owns the association and exposes bounded lookup through the Plan service. Attempt-scoped execution sessions remain separate and never replace the originating-session association. This single association drives both pause-before-planning and navigation to Plan Center.

An alternative top-level Plan application separate from chat was rejected because it duplicates mode state and makes the approval transition hard to understand. Plan Center remains reachable as a panel or route without becoming a permission mode.

### 2. Use a dedicated read-only OnePiece discovery profile

Planning becomes a bounded Agent generation rather than an unrestricted tool-less completion. Task orchestration creates a planning discovery request rooted at the canonical project and captures the active OnePiece Profile reference. Agent runtime constructs an explicit catalog containing only workspace-bounded file reads, `grep`, `glob`, available `search_code`, and configured trusted read-only language-intelligence queries.

The profile excludes shell, file mutation, MCP, memory mutation, arbitrary network tools, and all out-of-root operations. It applies independent tool-call, token, context-character, and wall-clock limits. The final response must match the strict Plan JSON schema and includes limitation metadata indicating unavailable indexing or exhausted discovery budget.

This reuses the existing tool loop and Plan-mode enforcement while making the narrower orchestration catalog authoritative. A native static file-tree snapshot alone was rejected because it cannot iteratively inspect the files relevant to an unfamiliar goal. Unrestricted Plan-mode sessions were rejected because they currently include capabilities, such as memory writes, that are unnecessary for planning.

### 3. Add evidence bindings without replacing stored criterion text

Existing `acceptance_criteria` JSON remains readable. Additive tables associate each criterion ordinal in a Plan version with an evidence kind and optional validation-command id. Supported initial kinds are:

- `automated`: requires a named guarded validation command to pass.
- `manual`: requires an explicit user evidence decision and cannot silently pass.

Each new or edited required SubTask must have at least one required automated validation command, even when it also contains manual criteria. Plan versions also store structured final validation commands. Commands retain the existing program-plus-argument-array contract; shell strings remain invalid.

Plan approval writes an execution-policy snapshot containing criterion bindings, final commands, maximum attempts, eligible repair classes, discovery limitations, and the non-secret Profile reference. Existing completed runs remain queryable. Existing drafts without bindings remain legacy drafts and must be upgraded and revalidated before new approval.

Replacing criteria strings with a new nested JSON shape was rejected because it complicates migration and silently changes archived version decoding. Treating every textual criterion as model-evaluated was rejected because it would turn subjective output into proof of command success.

### 4. Make a native singleton driver own continuous execution

`start_plan_run` continues preparing the retained integration worktree but also persists desired execution intent and requests driver activation. It returns after durable preparation and activation rather than blocking for the full Plan.

A native `PlanDriverRegistry` holds at most one cancellation token and worker handle per PlanRun. The worker repeatedly:

```text
reconcile durable control/recovery state
  -> transactionally schedule_next
  -> create and execute one Attempt
  -> verify and persist evidence
  -> project run state
  -> continue only when durable state remains runnable
```

SQLite remains authoritative. The registry is an optimization and cancellation bridge, not the source of truth. Existing compare-and-set claims prevent duplicate dispatch if startup and a command race. Blocking provider or validation work runs outside the Tauri command handler so the frontend and unrelated commands stay responsive.

Driver transitions persist `running -> verifying` before guarded SubTask commands and `verifying -> repairing` when an eligible repair Attempt is claimed. A successful repair returns through `verifying`; terminal or exhausted outcomes move to the existing action-required, recovery-required, cancelled, or failed boundaries. These states do not authorize new work by themselves: persisted desired intent and transactional claims remain authoritative.

Startup first applies shared session recovery evidence, then activates only conclusively runnable PlanRuns. Ambiguous in-flight work remains `recovery_required`; it is never automatically replayed. Pause and cancel intent is persisted before signaling the worker. The worker stops at the same safe boundaries already defined by the foundation.

Driving the loop from React polling was rejected because closing or reloading the view would stop execution. A global interval scanning every PlanRun was rejected in favor of event-driven per-run activation plus bounded startup reconciliation.

### 5. Classify failures before automatic repair

The approved policy contains `max_attempts_per_subtask`, defaulting to three, and an allowlist of repair-eligible classes. The initial automatic class is `verification_failed`; explicitly classified complete-but-invalid Agent output may also be eligible. Cancellation, safety rejection, missing credentials, invalid policy, timeout with ambiguous filesystem effects, and inconclusive recovery are never automatically retried.

A repair creates a new Attempt and OnePiece session in the retained worktree. Its context contains:

- current task identity, description, criteria, and limits;
- failed command ids and bounded redacted output summaries;
- previous attempt outcome and changed-file summary;
- direct predecessor evidence already allowed by the foundation;
- attempt sequence and remaining budget.

It excludes raw transcripts, credentials, tool arguments/results, and unrelated historical attempts. Prior attempts and evidence are immutable. After budget exhaustion, descendants remain blocked and the PlanRun exposes action-required controls. Independent eligible branches may still finish before the driver reaches that boundary.

Reusing the same failed session was rejected because it blurs attempt identity and recovery evidence. Unlimited self-repair was rejected for cost, safety, and nontermination reasons.

### 6. Add a separate finalization aggregate for integrated verification

After every required SubTask succeeds, the driver creates a Plan finalization run rather than moving directly to `awaiting_acceptance`. It executes the snapshotted final validation commands through the guarded operation boundary and records evidence separately from SubTask evidence.

If final checks fail with eligible budget remaining, task orchestration may create a final-repair Attempt and OnePiece session rooted at the same integration worktree. This attempt receives the Plan goal summary, changed-file summary, failed final-check evidence, and final-repair limits, but it does not create or mutate a SubTask or DAG edge. Final checks run again after the repair.

Additive finalization and final-repair records avoid fabricating a hidden SubTask in the approved graph. Only passing required final checks allows `awaiting_acceptance`. User acceptance remains required for `completed`.

### 7. Extend services for observation and durable controls, not scheduling

The Plan service gains discovery metadata, criterion bindings, final command editing, approved retry policy, driver state, repair chain, finalization evidence, and mode-transition details. `startPlanRun` returns a prepared/running projection. The public UI no longer needs `executeNextAttempt`; the underlying operation may remain temporarily internal for driver tests and compatibility until callers migrate.

Plan generation and approval may carry an optional originating OnePiece session id. Run detail exposes that id, and a bounded association lookup returns the current linked PlanRun summary for a session. The composer uses this service boundary to request a durable pause and wait for a safe paused boundary before changing an active linked session to Plan mode; it also uses the same association to navigate to Plan Center. Web/mock stores the association in memory and simulates identical transitions without claiming durability.

The Tauri adapter invokes declared commands or subscribes to bounded updates. The Web/mock adapter runs the same deterministic state machine over simulated attempts and clearly marks them simulated. It does not create paths, run providers, invoke commands, or imply durable SQLite state.

Polling remains an acceptable bounded projection fallback, but polling never becomes execution authority. React components keep async errors, loading states, keyboard focus, and status announcements local to the view while all runtime mutations cross the service boundary.

### 8. Extend redacted correlation for discovery, repair, and finalization

Diagnostics add event families for driver activation/stop, schedule claim, discovery limit, repair decision, repair exhaustion, final verification, and action-required transition. Records contain stable ids, sequence, phase, safe failure class, counts, durations, and non-reversible fingerprints.

User-facing evidence may expose bounded approved command summaries through the Plan service. Unified logs and execution telemetry continue excluding user goals, task descriptions, prompts, credentials, raw tool payloads, and unredacted command output.

## Risks / Trade-offs

- **[A background driver can continue spending tokens after the user leaves the view]** → Require explicit Plan approval, snapshot strict token/tool/time/attempt limits, expose pause/stop globally, and stop at action-required boundaries.
- **[A repair works on partial changes left by the failed attempt]** → Include changed-file and failure evidence, retain the worktree for inspection, cap attempts, and never claim rollback occurred.
- **[Read-only discovery can still expose sensitive source text to the configured provider]** → Reuse workspace boundaries, sensitive-file denial, redaction, explicit Profile configuration, and bounded context; do not add arbitrary network tools.
- **[Required commands can encourage planners to invent invalid validation]** → Make commands editable before approval, validate executable shape and working directory, and surface execution-policy errors rather than weakening verification.
- **[Manual criteria interrupt autonomy]** → Keep them explicit and rare; the UI shows exactly which criterion needs evidence instead of pretending it passed.
- **[Driver activation races after restart]** → Treat SQLite claims and recovered session evidence as authoritative and make the in-memory registry idempotent.
- **[New states increase adapter and migration surface]** → Use additive columns/tables, central parsers, shared contract fixtures, and preserve legacy read projections.

## Migration Plan

1. Add criterion-binding, PlanRun policy-snapshot, driver-intent, finalization, final-repair, first-class verifying/repairing states, and optional originating-session association persistence with idempotent migrations and legacy read tests.
2. Extend domain validation and repositories while leaving the existing manual execution entry point available to tests.
3. Add bounded OnePiece discovery and repair profiles behind Agent runtime APIs.
4. Add the native driver registry, activation, continuous loop, controls, and startup recovery; keep the feature entry disabled until service contracts are ready.
5. Extend Tauri and Web/mock Plan adapters and shared contracts, then update mode presentation, approval, editor, progress, repair, and final evidence UI.
6. Remove the user-facing execute-next control and enable driver activation after desktop/Web conformance and end-to-end tests pass.
7. On rollback, stop new driver activation and restore the manual progression UI; additive records and retained worktrees remain inspectable.
