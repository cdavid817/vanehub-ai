## 1. Map the cut lines

- [ ] 1.1 Record the baseline: `web-agent-client.ts` physical lines, its inline `async` method count, and the `src/services` aggregate
- [ ] 1.2 Build the state-ownership map: for each of the 15 module-level `let` bindings, list every method that reads or reassigns it
- [ ] 1.3 Group the 218 inline methods into candidate bounded contexts, marking each group extractable only if every `let` it touches is touched by no other group
- [ ] 1.4 List the methods that share state across candidate groups; these stay in the composition root for this change, and the reason is recorded
- [ ] 1.5 Enumerate the 10 `this.` call sites and note, for each, whether caller and callee land in the same group

## 2. Extract group by group

- [ ] 2.1 For each extractable group: create `src/services/web-<context>-client.ts`, define its narrow service interface, move the methods and their `let` state verbatim, and keep the state unexported
- [ ] 2.2 Give any moved method that uses `this` an explicit `this: AgentService` parameter rather than rewriting the call into a direct import
- [ ] 2.3 Replace the moved methods in `webAgentClient` with a single spread, matching the existing `...webBuiltinToolClient` style
- [ ] 2.4 After each group: `tsc --noEmit`, `npm run contracts:check`, and `npm run test` pass before starting the next group
- [ ] 2.5 Lower the `web-agent-client.ts` budget in `eslint.config.js` after each group, so the ratchet tracks the work

## 3. Consolidate this file's localStorage access

- [ ] 3.1 Route the 12 `localStorage` accesses in `web-agent-client.ts` through a single storage module, preserving the exact keys, values, and write timing
- [ ] 3.2 Confirm no plaintext secret reaches browser storage, as the `frontend-runtime-architecture` requirement "Honest Web/mock behavior" requires
- [ ] 3.3 Leave the nine other `src/services` files that use `localStorage` untouched, and record them as follow-up scope

## 4. Prove the surface did not move

- [ ] 4.1 `webAgentClient` is still annotated `: AgentService` and `tsc --noEmit` passes
- [ ] 4.2 `npm run contracts:check` passes unchanged — no edit to `contract-conformance.test.ts` was needed
- [ ] 4.3 `npm run test` passes with an unchanged total test count
- [ ] 4.4 Confirm no React component or `src/components` file appears in the diff
- [ ] 4.5 Confirm `src/services/tauri-agent-client.ts` is untouched

## 5. Budgets and verification

- [ ] 5.1 Confirm the `src/services` subtree aggregate did not rise beyond per-module boilerplate; a material rise means code was duplicated rather than moved, and must be traced rather than absorbed by raising the budget
- [ ] 5.2 Set the final `web-agent-client.ts` budget to the measured value, or remove its `eslint.config.js` entry entirely if the file reached 300 lines
- [ ] 5.3 `npm run lint:ci`, `npm run build`, and `npm run architecture:check` pass
- [ ] 5.4 `npx playwright test` passes, since the Web/mock adapter backs the browser-mode UI these specs drive
- [ ] 5.5 `openspec validate split-web-agent-client --strict` and `openspec validate --specs --strict` pass
