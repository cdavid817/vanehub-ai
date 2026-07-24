# Multi-Agent coordination

**Feature state: service/runtime delivered; creation UI preview.**

The native and Web/mock service boundaries can start, list, read, and cancel coordination runs. The normal create-session UI still disables Multi-Agent mode, so this is not yet a delivered end-user workflow.

## Plan model

A plan contains nodes with:

- a stable node id;
- one primary stable Agent id;
- ordered fallback stable Agent ids;
- a non-empty instruction;
- prerequisite node ids.

The complete graph is validated before execution. Duplicate ids, missing prerequisites, self-dependencies, cycles, unknown Agent ids, and repeated candidates reject the plan.

## Scheduling and context

Ready nodes are selected deterministically. A dependent starts only after every prerequisite succeeds. Successful prerequisite output is bounded and propagated with source node, actual Agent, and attempt provenance. Combined context overflow is non-retryable rather than silently dropping input.

## Fallback and cancellation

The primary Agent runs first. Ordered fallbacks are attempted only for retryable availability, process, provider, transport, or timeout failures. Validation, policy, cancellation, persistence, and context-bound failures do not trigger fallback.

Cancellation is idempotent: it stops the active attempt, prevents new attempts, and moves remaining nodes to consistent terminal states.

## Persistence and diagnostics

Desktop runs, node states, attempts, bounded outputs, timestamps, and errors are persisted in SQLite. Web/mock uses deterministic in-memory simulation. Unified logs retain redacted lifecycle correlation; raw instructions and outputs remain out of diagnostic channels.

The user-facing preview is described in the localized user guides without fictitious controls or screenshots.
