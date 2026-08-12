## Context

See `proposal.md` for motivation. The sidebar already holds complete `Session` records, but `useMainLayoutModel` currently sends only the id to `switchSession`, waits for the mutation, and then invalidates the entire `sessions` query family. `MainLayout` also increments its activation key before the active-session query changes. This produces avoidable latency, broad refetching, and an early workspace reset.

The optimization must work identically with the Tauri and Web adapters and must keep persistence behind `AgentService`.

## Goals / Non-Goals

**Goals:**

- Make the visible active session change in the same React interaction as the sidebar click.
- Preserve correctness when persistence fails or rapid switch requests finish out of order.
- Avoid redundant resets, subscriptions, and broad query-family refetches.
- Reuse cached session-scoped conversation data when revisiting a session.
- Add deterministic regression coverage instead of relying only on subjective timing.

**Non-Goals:**

- Changing the native/Web `switchSession` contract or SQLite workflow schema.
- Keeping every session workspace mounted simultaneously.
- Changing terminal lifecycle semantics when leaving a live CLI session.
- Adding a new state-management dependency.

## Decisions

### 1. Optimistically reconcile the active-session query

The switch mutation accepts the already-loaded `Session` record. Its optimistic phase cancels the exact active-session query, records the previous value and a monotonically increasing request id, then places the selected record into `['sessions', 'active']`. This changes the sidebar marker and workspace without waiting for desktop IPC or Web persistence.

Alternative considered: introduce a second local `selectedSession` state. Rejected because it would create two active-session sources and duplicate reconciliation across every consumer.

### 2. Make rollback and completion latest-request-wins

Mutation callbacks compare their request id with the latest issued switch. Only the latest request may commit its returned canonical record or roll back after failure. Older completions are ignored in the visible cache, preventing response-order races during rapid selection.

Alternative considered: serialize clicks until each switch completes. Rejected because it preserves correctness by making navigation feel slower and prevents users from correcting an accidental selection immediately.

### 3. Remove broad invalidation from the switch path

A successful switch writes the canonical response to the exact active-session query and leaves session lists, archived sessions, categories, and searches intact because selection does not mutate those records. Other session mutations continue using their existing invalidation behavior.

Alternative considered: retain `invalidateSessions` after the optimistic update. Rejected because it immediately marks multiple unrelated queries stale and can refetch the active record that was just reconciled.

### 4. Reset workspace state only after a real active-id change

`MainLayout` increments the activation key only when the model reports a different effective active-session id. Clicking the current card becomes a no-op. Session-scoped draft and message state continue to reset from the existing active-id effect, so no new cross-session state leakage is introduced.

### 5. Test responsiveness through state ordering

Unit tests hold the persistence promise unresolved and assert that the target session is already visible, then cover rollback and out-of-order completion. Playwright verifies that rapid sidebar navigation ends on the last selected session. This provides deterministic evidence without depending on machine-specific millisecond thresholds.

## Risks / Trade-offs

- [Persistence failure briefly displays a session that cannot become active] → Roll back only the latest request and show the existing localized error notification.
- [Native session events may arrive after an optimistic update] → Keep the cache write canonical and ignore older mutation completions; existing recovery events remain independently scoped.
- [Cached conversation content may be stale] → React Query may refresh it in the background; session ids keep cached data isolated.
- [A live terminal has provider-specific leave behavior] → Preserve the existing session-id-driven terminal activation and service lifecycle logic.

## Migration Plan

No data migration is required. Deploy the frontend change with both adapters unchanged. Rollback restores the previous mutation and activation-key behavior without affecting persisted sessions.
