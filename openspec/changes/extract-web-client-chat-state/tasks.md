## 1. Re-measure the residual before building on it

- [x] 1.1 Rebuild the state-ownership map from the TypeScript AST — union-find over module-level mutable bindings, `let` and in-place-mutated `const` containers alike, with the transitive closure taken through the file's top-level helpers — and record where `extract-web-client-state-modules`'s numbers need correcting
  - The headline numbers hold: **8 bindings, 39 inline methods, 20 touching at least one binding, 19 fully disjoint**, and the binding names match the deferral note exactly.
  - **Correction: the component is 25 units, not 20.** Five exported top-level helpers sit in it alongside the 20 methods — `seedWebRecoverySessionForTest`, `resetWebRecoverySessionsForTest`, `resolveWebMockToolApproval`, `resetWebAgentMemoriesForTest`, `resetWebLoopsForTest`. Counting only object methods understates the cut, because those helpers also need a home and they also fuse bindings; `resetWebRecoverySessionsForTest` fuses recovery to chat on its own, so recovery could not have been freed by handling `deleteSession` alone.
  - **Correction: memories is three methods to move but five call sites.** `deleteApiAgent` counts the pool for its delete-blocking check and `sendMessage` both writes and indexes it. Both stay behind and needed accessors.
- [x] 1.2 Confirm the binding inventory and the method partition against the deferral note
  - `nextMessageId`, `recoveryReportsBySession`, `messagesBySession`, `subscribersBySession`, `activeStreams`, `memoryChatConfigs`, `webAgentMemories`, `nextAgentMemoryId` — 4 `let` and 4 in-place-mutated `const` containers. Confirmed by AST, not by reading.
- [x] 1.3 Identify the minimal binding set whose relocation breaks the component, and confirm by re-running union-find with each candidate removed rather than by inspection
  - **The chat core is the only fusing set.** Re-running union-find with memories alone removed, with recovery alone removed, and with chat configs alone removed each leaves **one** component. Removing `messagesBySession` / `subscribersBySession` / `activeStreams` / `nextMessageId` splits it into exactly two: `{webAgentMemories, nextAgentMemoryId}` and `{memoryChatConfigs, recoveryReportsBySession}`. Measured before any code moved.
  - The two fusing methods are `sendMessage` (memories to chat) and `deleteSession` (chat configs and recovery to chat).
- [x] 1.4 Enumerate the remaining `this.` call sites and note whether caller and callee land in the same group
  - **None.** No inline method used `this`, so no moved method needed a `this: AgentService` parameter and no `this.x()` was rewritten into an import. Checked against the AST rather than assumed.

## 2. Extract the chat-state module that fuses the residual

- [x] 2.1 Create the chat-state module owning `messagesBySession`, `subscribersBySession`, `activeStreams` and `nextMessageId`, exporting accessors only, and move the message/stream write path with them
  - `src/services/web-chat-state.ts`, 173 lines. It also takes the write path — `upsertMessage`, `applyStreamEvent`, `finishWebGeneration` stay private; `createWebMessageId`, `getWebSessionMessages`, `setWebSessionMessages`, `deleteWebSessionMessages`, `listWebSessionMessageBuckets`, `emitWebChatEvent`, `publishWebChatEvent`, `cancelWebActiveStream`, `hasWebActiveStream`, `setWebActiveStream`, `deleteWebActiveStream`, `subscribeWebChatEvents` and `deleteWebChatSubscribers` are the exported surface.
- [x] 2.2 Confirm no state module exports a mutable binding, and that the accessor set matches what the call sites need rather than exposing the binding shape
  - `grep -nE "^export (let|var) "` over `src/services` is empty, and none of the four new state modules exports a container either.
  - The accessor set was derived from the call sites: `nextMessageId` gets only `createWebMessageId()` because every use was `web-message-${nextMessageId++}`; `listWebSessionMessageBuckets()` exists because `saveMessageFeedback` addresses a message by id across every session's bucket. `getWebSessionMessages` returns the live array because the previous expression did, which `saveMessageFeedback` depends on — it mutates the `ChatMessage` in place.
