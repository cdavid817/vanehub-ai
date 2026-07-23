## Why

VaneHub currently models one selected Agent per interactive session and has no explicit contract for coordinating several Agent executions. Teams therefore cannot express execution prerequisites, reuse an upstream Agent result as downstream context, or keep a workflow progressing when its preferred Agent fails.

## What Changes

- Add a first-class Multi-Agent coordination plan whose nodes declare a primary Agent, ordered fallback Agents, task instructions, and prerequisite node ids.
- Validate coordination plans as directed acyclic graphs before any Agent starts, rejecting missing references, duplicate ids, self-dependencies, cycles, and repeated primary/fallback Agent ids.
- Schedule ready nodes in deterministic topological order and expose durable run and node lifecycle state.
- Capture each successful node's bounded structured output and inject prerequisite outputs into the downstream Agent context with stable source metadata.
- Fail over from the primary Agent to ordered fallback Agents for retryable execution failures while preserving attempt history; do not fail over for cancellation, invalid plans, or non-retryable failures.
- Expose matching desktop and Web/mock service contracts for starting, reading, listing, and cancelling coordination runs.
- Record redacted coordination lifecycle and failover diagnostics through the unified logging/operation boundaries while keeping node output available to page consumers.

## Capabilities

### New Capabilities

- `multi-agent-coordination`: Defines plan validation, dependency scheduling, output-to-context propagation, failure policy, lifecycle state, service contracts, and runtime parity.

### Modified Capabilities

- `agent-execution-observability`: Extends execution correlation and bounded metrics to coordination runs, nodes, and failover attempts.

## Impact

- Desktop runtime: adds domain and application orchestration in `agent_runtime`, SQLite-backed plan/run state, Tauri commands, bootstrap wiring, and unified-log/operation integration.
- Web runtime: adds an equivalent deterministic mock coordinator without SQLite or local process execution.
- Frontend boundary: extends `AgentService` and shared TypeScript contracts; React remains isolated from `invoke()` and is not required to understand native persistence.
- Agent execution: reuses the existing provider/process boundary rather than embedding provider-specific behavior in the coordinator.
- Database: adds versioned coordination tables and migration coverage; existing sessions and messages remain compatible.
- Dependencies: no new frontend or Rust package is required.
