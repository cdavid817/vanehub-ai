## Context

VaneHub AI already has durable sessions, cancellable Agent generation, local project and worktree support, observable operations, scheduled one-shot session creation, project inspection, unified logs, and parallel Tauri/Web frontend adapters. These pieces expose execution details, but they do not model a reusable goal, bounded iterations, independent verification, durable evidence, stop policy, or human acceptance as one coherent lifecycle.

The first Loop Engineering phase must work for local Git projects in both desktop and Web/mock modes. Desktop runs real Agent processes, Git commands, SQLite persistence, and verification processes. Web/mock mode preserves the same contracts and UI states with deterministic simulated runs. React must use the Agent service boundary, while native orchestration must respect the `agent_runtime`, `sessions`, `workspaces`, and `operations` bounded-context APIs.

## Goals / Non-Goals

**Goals:**

- Let a user define and manually start a bounded engineering Loop for one local Git project.
- Isolate every run in a dedicated worktree and preserve that worktree for review.
- Execute a fixed Worker, deterministic-check, Verifier, and decision cycle with durable per-iteration evidence.
- Use independent, non-activating Worker and read-only Verifier sessions without polluting normal session navigation.
- Enforce hard iteration, elapsed-time, runtime-error, and no-progress limits in native policy.
- Require explicit human acceptance, continuation feedback, or rejection after automated checks pass.
- Persist enough state to inspect all completed work and safely recover an interrupted run after restart.
- Provide equivalent service behavior and representative UI states in Tauri and Web/mock runtimes.

**Non-Goals:**

- Scheduled or event-based triggers, task discovery, remote workspaces, or cross-project runs.
- Parallel Workers, arbitrary role graphs, nested Loops, or a free-form flow editor.
- Automatic commits, pull requests, merges, deployments, worktree deletion, or permission expansion.
- Treating model judgment, token reporting, or chat completion as sufficient proof that a goal is complete.
- Automatically resuming Agent processes after application restart.

## Decisions

### 1. Use a fixed native-owned Loop state machine

A run has a coarse status (`queued`, `running`, `paused`, `awaiting-acceptance`, `succeeded`, `failed`, or `cancelled`), a current phase (`preparing`, `acting`, `verifying`, `deciding`, or `finalizing`), and a separate terminal reason. Native application policy owns all transitions. The first phase does not expose arbitrary graph editing.

This makes recovery and stop behavior testable and keeps the UI focused on evidence. A generic DAG engine was rejected because it would add graph validation, arbitrary node contracts, and recovery semantics before the core feedback cycle is proven.

### 2. Keep Loop aggregates separate from observable operations

`LoopDefinition`, `LoopRun`, `LoopIteration`, and `LoopEvidence` are durable domain records. Each variable-duration worktree, Worker, verification, or Verifier action may create an `OperationTask` associated with the run or iteration. An operation reports one asynchronous action; the Loop aggregate owns cross-action policy and history.

This avoids expanding the common operation model into a workflow database while retaining existing operation status, logs, and polling/event behavior.

### 3. Keep Loop ownership in `agent_runtime` for the first phase

The `agent_runtime` context owns Loop invariants, phase transitions, prompt/context construction, limits, and decisions because these are Agent workflow and generation-lifecycle concerns. It consumes deliberately published contracts from:

- `sessions` for non-activating role-session creation, message execution, completion signals, and usage reads;
- `workspaces` for guarded worktree creation and bounded Git inspection;
- `operations` for observable child operations and semantic diagnostics.

Bootstrap assembles concrete implementations. Commands map DTOs and invoke the assembled Loop application API; they do not perform SQL, Git, or process execution. A new peer context was rejected for the first phase because the language and lifecycle still fit the existing `agent_runtime` ownership. Promotion can be reconsidered if scheduling, discovery, or queue ownership becomes independent.

### 4. Persist definitions, immutable run snapshots, iterations, and evidence

SQLite adds additive tables for `loop_definitions`, `loop_runs`, `loop_iterations`, and `loop_evidence`. A run stores an immutable serialized snapshot of the definition version used at start so later edits cannot change historical interpretation. Evidence rows store a typed evidence kind, status, summary, bounded structured payload, operation id, and timestamps. Large/raw output remains in unified logs and is referenced rather than duplicated without bounds.

Repository methods provide atomic transitions that update the run, current iteration, and terminal reason together. External effects are sequenced after a durable intent transition and before a durable completion transition so restart reconciliation can detect interrupted phases.

### 5. Create one isolated worktree per run

The workspaces API creates a collision-safe `vanehub/loop-<definition>-<run>` branch and sibling worktree from the selected local base branch. Both role sessions use that effective root. The Verifier receives read-only tool policy even though it inspects the same worktree. A worktree is never removed automatically in this phase; the result surface exposes its path for human review.

Reusing the user's project directory was rejected because a cancellation, verifier defect, or failed iteration could mutate unrelated work. Creating a new worktree per iteration was rejected because it would make incremental correction and evidence comparison unnecessarily expensive.

### 6. Use non-activating, hidden-by-default role sessions

Each iteration creates a fresh Worker session and, after deterministic checks, a fresh Verifier session. Sessions carry Loop ownership metadata including run id, iteration id, and role. Their creation does not change the desktop active session. Normal session navigation excludes Loop-owned role sessions by default, but the Loop iteration view can open their transcript, terminal, changes, files, and logs.

