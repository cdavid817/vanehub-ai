## Context

The main layout already renders a Bell icon and a decorative unread dot, but there is no notification state, publishing contract, transient presentation, or history view. Feature components currently choose their own inline success/error feedback, which will become inconsistent as agent, session, settings, and background-operation workflows grow.

The first version must work identically in the Tauri and Web runtimes, use only React state/context, preserve both existing visual themes, and avoid expanding the native service boundary before persistence or operating-system delivery is required. The implementation is informed by Clowder AI's bounded toast store and timer cleanup, but replaces Zustand with React Context and separates transient toast visibility from notification history.

## Goals / Non-Goals

**Goals:**

- Provide one typed API that any descendant React component can use to publish and manage user-facing notifications.
- Present a bounded bottom-right toast stack while retaining recent entries in a Bell notification center after a toast expires.
- Support global and session-scoped notifications, unread state, four semantic tones, accessible controls, and localized framework text.
- Keep the first version runtime-neutral and document a compatible path to persistence and native delivery.
- Integrate at least one real session workflow so the framework is exercised by product behavior rather than existing only as infrastructure.

**Non-Goals:**

- SQLite persistence, cross-window synchronization, Web Push, Tauri operating-system notifications, and notification permission management.
- A dedicated notification route, server-side inbox, pagination, search, or per-category preference controls.
- Persisting translated notification copy or replaying notifications after an application restart.
- Replacing inline validation that is more actionable beside the affected field.

## Decisions

### Use a reducer-backed React provider as the application contract

`NotificationProvider` will be mounted inside the existing settings/theme providers and expose `useNotifications()`. The public contract will include notification records plus `notify`, `remove`, `markRead`, `markAllRead`, and `clear`. A separate private presentation context will expose toast exit/hide operations only to a `NotificationHost`, keeping lifecycle controls out of the business-component hook.

This follows the project's React-only state constraint and keeps feature components independent of toast markup. A module-level event bus was rejected because it makes lifecycle/testing implicit, and Zustand from the Clowder reference was rejected because it violates the project's state-management constraint.

### Separate retained history from transient toast visibility

Each notification record contains a generated id, semantic type, title, optional message, creation time, read state, scope, duration, and toast lifecycle. Publishing marks an item unread and toast-visible. Expiration hides only the toast; it does not remove the recent-history record. Records are bounded to the 20 newest entries, and no more than four toasts are visible at once.

This differs deliberately from Clowder AI, where removing a toast also removes the store item. The separation is necessary because VaneHub's Bell control is a notification center, not only a toast viewport.

### Scope presentation without discarding history

The scope is a discriminated union: `global` or `session` with a stable session id. The viewport shows global notifications and notifications matching the active session. The notification center shows all retained entries, so changing sessions never silently loses a message.

The initial center does not resolve session titles because that would couple the framework to session queries. A future metadata resolver can enrich scoped entries without changing the scope identity contract.

### Keep first-version notification content as localized display strings

Framework-owned labels such as "Notifications", "Mark all as read", empty state, relative-time fallback, and accessible names live in both `zh-CN` and `en` resources. Producers use the existing `useI18n()` hook and pass already-localized `title` and `message` strings to `notify`.

This short-term choice keeps the API small and avoids storing translation-library concerns in generic state. Before notifications become persistent or originate from Rust/background events, the content type should be extended to a typed translation descriptor (`key` plus interpolation values and fallback) so locale changes can re-render retained entries correctly.

### Use existing theme tokens and Lucide icons

Toast and center surfaces use the existing panel, border, muted, hover, success, warning, and danger tokens. Semantic status is conveyed by icon, text, and color together. The center is a compact anchored popover in the top bar; the toast viewport is fixed to the lower-right with responsive insets and width constraints.

No new UI library or inline style is introduced. The futuristic theme retains translucent/dark surfaces and the minimal theme retains crisp/light surfaces through shared tokens rather than duplicated component variants.

### Keep this version frontend-only

Because state is intentionally in memory, neither `agent-service.ts` nor the Tauri/Web adapters need a new method. When persistence is added, notification storage and retrieval must be introduced through aligned service interfaces, implemented by both runtime adapters, and backed by SQLite only in Rust for desktop mode. OS delivery must be a separate capability with explicit permission and platform behavior.

## Current Short-Term Implementation

| Area | First-version behavior | Intentional boundary |
| --- | --- | --- |
| State | Public React Context + private presentation Context over one reducer, newest 20 records | Lost on reload/restart and not shared across windows |
| Delivery | In-app toast and Bell popover | No OS notification, Web Push, or background delivery |
| Toast lifecycle | Auto-hide by duration, maximum four visible | No pause-on-hover or progress indicator |
| History | Recent entries, unread count, mark/remove/clear | No pagination, filtering, or durable read state |
| Scope | Global and stable session-id scope | No agent/operation category model or session-title resolver |
| Content | Producers pass localized strings | Existing entries do not retranslate after locale changes |
| Actions | Notification management controls only | No deep-link/action callback persistence |
| Quality gates | `npm run lint` performs strict TypeScript checking; notification and repository Playwright suites follow current UI workflows | A dedicated ESLint policy can be introduced as a separate tooling change |

## Optimization Roadmap

1. Introduce a versioned notification service contract and SQLite schema with retention policy, read timestamps, source/category, deduplication key, and migration tests; keep Web adapter behavior aligned with an in-memory or HTTP implementation.
2. Replace display-string-only content with typed translation descriptors and safe serializable interpolation values so persisted/runtime-originated notifications follow the active locale.
3. Add an event bridge for Rust task/session lifecycle events, with deduplication and throttling for noisy progress updates; persistent diagnostic data continues through unified logging rather than notification storage.
4. Add optional Tauri OS notifications and Web Notifications behind user preferences, permission checks, foreground suppression, and platform-specific tests.
5. Add actions/deep links through serializable action identifiers routed by the application, plus category filters and per-category delivery preferences.
6. Add cross-window synchronization, pagination/virtualization, accessibility announcements tuned by severity, pause-on-hover, and deterministic timer scheduling for large volumes.

## Risks / Trade-offs

- [Localized display strings become stale after a locale switch] -> Keep all framework chrome reactive now and migrate payloads to translation descriptors before persistence.
- [A global context can cause broad renders] -> Memoize the context value, keep records bounded, and split state/actions contexts if profiling later shows meaningful churn.
- [Toast timers can update unmounted components] -> Own timers in toast components, clear them on cleanup, and make reducer actions idempotent.
- [High-frequency producers can crowd the interface] -> Bound visible toasts and history now; add deduplication/throttling before wiring noisy background events.
- [Session-scoped notifications can be hidden while another session is active] -> Keep them visible in the all-session notification center and retain unread state.

## Migration Plan

1. Mount the provider without changing existing service contracts.
2. Replace the top-bar decorative dot with real unread state and add the center/viewport.
3. Wire a focused session workflow and migrate other components incrementally through `useNotifications()`.
4. Rollback is removal of the provider and UI integration; no persistent data or native migration requires cleanup.

## Open Questions

- The exact categories and retention period for durable notifications should be decided with the persistence change.
- Native notification permission timing and default foreground behavior require a separate product decision.
- Action routing should be designed together with stable application routes and operation identifiers.
