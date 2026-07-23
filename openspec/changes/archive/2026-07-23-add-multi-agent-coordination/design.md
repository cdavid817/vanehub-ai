## Context

VaneHub's `agent_runtime` owns Agent registry, provider invocation, workflow state, generation lifecycle, and Loop orchestration. Interactive chat currently starts one Agent generation for a session, while Loop engineering already demonstrates the required layering for long-running orchestration: domain invariants, application-owned ports, background execution, SQLite projections, operation lifecycle, and completion callbacks.

Multi-Agent coordination introduces a different aggregate. A coordination plan is a finite DAG of task nodes, not an open-ended Loop. Each node chooses a primary stable Agent id, ordered fallback ids, an instruction, and prerequisite node ids. A run snapshots its plan so later plan edits cannot change an active or historical execution.

The feature affects the Rust runtime, Tauri commands, TypeScript service contract, and Web/mock adapter. React consumers remain unaware of SQLite, process details, or provider-specific invocation.

## Goals / Non-Goals

**Goals:**

- Validate the complete dependency graph before a run is accepted.
- Execute ready nodes in a deterministic topological order with explicit durable state.
- Provide bounded, provenance-preserving prerequisite output as downstream context.
- Retry a node on its declared fallback Agents only for retryable Agent execution failures.
- Keep attempt history, run/node outcomes, operation correlation, and redacted logs queryable.
- Keep desktop and Web/mock contracts structurally equivalent.

**Non-Goals:**

- Concurrent node execution in the first version; a deterministic sequential scheduler establishes semantics without race-prone persistence.
- Dynamic graph mutation after a run starts.
- Automatic task decomposition, Agent selection, or fallback selection by an LLM.
- Treating cancellation, invalid configuration, policy rejection, or output validation errors as retryable.
- Persisting raw prompts or outputs in observability telemetry or unified diagnostic logs.
- Replacing ordinary Single-Agent sessions or enabling the currently disabled create-session Multi Agent UI in this foundational change.

## Decisions

### 1. Model coordination as a dedicated `agent_runtime` aggregate

`CoordinationPlan` owns graph invariants and `CoordinationRun` owns run, node, and attempt transitions. Stable string ids are validated and normalized at construction. A node contains `id`, `primary_agent_id`, ordered `fallback_agent_ids`, `instruction`, and `depends_on`.

This keeps orchestration rules in the domain instead of React or Tauri commands. Extending the existing singleton workflow was rejected because it cannot represent several node lifecycles or historical attempts. Modeling each node as a separate user-visible session was rejected because session navigation is not a dependency graph.

### 2. Validate with deterministic Kahn ordering

Plan construction verifies non-empty unique node ids, registered unique Agent ids, existing dependency references, no self-dependency, and a DAG. Kahn's algorithm uses lexical node-id ordering whenever several nodes are ready, producing a stable execution order across desktop and Web/mock runtimes.

The first scheduler executes one ready node at a time. Independent branches continue after another branch fails, but every transitive dependent of a failed node becomes `skipped`. The run ends `succeeded` only when every node succeeds, `cancelled` after a cancellation request, and `failed` when any node fails or is skipped.

Sequential scheduling was chosen over immediate concurrency because deterministic failover, cancellation, and crash-safe state transitions matter more than throughput in the first release. The model leaves room for a future bounded concurrency setting.

### 3. Use an application-owned node executor port

The coordination application service depends on a narrow `CoordinationNodeExecutor` port accepting the run/node/attempt identity, stable Agent id, workspace/session scope, assembled instruction, and cancellation token. The native adapter reuses existing Agent registry and provider generation infrastructure; it does not branch on provider inside the coordinator.

The executor returns either a bounded successful output or a classified failure:

- `retryable`: Agent unavailable at attempt time, process start/exit failure, provider transport failure, or timeout.
- `non_retryable`: invalid request, policy/permission rejection, context/output limit violation, or persistence failure.
- `cancelled`: explicit user/runtime cancellation.

