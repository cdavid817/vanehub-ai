## Context

See `proposal.md` for motivation. Runner discovery currently returns one fallible aggregate even though Local readiness is independent of session-bound SSH metadata. Run status polling refreshes snapshots, but the elapsed presenter subtracts two persisted timestamps, so a running snapshot whose `updatedAt` is unchanged remains at `0:00`. Multi-seat generation already publishes seat-attributed turn status and message stream events; the frontend cannot apply events for a message row that has not reached its query cache yet, creating a start/refetch/token race.

The implementation crosses React presentation, the shared service event contract, Web/mock behavior, and Rust-native discovery. React components must remain independent of direct Tauri invocation, and runtime diagnostics must use the unified redacted logging path.

## Goals / Non-Goals

**Goals:**

- Preserve usable independent Runner descriptors when optional discovery inputs fail.
- Compute active and terminal elapsed durations from the correct clock boundary.
- Reconcile newly created multi-seat messages early enough for incremental output to remain visible.
- Derive compact member activity from existing stable seat and stream events.

**Non-Goals:**

- Implement Docker or Cloud Runners.
- Add a second multi-Agent transcript or a second Run state machine.
- Expose raw subprocess output, credentials, prompts, or transport diagnostics in roster state.
- Make multi-Agent seats execute concurrently when their handoff policy is sequential.

## Decisions

### Runner discovery is fail-soft per independent source

Local is constructed and validated independently. A missing session or invalid optional SSH binding omits that SSH choice while Local remains selectable; storage and other catalog-critical errors still cross the safe command boundary as discovery failures. This is preferred over a frontend-only fallback because readiness authority belongs to the native Runner adapter and every client must receive the same truth.

### Elapsed presentation selects live or terminal clock by Run state

The shared presenter uses the current UI clock for non-terminal states and the persisted terminal update timestamp for terminal states, always bounded below by the canonical creation timestamp. The existing one-second refresh supplies rerenders; no per-status native timer event is added. This is preferred over mutating `updatedAt` every second because timestamp persistence should represent lifecycle changes, not display ticks.

### Managed CLI terminal handling closes the canonical Run

The managed CLI stream terminal path maps success, failure, and cancellation to the existing canonical Run terminal API before completing its observability records. Message and Operation persistence remain the detailed evidence, while the canonical Run remains the single lifecycle authority shown after restart. Duplicate terminal callbacks continue to be coalesced by the existing stream terminal guard and Run witness/version checks. This is preferred over teaching the UI to override a failed Run from message state, which would create a second lifecycle authority.

### Existing stream events remain the output authority

Member output continues through the shared conversation and existing `started`, `thinking`, `tool_use`, `token`, and terminal events. The frontend subscription detects unknown message ids, reconciles the persisted row, and performs a settle-time reconciliation that cannot wait until terminal completion. This is preferred over adding a parallel member-output channel, which would duplicate ordering, persistence, cancellation, and attribution semantics.

### Roster activity is a projection keyed by stable seat id

The active seat comes from `turn_status`; message activity comes from the speaker seat on the reconciled message and its streaming fields/status. The UI shows compact localized states and keeps the full incremental content in the shared thread. Legacy events without a stable seat id retain the current session-level behavior instead of guessing by display name or seat position.

### Web/mock follows the same observable contract

Web/mock continues emitting deterministic seat-attributed events and must exercise the unknown-row reconciliation and live elapsed behavior. Native-only SSH warnings are simulated only in targeted adapter tests; browser mode does not claim native discovery.

## Risks / Trade-offs

- [An early refetch can still overlap a high-volume token burst] → Buffer/reconcile by message id and schedule a bounded follow-up refresh after the first reconciliation.
- [Using the client clock can expose clock skew] → Clamp elapsed to zero and freeze terminal values against persisted timestamps.
- [An unavailable SSH descriptor may have incomplete display metadata] → Return only bounded known identifiers/labels and never fabricate authority or credentials.
- [More frequent member-state rendering can add churn] → Reuse animation-frame event batching and derive only compact state changes.
- [A terminal message can be durable before a later Run transition fails] → Keep the transition on the shared native boundary, retain correlated Operation evidence for recovery diagnostics, and cover the normal terminal path with a regression test.

## Migration Plan

No database migration is required. Deploy native fail-soft discovery and frontend projections together. Existing Run and message rows remain readable. Rollback restores the earlier projection without altering persisted data.
