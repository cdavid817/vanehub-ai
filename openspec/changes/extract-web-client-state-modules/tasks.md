## 1. Re-measure the hub before building on it

- [ ] 1.1 Rebuild the state-ownership map from the TypeScript AST with the transitive closure taken through the top-level helpers, and record where `split-web-agent-client`'s numbers need correcting
- [ ] 1.2 Separate `let` bindings from `const` containers that are mutated in place, and confirm both classes are treated as shared mutable state by the cut rule
- [ ] 1.3 Identify the minimal binding set whose relocation breaks the hub component, and confirm by re-running union-find with those bindings removed
- [ ] 1.4 Enumerate the remaining `this.` call sites and note whether caller and callee land in the same group

## 2. Extract the two state modules that fuse the hub

- [ ] 2.1 Create the session-state module owning `sessions`, `activeSessionId`, `workflowState`, `nextSessionId`, `nextSeatId` and `sessionEventSubscribers`, exporting accessors only
- [ ] 2.2 Create the skills-state module owning `webSkills`, `webSkillMountPaths`, `webSkillApiAgentBindings`, `webSkillDocuments`, `webSkillResourceDocuments`, `deletedBuiltinSkillIds` and `nextWebSkillRevision`, exporting accessors only; move the fixture seeds to a seeds module so neither file breaks the 300-line rule
- [ ] 2.3 Confirm no state module exports a mutable binding, and that the accessor set matches what the call sites need rather than exposing the binding shape
- [ ] 2.4 `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before starting the next group

## 3. Extract the contexts the new seams unblock

- [ ] 3.1 Skills: the skill catalogue, mount-path and binding methods, plus the 14 skill-overlay delegations that were blocked by the resolver closing over `webSkills`
- [ ] 3.2 Loops and agent runs / mission control, each with its own state module
- [ ] 3.3 The smaller session-adjacent contexts the session-state module unblocks — memories, session categories, expert roles, known projects, usage statistics, terminals, recovery, workflow, chat configs
- [ ] 3.4 Chat and messaging, including the cascade method `sendMessage`
- [ ] 3.5 Session lifecycle, including the cascade methods `createSession`, `deleteSession` and `archiveSession`
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
- [ ] 5.5 `openspec validate extract-web-client-state-modules --strict` and `openspec validate --specs --strict` pass