Only `retryable` advances to the next declared fallback. Exhausting candidates fails the node. Agent availability is assessed before launch but remains separate from launch.

### 4. Propagate bounded output with explicit provenance

A successful attempt stores a `CoordinationOutput` containing source node id, actual Agent id, attempt number, UTF-8 text, truncation flag, and byte count. The default stored output limit is 64 KiB per node.

Before a node executes, the service assembles prerequisite context in the exact order of `depends_on`. Each block has a stable delimiter and source metadata followed by the bounded output. The combined prerequisite context is capped at 256 KiB; exceeding the cap is a non-retryable validation failure rather than silently dropping an entire prerequisite.

Passing plain strings without source metadata was rejected because downstream Agents could not distinguish instructions from upstream evidence. Arbitrary JSON was rejected for the first version because providers ultimately consume text and a schema would imply unsupported cross-provider structured-output guarantees.

### 5. Persist plan snapshots and run transitions atomically

SQLite stores an atomic run snapshot containing the plan, node/dependency/fallback order, run-node state/output, and attempts. Starting a run writes the complete snapshot before background execution. Attempt and node transitions are persisted before the next side effect; cancellation reloads and updates the snapshot in one transaction, while terminal re-execution is idempotent so duplicate scheduling cannot create duplicate fallbacks or outcomes.

Migration ordering remains in `platform::database`; coordination table ownership and repository code remain in `agent_runtime`. Rollback can stop exposing the commands while leaving additive tables intact.

### 6. Expose asynchronous service operations

`AgentService` gains `startCoordination`, `getCoordinationRun`, `listCoordinationRuns`, and `cancelCoordinationRun`. Starting returns an `OperationTask`/run identity before Agent work completes. Tauri adapter methods contain all `invoke()` calls; React consumers call only the service interface.

The Web/mock adapter uses the same validation and deterministic scheduling semantics in TypeScript with simulated outputs and failure fixtures. It reports simulated capability and never claims native process, SQLite, or logging side effects.

### 7. Correlate without logging content

One observable operation and execution run represent the coordination run. Node attempts are child spans/events with bounded attributes: node outcome, candidate role (`primary` or `fallback`), stable Agent id, and attempt number. Unified logs record lifecycle and failure classifications after redaction, never raw instructions, propagated context, or Agent output. Page-visible output remains available through the coordination service projection.

## Risks / Trade-offs

- [Sequential scheduling can be slower than parallel DAG execution] → Preserve a scheduler boundary and stable ready-set semantics so bounded concurrency can be added later without changing plan contracts.
- [Fallback Agents may interpret context differently] → Use provider-neutral provenance blocks and retain the actual Agent id and attempt history.
- [Agent output may contain secrets] → Keep it out of diagnostics/telemetry, bound storage, and rely on the existing content storage boundary; future content redaction policy can evolve separately.
- [Crash during an active attempt can leave a run non-terminal] → Startup recovery marks orphan `running` attempts failed with a retryable runtime-interruption classification, then either advances to the next fallback or fails the node deterministically.
- [Additive tables increase migration surface] → Use one versioned migration with clean-database and upgrade tests; no existing columns are rewritten.
- [Web mock can drift from Rust validation] → Maintain shared contract fixtures covering graph rejection, topological order, output provenance, and fallback ordering in both runtimes.

## Migration Plan

1. Add the domain/application contracts and deterministic tests.
2. Add the SQLite migration and repository with transaction and recovery tests.
3. Wire the native executor, scheduler, operation/logging adapters, Tauri commands, and command-safe errors.
4. Extend TypeScript contracts plus Tauri and Web/mock adapters.
5. Run strict OpenSpec, frontend, Rust, architecture, and migration validation.

Rollback removes command registration and frontend methods while leaving additive coordination tables readable for a later release. No existing session or message data requires transformation.

## Open Questions

- A future UI proposal must decide whether coordination plans live in a dedicated workspace destination or are attached to a Multi Agent session creation flow.
- Bounded parallelism and per-node timeouts should be introduced only after sequential recovery and cancellation behavior has production evidence.