Fresh sessions prevent unbounded provider context growth. The persisted Loop memory supplied to each Worker contains the immutable goal, acceptance criteria, current Git state, prior decision summary, failed evidence, user continuation feedback, and remaining limits.

### 7. Execute structured verification commands through a guarded native port

A verification command is stored as `program`, `args`, optional relative working directory, timeout seconds, and required flag. The native layer resolves the working directory under the run root, rejects escaping paths and unsupported executables according to policy, executes without shell concatenation, bounds output, and associates full redacted output with an operation and unified logs.

Raw shell scripts and command chaining were rejected for the first phase because they weaken validation and make the persisted configuration an implicit arbitrary-code launcher. Web/mock mode simulates the same command and evidence contracts.

### 8. Separate deterministic evidence, Verifier advice, and native decision

Required deterministic checks must pass before a run can await human acceptance. The Verifier receives the goal snapshot, acceptance criteria, bounded diff, and check evidence, and returns a structured `pass`, `revise`, or `blocked` recommendation with findings. It cannot mutate files or transition the run.

The native decision policy chooses among next iteration, awaiting acceptance, failed, or limit reached. Model self-approval was rejected because Worker and Verifier output can be incorrect or specification-game the visible checks.

### 9. Detect no progress from objective fingerprints

After each decision, native policy records the Git diff fingerprint and required-check failure fingerprint. A revision round counts as no progress when both fingerprints repeat without new passing required evidence. Reaching the configured consecutive no-progress threshold terminates the run with `no-progress`.

The UI may show Agent summaries, but summaries do not drive this counter because wording changes are not engineering progress.

### 10. Define pause, cancellation, acceptance, and recovery precisely

Pause means finish or reconcile the currently active child operation and do not schedule the next phase. Immediate stop requests cancellation of the active generation or verification process and transitions the run to `cancelled` after reconciliation. Acceptance marks an awaiting run `succeeded`; continuation feedback creates the next iteration if limits permit; rejection cancels the run while preserving evidence and files.

On startup, a nonterminal run with no live in-memory lease becomes `paused` with terminal detail `recovery-required`. The user may resume from the last durable phase boundary or cancel it. Native code does not assume an external child process survived restart.

### 11. Add Loop APIs to the existing frontend service boundary

Loop DTOs live in synchronized `types` and `contracts` modules. `AgentService` exposes definition CRUD, manual start, run reads, pause, resume, cancel, acceptance, continuation, rejection, and event subscription or polling. `tauri-agent-client` contains all Loop `invoke()` calls. `web-agent-client` implements equivalent in-memory behavior and representative asynchronous transitions.

React Query owns server-state caching and React state owns local forms and selection. No additional state-management library is introduced.

### 12. Use a dedicated Loop Center instead of session tabs or a modal

The activity bar switches between the session workspace and a Loop Center. The Loop Center uses a 240px definition/run list, flexible run timeline, and 300px configuration/control inspector. Narrow layouts turn side panels into drawers. Creation uses a four-step dialog: goal and scope, role Agents, verification and limits, then review.

Iteration details deep-link into existing session inspection surfaces rather than duplicating diff, terminal, file, and log renderers. All visible text is localized in zh-CN and en, and both visual styles use shared semantic tokens.

## Risks / Trade-offs

- [Verifier is still a model and can miss defects] -> Required deterministic checks remain authoritative, Verifier is read-only, and final success requires a human.
- [Configured verification commands execute local programs] -> Use structured commands, explicit user review, root confinement, executable policy, timeouts, cancellation, bounded output, and redacted logging.
- [Fresh role sessions increase session and message volume] -> Hide them from normal navigation, retain ownership metadata, and expose them only through Loop iteration details.
- [Worktrees accumulate on disk] -> Preserve them deliberately in phase one and show their paths; add explicit reviewed cleanup in a later proposal.
- [Crash recovery cannot resume an external process exactly] -> Recover only at durable phase boundaries and require user confirmation before resuming.
- [No-progress fingerprints can stop a useful but subtle revision] -> Make the threshold configurable, show the evidence, and allow a human to start a new run or revise the definition.
- [Web/mock cannot perform real Git or Agent work] -> Preserve contract and state-machine parity while clearly marking simulated execution.
- [Loop APIs enlarge AgentService] -> Keep DTOs and implementation modules separated internally; reconsider a dedicated published frontend Loop service only through a later architecture proposal.

## Migration Plan

1. Add contract models and additive SQLite migrations without exposing the UI.
2. Implement and test native domain/application policy, repositories, guarded verification, and cross-context ports.
3. Add Tauri commands and both frontend adapters with conformance tests.
4. Add the Loop Center, creation flow, run timeline, controls, localization, and responsive visual tests behind the new activity destination.
5. Enable manual starts after native and Web/mock end-to-end tests cover success, revision, limits, cancellation, and restart reconciliation.

Rollback removes the UI entry and command registration while leaving additive Loop tables intact so evidence is not destroyed. No existing session, scheduled-task, operation, or worktree data requires destructive migration.

## Open Questions

- Whether a later cleanup capability should archive or delete retained Loop worktrees remains outside this change.
- Provider token reporting is not complete enough to be a mandatory phase-one hard limit; the UI will display available usage while later work can define normalized cost budgets.

