## Why

`extract-web-client-state-modules` cannot finish its chat extraction while `sendMessage` remains a roughly 470-line orchestration method in `web-agent-client.ts`: moving it verbatim would violate the 300-line production-file limit, while splitting it without an explicit plan risks changing the Web/mock event order and delays. This follow-up establishes the safe decomposition seam needed to finish that refactor without changing behavior.

## What Changes

- Characterize `sendMessage`'s admission rules, synchronous state transition, scheduled event order, delays, cancellation behavior, and completion semantics with focused tests before moving code.
- Extract cohesive scheduling helpers whose inputs carry the already-computed message context, keeping every event payload, condition, timeout delay, and registration order unchanged.
- Move the remaining `sendMessage` orchestration into the existing chat client and compose it through the existing `ChatMessagingService` spread.
- Remove the inline implementation from `web-agent-client.ts`, ratchet file budgets to measured values, and complete task 3.4 in `extract-web-client-state-modules` once verification passes.
- Preserve the public `AgentService` surface and both runtime adapters. This affects only the Web/mock runtime's internal organization; desktop behavior and the Tauri adapter remain untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a behavior-preserving refactor, so `.openspec.yaml` sets `skip_specs: true`.

## Impact

- `src/services/web-agent-client.ts` loses the inline `sendMessage` implementation and becomes a smaller composition root.
- `src/services/web-chat-client.ts` owns `sendMessage`; focused Web/mock chat helpers and tests may be added under `src/services/`.
- `eslint.config.js` and the frontend architecture budget are ratcheted only to measured post-refactor values.
- `src/services/agent-service.ts`, `src/services/tauri-agent-client.ts`, React components, and Rust code keep their behavior and public contracts unchanged.
- The existing change `extract-web-client-state-modules` is updated only to record verified completion of task 3.4.
