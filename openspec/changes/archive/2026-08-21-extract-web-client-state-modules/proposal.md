## Why

`split-web-agent-client` took `src/services/web-agent-client.ts` from 6,013 to 3,861 lines by moving 102 methods into 32 `web-*` modules, then stopped at a boundary it could not cross safely. Its union-find over shared module-level mutable bindings left one hub component that its own tasks.md records as 101 methods over 29 bindings; it declined to split that hub because every candidate cut would have forced a `let` to be exported, and design.md rules that out — an exported mutable binding re-imported from two modules gives two divergent copies of the mock world, which surfaces as one UI panel showing stale data while another shows fresh.

Re-measuring the hub on the post-#168 file from the TypeScript AST, with the transitive closure taken through the 90 remaining top-level helpers, it is **99 of the 116 remaining inline methods over 45 mutable module-level bindings** — 26 `let` and 19 `const` Map/Set/array containers that are mutated in place. The `const` containers matter as much as the `let`s: `messagesBySession`, `webAgentRunEvents`, `loopTimers` and `sessionEventSubscribers` are as much shared mutable state as `sessions` is, and a cut line that ignores them is not safe either.

The reason the hub is one component is not that 99 methods are genuinely one bounded context. It is that they are 12 contexts joined by a handful of bindings every context reaches: `sessions`, `activeSessionId`, `workflowState` on the session side and `webSkills`, `webSkillMountPaths`, `webSkillDocuments` on the skills side. `deleteApiAgent` touches sessions **and** loops **and** memories **and** skills; `sendMessage` touches sessions **and** runs **and** memories **and** skills. Under union-find that fuses everything reachable, but the fusion is an artifact of *where the binding is declared*, not of the cascade being irreducible.

#168 already demonstrated the escape hatch on four occasions — `deleteWebApiAgentProviderConfig`, `discoverWebSessionCodeIndex`, `findWebCliConfigProfile`, `setWebCliConfigStatus` are all behaviour exported from a state module in place of a binding. This change applies that systematically to the two bindings sets that hold the hub together.

## What Changes

- Add state modules that **own** the hub's shared bindings and expose accessors — behaviour, never the binding: a session-state module for `sessions` / `activeSessionId` / `workflowState` / `nextSessionId` / `nextSeatId` / `sessionEventSubscribers`, and a skills-state module for `webSkills` / `webSkillMountPaths` / `webSkillApiAgentBindings` / `webSkillDocuments` / `webSkillResourceDocuments` / `deletedBuiltinSkillIds` / `nextWebSkillRevision`.
- Once the two hubs are behind accessors, extract the contexts the new seams unblock into `web-*` modules composed by spread, following the interface-plus-spread convention `split-web-agent-client` established.
- Keep the cascade methods (`deleteApiAgent`, `sendMessage`, `createSession`, …) working unchanged. They stop being blockers because each of their several state touches now goes through an imported accessor rather than through a file-local binding, so no cut forces a binding export.
- Continue ratcheting the `web-agent-client.ts` per-file budget in `eslint.config.js` down after each group, and remove the entry entirely if the file reaches 300 lines.
- **No UI change and no behavior change.** No React component is touched, no Rust is touched, `tauri-agent-client.ts` stays byte-identical. Every method keeps its signature, return shape, mock data, ordering, and timing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure code-organization refactor of one adapter's internals, continuing the lane `split-web-agent-client` opened. The `frontend-runtime-architecture` requirements "Mechanically enforced runtime adapter parity" and "Honest Web/mock behavior" describe behavior this change deliberately preserves. The change sets `skip_specs: true`.

## Impact

- `src/services/web-agent-client.ts` — shrinks further toward a composition root.
- `src/services/web-*-state.ts` — new state modules owning the previously file-local mutable bindings, exporting accessors only.
- `src/services/web-*-client.ts` — new context modules joining the 32 `split-web-agent-client` added.
- `src/services/*-service.ts` — narrow interfaces whose signatures **move** out of `agent-service.ts`, which `AgentService` then extends.
- `eslint.config.js` — the `web-agent-client.ts` and `agent-service.ts` per-file budgets drop.
- `scripts/architecture/frontend-rules.mjs` — the `src/services` subtree budget of 18,513 continues to bind. Extraction moves code within the subtree, so the aggregate is neutral apart from per-module boilerplate and the accessor pairs a state module must add where a direct read/write used to suffice; a material rise means code was duplicated rather than moved and the budget failing is the correct outcome.
- `src/contracts/contract-conformance.test.ts` — must keep passing unchanged; it is the evidence the surface did not move.
- No Rust file is touched. No Tauri command, SQLite schema, or native behavior is affected.
