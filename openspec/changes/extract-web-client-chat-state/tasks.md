## 1. Re-measure the residual before building on it

- [ ] 1.1 Rebuild the state-ownership map from the TypeScript AST — union-find over module-level mutable bindings, `let` and in-place-mutated `const` containers alike, with the transitive closure taken through the file's top-level helpers — and record where `extract-web-client-state-modules`'s numbers need correcting
- [ ] 1.2 Confirm the binding inventory and the method partition against the deferral note
- [ ] 1.3 Identify the minimal binding set whose relocation breaks the component, and confirm by re-running union-find with each candidate removed rather than by inspection
- [ ] 1.4 Enumerate the remaining `this.` call sites and note whether caller and callee land in the same group

## 2. Extract the chat-state module that fuses the residual

- [ ] 2.1 Create the chat-state module owning `messagesBySession`, `subscribersBySession`, `activeStreams` and `nextMessageId`, exporting accessors only, and move the message/stream write path with them
- [ ] 2.2 Confirm no state module exports a mutable binding, and that the accessor set matches what the call sites need rather than exposing the binding shape
- [ ] 2.3 Collapse `resetWebLoopsForTest` back into `web-loop-state.ts` and stop exporting the two half-steps the previous stage needed
- [ ] 2.4 `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before starting the next group

## 3. Extract the contexts the chat seam unblocks

- [ ] 3.1 Chat and messaging: the message, feedback, subscription and generation-control methods
- [ ] 3.2 Memories: the memory-state module and the three memory methods, leaving the two foreign readers on accessors
- [ ] 3.3 Session recovery: the recovery-state module, the recovery trio, and the two exported recovery seams
- [ ] 3.4 Chat configs: the chat-config-state module and the two session chat-config methods
- [ ] 3.5 Session lifecycle and queries: `createSession`, `deleteSession`, `switchSession`, `archiveSession` and the rest, plus the session query and runner surface
- [ ] 3.6 Each group: a narrow interface whose signatures **move** out of `agent-service.ts`, methods moved verbatim, a single spread in the composition root, and `this: AgentService` on any method that uses `this`
- [ ] 3.7 After each group: `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before the next group starts
- [ ] 3.8 Lower the `web-agent-client.ts` budget in `eslint.config.js` after each group so the ratchet tracks the work
- [ ] 3.9 Record any context that could not be cut without exporting a binding, with the reason, rather than exporting the binding

## 4. Prove the surface did not move

- [ ] 4.1 `webAgentClient` is still annotated `: AgentService` and `tsc --noEmit` passes
- [ ] 4.2 `npm run contracts:check` passes with no edit to `contract-conformance.test.ts`
- [ ] 4.3 `npm run test` passes with an unchanged total test count and no test file edited
- [ ] 4.4 No React component, `.tsx` file, or Rust file appears in the diff
- [ ] 4.5 `src/services/tauri-agent-client.ts` is byte-identical

## 5. Budgets and verification

- [ ] 5.1 Confirm the `src/services` subtree aggregate rose only by per-module boilerplate plus the bounded accessor set; trace any material rise rather than absorbing it into the budget
- [ ] 5.2 Set the final `web-agent-client.ts` and `agent-service.ts` budgets to the measured values, or remove the `web-agent-client.ts` entry if the file reached 300 lines
- [ ] 5.3 `npm run lint:ci`, `npm run build` and `npm run architecture:check` pass
- [ ] 5.4 `npx playwright test` passes, since the Web/mock adapter backs the browser-mode UI these specs drive
- [ ] 5.5 `openspec validate extract-web-client-chat-state --strict` and `openspec validate --specs --strict` pass
