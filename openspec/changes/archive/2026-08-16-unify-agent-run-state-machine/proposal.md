## Why

VaneHub already assigns execution identities and persists lifecycle evidence, but Session generations, operations, Plans, Loops, approvals, questions, and child work still expose incompatible local state vocabularies. A canonical Run lifecycle is needed now so cancellation, recovery, verification, UI status, and future review/evaluation features can correlate the same execution without replacing each bounded context's business model.

## What Changes

- Extend the existing `operations` bounded context into the canonical lifecycle substrate for observable Runs while reusing `execution_observability` run and trace identities.
- Define guarded canonical states, transition triggers, terminality, recoverability, timestamps, bounded reason codes, retry policy, and safe lifecycle events.
- Persist canonical Run snapshots, append-only transition events, owner links, and parent/child relationships through an additive forward-only SQLite migration.
- Add restart reconciliation that preserves terminal Runs, delegates recoverable work to the owning runtime, and marks non-recoverable live work interrupted without replaying destructive actions.
- Add unified cancellation and resume contracts, including idempotent terminal handling and explicit parent-to-child propagation.
- Project Session generation, Plan, Loop, approval, user-question, and verification boundaries onto canonical Runs while retaining their existing commands, serialized models, routing, and domain-specific states.
- Extend the shared frontend service boundary and both Tauri and Web/mock adapters with compatible Run query/control contracts.
- Add a minimal localized Run status presentation for status, elapsed time, waiting reason, retry count, and allowed cancel/resume actions; this change does not implement Mission Control or evaluation scoring.

## Capabilities

### New Capabilities

- `agent-run-state-management`: Defines canonical Run identity, lifecycle invariants, events, persistence, ownership, hierarchy, cancellation, recovery, verification, adapter parity, and minimal status presentation.

### Modified Capabilities

- `agent-execution-observability`: Makes the existing execution run identity the canonical correlation identity and records canonical lifecycle transitions without conflating telemetry status with business state.
- `session-runtime-management`: Projects accepted Agent generations and their waits, verification, cancellation, and terminal outcomes onto canonical Runs while preserving Session and message semantics.
- `session-recovery`: Reconciles canonical Runs conservatively during startup and records interrupted outcomes for non-recoverable processes.
- `plan-execution-runtime`: Links Plan, SubTask, Attempt, verification, pause, recovery, and cancellation lifecycle to parent/child canonical Runs.
- `loop-engineering-runtime`: Links Loop execution and verification boundaries to canonical Runs while preserving Loop phases and acceptance semantics.
- `goal-management`: Allows Goals to link to canonical Runs without making Session links participate in goal completion derivation.
- `multi-agent-group-chat`: Correlates delegated child execution with parent/child Runs while retaining Seat and turn routing semantics.
- `permissions-approval`: Projects approval waits and decisions as distinct canonical Run transitions and rejects late decisions after terminal cancellation.
- `agent-user-question`: Projects interactive question waits as a distinct canonical state and clears them on cancellation or restart.

## Impact

- Native: `operations` domain/application/infrastructure/API, composition root, database migration, and adapters in `agent_runtime`, `sessions`, and `task_orchestration`; no new bounded context.
- Frontend: additive `AgentService` Run contracts, matching Tauri/Web implementations, localized reusable status UI, and existing chat/Plan/Loop integration points.
- Compatibility: existing Tauri command names, Session/Plan/Loop/Goal tables and DTOs remain readable; new commands and fields are additive. The migration is forward-only and rollback is performed by running the previous binary against untouched legacy tables while the new Run tables remain inert.
- Security/privacy: lifecycle events contain stable ids, bounded classifications, and timestamps only; raw prompts, tool payloads, credentials, unrestricted paths, and destructive-action replay are prohibited.
- Dependencies: reuses existing UUID/clock, SQLite, unified logging, operation cancellation, execution observability, permissions, questions, and service adapters; introduces no replacement state library or runtime framework.