- [x] 2.3 Collapse `resetWebLoopsForTest` back into `web-loop-state.ts` and stop exporting the two half-steps the previous stage needed
  - Back to one function with its original statement order. `clearWebLoopTimersAndSubscribers` and `clearWebLoopStateForTest` are gone, and `snapshotWebLoopRoleSessionIds` is now private since its only caller is in the same module.
- [x] 2.4 `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before starting the next group

## 3. Extract the contexts the chat seam unblocks

- [x] 3.1 Chat and messaging: the message, feedback, subscription and generation-control methods
  - 6 methods into `web-chat-client.ts` (162) over `chat-messaging-service.ts` (29), with `resolveWebMockToolApproval`, `resolveSimulatedQuestion`, `resolveSimulatedPlanExit` and the two mock triggers.
  - `sendMessage` is **not** extracted — see 3.9.
- [x] 3.2 Memories: the memory-state module and the three memory methods, leaving the two foreign readers on accessors
  - 3 methods into `web-agent-memory-client.ts` (20) over `agent-memory-service.ts` (9), with the pool and its derivation/disambiguation/injection helpers in `web-agent-memory-state.ts` (106).
- [x] 3.3 Session recovery: the recovery-state module, the recovery trio, and the two exported recovery seams
  - 3 methods into `web-session-recovery-client.ts` (53) over `session-recovery-service.ts` (14), with `web-session-recovery-state.ts` (132) taking the reports map, the report factory, the summary read and both `*ForTest` seams.
- [x] 3.4 Chat configs: the chat-config-state module and the two session chat-config methods
  - 2 methods into `web-session-chat-config-client.ts` (30) over `session-chat-config-service.ts` (6), with `web-chat-config-state.ts` (27). The state module exposes `deleteWebSessionChatConfig` so the delete path is a named step rather than a raw copy-delete-write at the call site.
- [x] 3.5 Session lifecycle and queries: `createSession`, `deleteSession`, `switchSession`, `archiveSession` and the rest, plus the session query and runner surface
  - 11 read-side methods into `web-session-query-client.ts` (172) over `session-query-service.ts` (26), with `searchText`, `sessionSearchMatches` and `serializeWebSessionExport` as private helpers.
  - 8 lifecycle methods into `web-session-lifecycle-client.ts` (218) and 2 seat methods into `web-session-seat-client.ts` (87), both over `session-lifecycle-service.ts` (18). Split in two because one module holding all ten would have crossed the 300-line rule.
  - The runner descriptor helpers went to `web-agent-runner.ts` (80) because `sendMessage` and `listAgentRunners` now live in different modules.
- [x] 3.6 Each group: a narrow interface whose signatures **move** out of `agent-service.ts`, methods moved verbatim, a single spread in the composition root, and `this: AgentService` on any method that uses `this`
  - Six new interface files carrying seven interfaces; `AgentService` extends 6 more of them and `agent-service.ts` fell 364 → 308. No signature is duplicated. 18 new modules in total, 35 of the 39 inline methods moved.
  - Method bodies moved verbatim apart from two mechanical substitutions, each behaviour-identical: a direct binding read became the accessor call, and the file-local `tr(key)` shim became `i18n.t(key)`, which is what it forwarded to and what the other extracted modules already use. `tr` is gone with its last caller.
- [x] 3.7 After each group: `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before the next group starts
  - Run at all five group boundaries. `npm run test` reported 286 files / 1301 tests passing each time — the same totals as the pre-change baseline, with no test file edited.
- [x] 3.8 Lower the `web-agent-client.ts` budget in `eslint.config.js` after each group so the ratchet tracks the work
  - 1,877 → 1,608 → 1,514 → 1,332 → 1,105 → 822. `agent-service.ts` ratcheted alongside, 364 → 346 → 342 → 338 → 320 → 308.
