## Why

`extract-web-client-state-modules` took `src/services/web-agent-client.ts` from 3,861 to 1,877 lines by moving 77 methods into 23 `web-*` modules, breaking the 99-method hub that `split-web-agent-client` could not cross. It stopped at a residual it recorded as "a 20-method component over 8 bindings", deferring three contexts — chat and messaging, memories, and session lifecycle plus recovery — to a follow-up, and it was explicit that the stop was for review size, not for safety: no context required exporting a mutable binding.

Every stage in this chain has found the previous stage's analysis partly wrong, so this one re-derived the map from the TypeScript AST before planning a cut: union-find over module-level mutable bindings, `let` and in-place-mutated `const` containers alike, with the transitive closure taken through the file's top-level helpers.

**The headline numbers hold.** 8 mutable bindings, 39 inline methods, 20 of them touching at least one binding, 19 fully disjoint. The binding names match the deferral note exactly.

Two things need correcting, and both change the plan:

- **The component is 25 units, not 20.** Five exported top-level helpers are in it alongside the 20 methods — `seedWebRecoverySessionForTest`, `resetWebRecoverySessionsForTest`, `resolveWebMockToolApproval`, `resetWebAgentMemoriesForTest`, `resetWebLoopsForTest`. Counting only object methods understates the cut, because those helpers also need a home and they also fuse bindings. `resetWebRecoverySessionsForTest` alone fuses recovery to chat, so recovery cannot be freed by handling `deleteSession`.
- **Only one binding set fuses the component, and it is the chat core.** Re-running union-find with each candidate removed: relocating memories alone, recovery alone, or chat configs alone leaves one component intact every time. Relocating `messagesBySession` / `subscribersBySession` / `activeStreams` / `nextMessageId` splits it into exactly two — `{webAgentMemories, nextAgentMemoryId}` and `{memoryChatConfigs, recoveryReportsBySession}`. Chat state is not merely the largest context, it is the only load-bearing one, so it has to move first rather than last.

The deferral note also frames memories as "3 methods over `webAgentMemories` / `nextAgentMemoryId`". Three methods move, but **five** call sites read the pool: `deleteApiAgent` counts memories for its delete-blocking check and `sendMessage` both writes and indexes it. Those two stay behind and need accessors, which is a state module's job, not a client module's.

## What Changes

- Add state modules that **own** the last 8 shared bindings and expose behaviour, never the binding: a chat-state module for `messagesBySession` / `subscribersBySession` / `activeStreams` / `nextMessageId`, a memory-state module for `webAgentMemories` / `nextAgentMemoryId`, a recovery-state module for `recoveryReportsBySession`, and a chat-config-state module for `memoryChatConfigs`.
- Extract the contexts the chat seam unblocks into `web-*` client modules composed by spread, following the interface-plus-spread convention the two prior stages established.
- Collapse the one seam `extract-web-client-state-modules` had to split: `resetWebLoopsForTest` clears loop state *and* `messagesBySession`, so it currently lives in the composition root calling `clearWebLoopTimersAndSubscribers()` and `clearWebLoopStateForTest()` around the message cleanup. Once chat state is behind accessors it returns to a single function in the loop state module, and the two half-steps stop being exported.
- Continue ratcheting the `web-agent-client.ts` per-file budget in `eslint.config.js` down after each group, and remove the entry if the file reaches 300 lines.
- **No UI change and no behavior change.** No React component is touched, no Rust is touched, `tauri-agent-client.ts` stays byte-identical. Every method keeps its signature, return shape, mock data, ordering, and timing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure code-organization refactor of one adapter's internals, continuing the lane `split-web-agent-client` opened and `extract-web-client-state-modules` advanced. The `frontend-runtime-architecture` requirements "Mechanically enforced runtime adapter parity" and "Honest Web/mock behavior" describe behavior this change deliberately preserves. The change sets `skip_specs: true`.

## Impact

- `src/services/web-agent-client.ts` — shrinks further toward a composition root.
- `src/services/web-*-state.ts` — new state modules owning the previously file-local mutable bindings, exporting accessors only.
- `src/services/web-*-client.ts` — new context modules joining the 55 the two prior stages added.
- `src/services/web-loop-state.ts` — `resetWebLoopsForTest` returns whole; `clearWebLoopTimersAndSubscribers` and `clearWebLoopStateForTest` stop being exported.
- `src/services/*-service.ts` — narrow interfaces whose signatures **move** out of `agent-service.ts`, which `AgentService` then extends.
- `eslint.config.js` — the `web-agent-client.ts` and `agent-service.ts` per-file budgets drop.
- `scripts/architecture/frontend-rules.mjs` — the `src/services` subtree budget of 19,087 continues to bind. Extraction moves code within the subtree, so the aggregate is neutral apart from per-module boilerplate and the accessor pairs a state module must add where a direct read/write used to suffice; a material rise means code was duplicated rather than moved and the budget failing is the correct outcome.
- `src/contracts/contract-conformance.test.ts` — must keep passing unchanged; it is the evidence the surface did not move.
- No Rust file is touched. No Tauri command, SQLite schema, or native behavior is affected.
