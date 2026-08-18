## Context

See proposal.md — Why for the re-measurement and for where `extract-web-client-state-modules`'s numbers needed correcting.

Four properties of the post-#175 file determine how the residual can be cut:

- **8 mutable module-level bindings remain**, and they are one component: `nextMessageId`, `recoveryReportsBySession`, `messagesBySession`, `subscribersBySession`, `activeStreams` (declared together at the top of the file), `memoryChatConfigs`, `webAgentMemories`, `nextAgentMemoryId`. 20 of the 39 inline methods touch at least one; the other 19 touch none and are already free.
- **The chat core is the only fusing set.** Removing `{messagesBySession, subscribersBySession, activeStreams, nextMessageId}` splits the component into `{webAgentMemories, nextAgentMemoryId}` and `{memoryChatConfigs, recoveryReportsBySession}`. Removing any other candidate on its own leaves one component. Two methods do the fusing: `sendMessage` joins memories to chat, `deleteSession` joins chat configs and recovery to chat.
- **Five exported helpers are inside the component**, not just object methods. `resolveWebMockToolApproval` is imported by `web-permissions-client.ts`; the four `*ForTest` seams are imported by tests. Their implementations move with their state; `web-agent-client.ts` re-exports them so no importer changes.
- **The write path is already helper-shaped.** `getSessionMessages`, `setSessionMessages`, `upsertMessage`, `emitChatEvent`, `applyStreamEvent`, `publishChatEvent`, `cancelActiveStream`, `createMessageId` are the accessor set for the chat core; they just live in the same file as the bindings today. The same holds for `createAgentMemory` / `simulateMemoryIndexInjection` over the memory pool and `readChatConfigs` / `writeChatConfigs` over `memoryChatConfigs`.

## Goals / Non-Goals

**Goals:**

- Relocate the last 8 shared bindings into state modules that own them and expose accessors, so no context is blocked by state ownership any more.
- Extract every context the chat seam unblocks, keeping each intermediate step compiling and green so the work is resumable at a group boundary.
- Leave the observable surface — signatures, return shapes, mock data, ordering, timing — untouched.

**Non-Goals:**

- Decomposing `sendMessage`. See the decision below: its body is 529 lines, and splitting it is a rewrite, not a move.
- Touching any React component, the Tauri client, or Rust.
- Reaching a specific line count.

## Decisions

### A state module owns bindings and exports behaviour; it never exports a binding

Unchanged from both prior stages, and the reason this change exists rather than a one-line `export let`. An exported `let` is copied into the importing module's binding at import time under some bundler and interop configurations, so a reassignment in the owner is invisible to the importer and the mock world silently forks — one UI panel showing stale data while another shows fresh. An exported function reads the live binding on every call, so it cannot fork.

### Chat state moves first, because it is the only cut that changes the graph

The measured consequence, not a preference. Memories, recovery and chat configs each look independently extractable by inspection, and each is not: with only that binding relocated, union-find still returns one component, because `deleteSession` and `sendMessage` reach across. With the chat core relocated, the other three are free simultaneously. So group order is chat state, then the contexts it unblocks — the reverse of the order the deferral note lists them in.

### `sendMessage` stays in the composition root

`sendMessage` is 529 physical lines. `max-lines` is a hard ESLint rule at 300 for all production TypeScript, the technical-debt list in `eslint.config.js` is explicitly closed to new entries, and `web-agent-client.ts` already has an entry. So moving `sendMessage` into a new `web-chat-client.ts` would either break the build or require adding a forbidden exemption.

Making it fit means splitting its body — hoisting the simulation blocks (compaction, memory extraction, memory injection, skills, approval, question, plan exit, grep, remember, MCP, rich blocks) into scheduling helpers that take a shared context. That is a rewrite of a method whose entire observable contract is the ordering and the exact `setTimeout` delays of about fifteen interleaved blocks, in a change whose stated guarantee is that method bodies move verbatim. It is a legitimate follow-up — `decompose-api-tool-use-loop` did exactly this for the API adapter's 978-line loop — but it is a different change with a different risk profile, and folding it in here would make the "moved verbatim" claim untrue for the one method where it matters most.