- [x] 3.9 Record any context that could not be cut without exporting a binding, with the reason, rather than exporting the binding
  - **No context required a binding export.** Every one of the 8 bindings is now behind accessors, the composition root holds **zero** mutable module-level bindings, and all 4 methods still inline are fully disjoint.
  - **`sendMessage` is not extracted, and the reason is the 300-line rule, not the binding rule.** Its body is 529 physical lines. `max-lines` is a hard ESLint rule at 300 for all production TypeScript and the technical-debt list in `eslint.config.js` is closed to new entries, so a `web-chat-client.ts` holding it would either break the build or need a forbidden exemption. Making it fit means hoisting its roughly fifteen interleaved `setTimeout` simulation blocks into scheduling helpers — a rewrite of the one method whose entire observable contract is that ordering and those exact delays, in a change whose stated guarantee is that bodies move verbatim. It is a legitimate follow-up of the shape `decompose-api-tool-use-loop` took for the API adapter's 978-line loop. Its state is no longer file-local, so it blocks nothing.
  - `deleteApiAgent`, `applyCliConfigProfile` and `subscribeSessionEvents` also stay: each is a one-method context whose own module would be more boilerplate than body, and none touches shared state any more.

## 4. Prove the surface did not move

- [x] 4.1 `webAgentClient` is still annotated `: AgentService` and `tsc --noEmit` passes
  - `[ARCH-FE-003]` in `scripts/architecture/frontend-rules.mjs` also fails the build if either adapter loses the annotation; `npm run architecture:check` passes.
- [x] 4.2 `npm run contracts:check` passes with no edit to `contract-conformance.test.ts`
- [x] 4.3 `npm run test` passes with an unchanged total test count and no test file edited
  - 286 files / 1301 tests, before and after. The moved seams — `resetWebLoopsForTest`, `resetWebAgentMemoriesForTest`, `seedWebRecoverySessionForTest`, `resetWebRecoverySessionsForTest`, `resolveWebMockToolApproval`, `WEB_MOCK_QUESTION_TRIGGER`, `WEB_MOCK_PLAN_EXIT_TRIGGER` — are re-exported from `web-agent-client.ts`, so no importer changed.
- [x] 4.4 No React component, `.tsx` file, or Rust file appears in the diff
  - The diff is `eslint.config.js`, `scripts/architecture/frontend-rules.mjs`, this change's artifacts, and `src/services/*.ts`.
- [x] 4.5 `src/services/tauri-agent-client.ts` is byte-identical
  - `git diff --stat` against the branch point is empty for that path.

## 5. Budgets and verification

- [x] 5.1 Confirm the `src/services` subtree aggregate rose only by per-module boilerplate plus the bounded accessor set; trace any material rise rather than absorbing it into the budget
  - 19,087 → 19,347, **+260 over 18 new files, about 14 lines each** — inside the 11-25 band the two prior stages measured, and below this change's own chat group because only the chat core needed a new accessor layer at all. The other three state modules mostly moved helpers that were already accessor-shaped.
  - It is not duplication. Method bodies moved verbatim, and the narrow interfaces' signatures moved out of `AgentService` rather than being copied, which is why `agent-service.ts` fell 364 → 308.
- [x] 5.2 Set the final `web-agent-client.ts` and `agent-service.ts` budgets to the measured values, or remove the `web-agent-client.ts` entry if the file reached 300 lines
  - Set to 822 and 308. Both entries stay: `sendMessage` alone is 529 lines.
- [x] 5.3 `npm run lint:ci`, `npm run build` and `npm run architecture:check` pass
- [x] 5.4 `npx playwright test` passes, since the Web/mock adapter backs the browser-mode UI these specs drive
- [x] 5.5 `openspec validate extract-web-client-chat-state --strict` and `openspec validate --specs --strict` pass
