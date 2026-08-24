## 1. Diagnosis

- [ ] 1.1 Trace how the open transcript learns about new messages (event subscription vs. query invalidation) and identify why runtime-originated `MessageCreated`/streaming events do not reach it while the turn status bar's subscription does.

## 2. Implementation

- [ ] 2.1 Make the message list react to runtime-originated message lifecycle events for the open session, in both the Tauri and Web/mock service adapters.
- [ ] 2.2 Verify the fix does not double-render composer-originated flows or break optimistic composer updates.

## 3. Coverage

- [ ] 3.1 Component test: a message event for the open session that did not come from the composer updates the rendered list.
- [ ] 3.2 Desktop e2e: after a programmatic `send_message`, the open transcript shows the user message and the seat reply (extend the multi-agent UI or longrun layer).

## 4. Verification

- [ ] 4.1 Run the full validation command set from AGENTS.md (UI behavior changed: include `npx playwright test` and the desktop layers), then `openspec validate fix-chat-transcript-backend-message-updates --strict`.