So `sendMessage` stays where it is and reaches chat, memory, run and session state through the accessors this change creates. Its state is no longer file-local, so it stops being a blocker for anything else even though it does not move.

*Alternative rejected — move `sendMessage` with an exemption-list entry*: forbidden by AGENTS.md, and it would convert a hard rule into a negotiable one for every later file.

### Accessors are the narrowest set the call sites actually need

Every accessor pair costs lines a direct read/write did not, and the `[ARCH-FE-004]` subtree budget is the check on that. So the accessor set is derived from the call sites: `nextMessageId` gets only `createWebMessageId()` because every use was `web-message-${nextMessageId++}`; `messagesBySession` gets `getWebSessionMessages` / `setWebSessionMessages` / `deleteWebSessionMessages` / `listWebSessionMessageBuckets` because those are the four shapes the call sites use, and no more.

### The seam split across two owners collapses back to one

`resetWebLoopsForTest` was left in the composition root by the previous stage, calling `clearWebLoopTimersAndSubscribers()` and `clearWebLoopStateForTest()` around a `messagesBySession.delete` loop, because the loop module could not reach chat state without a binding export. With `deleteWebSessionMessages(sessionId)` available, the whole function moves into `web-loop-state.ts` in its original single-step form and the two half-steps stop being exported. Ordering is preserved because the moved function keeps the same statement order.

### `this.` call sites survive spread, but the extracted method must say so

Unchanged from both prior stages: a moved method that uses `this` declares an explicit `this: AgentService` parameter. Rewriting `this.x()` into a direct import is not equivalent — it would bypass any later override in the composition root. None of the 39 remaining methods uses `this` today, which is checked rather than assumed.

### Exported test seams keep their current import path

`resetWebAgentMemoriesForTest`, `resetWebLoopsForTest`, `seedWebRecoverySessionForTest`, `resetWebRecoverySessionsForTest`, `seedWebImSessionForTest`, `resolveWebMockToolApproval`, `WEB_MOCK_QUESTION_TRIGGER`, `WEB_MOCK_PLAN_EXIT_TRIGGER` are imported from `web-agent-client` by tests and by other modules. When their implementation moves, `web-agent-client.ts` re-exports them, exactly as both prior stages did. No test file is edited.

### Verify each step against the type checker and the contract test, not by reading

`webAgentClient` is annotated `: AgentService`, and `[ARCH-FE-003]` fails the build if the annotation goes missing. Any method dropped, renamed or given a different signature is a `tsc --noEmit` error, and `contract-conformance.test.ts` covers the surface besides.

## Risks / Trade-offs

- **An accessor changes semantics by accident** — e.g. returning a copy where the original returned the live array, so a caller's in-place mutation stops taking effect. This is a live risk here: `saveMessageFeedback` mutates a `ChatMessage` object in place through the array it gets back from `getSessionMessages`. Control: accessors return exactly what the previous expression returned, and the feedback path keeps reading through an accessor that hands back the live array.
- **Stream teardown ordering changes.** `cancelActiveStream` clears timeouts, deletes the stream, then publishes `cancelled`, which re-enters `applyStreamEvent` and deletes again. Moving it must preserve that re-entrancy. Control: the function moves whole, with its callers unchanged.
- **The accessor layer inflates the aggregate.** Control: the `[ARCH-FE-004]` subtree budget. A per-module fixed cost plus a bounded accessor set is the expected shape; anything materially larger means a method was rewritten.
- **A method is silently dropped** → `webAgentClient: AgentService` makes it a compile error.
- **Mock behavior drifts during a move** → `web-agent-client.test.ts`, the adapter-parity suites, and `contract-conformance.test.ts` run at every group boundary, and `npx playwright test` drives the browser-mode UI this adapter backs.

## Migration Plan

No deployment step. Each extraction group is an independently revertable commit that leaves `tsc --noEmit`, `npm run test` and `npm run contracts:check` green, so the work can stop at any group boundary without leaving the tree half-migrated.
