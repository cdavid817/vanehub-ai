## 1. Diagnosis

- [x] 1.1 Trace how the open transcript learns about new messages (event subscription vs. query invalidation) and identify why runtime-originated `MessageCreated`/streaming events do not reach it while the turn status bar's subscription does. Finding: `applyChatEvents` can only mutate rows already in the react-query cache — stream events carry no role/speaker/sequence and cannot create rows, and neither subscription refetched the list, so every event for an unknown message id was silently dropped; composer flows worked only because their optimistic updates seeded the rows first.

## 2. Implementation

- [x] 2.1 Make the message list react to runtime-originated message lifecycle events for the open session: `hasEventsForUnknownMessages` (services/chat-events.ts) detects events targeting unknown rows, and both subscription points — `use-session-stream-events.ts` (extracted from `use-main-layout-model.ts`) and `use-active-session-chat.ts` — refetch the list when it fires, plus one settle-time refetch after a stream that ever touched unknown rows. Adapter-agnostic: both the Tauri and Web/mock adapters deliver events through the same `subscribeMessageEvents` boundary.
- [x] 2.2 Verify the fix does not double-render composer-originated flows or break optimistic composer updates (full vitest suite green; refetch only triggers for unknown ids, which optimistic seeding prevents for composer sends).

## 3. Coverage

- [x] 3.1 Component test: a message event for the open session that did not come from the composer updates the rendered list (`use-active-session-chat.test.tsx`), plus unit coverage for `hasEventsForUnknownMessages` (`chat-events.test.ts`).
- [x] 3.2 Desktop e2e: after a programmatic `send_message`, the open transcript shows the backend-originated turns (`message-speaker` assertion in the multi-agent longrun layer; verified 5/5 green on 2026-08-25 with per-stage screenshots showing the rendered thread).

## 4. Verification

- [x] 4.1 Run the full validation command set from AGENTS.md, then `openspec validate fix-chat-transcript-backend-message-updates --strict` (2026-08-25: lint:ci, vitest suite, build, tsc green; `npx playwright test` 155/156 with the one multi-agent-session failure reproducing as a load flake — 5/5 green re-run in isolation; desktop verification via the multi-agent longrun layer, 5/5 green with the transcript assertion).
