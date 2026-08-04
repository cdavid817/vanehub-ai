## Context

Global Skill Settings already uses stable Agent navigation and granular CLI/API binding services. In a selected-Agent view, however, Assigned and Available are rendered as two full-width vertical sections, and each mutation is represented by a native checkbox whose label changes between Assign and Remove. A checkbox conventionally represents locally editable state, while this control immediately performs a filesystem or API binding mutation, so its affordance and failure behavior are easy to misread. Lifecycle actions are also repeated in this task-focused view.

The change is frontend-only and shared by Tauri desktop and Web/mock runtimes. React must continue calling the existing `AgentService` boundary; the Tauri adapter, Web adapter, Rust commands, SQLite schema, and native mount semantics remain unchanged.

## Goals / Non-Goals

**Goals:**

- Make Agent-to-Skill assignment direction explicit and distinguish it from global enablement.
- Reduce vertical scanning with a responsive two-column selection board on wide layouts.
- Keep stable binding state, pending state, and row-owned errors visible.
- Focus selected-Agent rows on assignment and preview while retaining full lifecycle controls in All Skills.
- Preserve keyboard, accessible-name, long-label, theme, and narrow-layout behavior.

**Non-Goals:**

- Add batch bind/unbind operations or partial-failure orchestration.
- Add drag-and-drop assignment, which is less discoverable and less keyboard-friendly.
- Change CLI mount ownership, API prompt injection, global enablement, persistence, or service contracts.
- Copy ClowderAI's scope model or per-mount-point policy UI.

## Decisions

### Decision: Use an explicit transfer-board presentation

Selected-Agent views render Assigned and Available as sibling panels in a responsive grid, with Assigned first in document order. Each panel owns its heading, count, empty state, and bounded list. Below the wide breakpoint the same panels stack, preserving a single accessible reading order without introducing a second mobile state model.

This keeps both destinations visible, unlike tabs, while avoiding the long single-column scan of the current layout. The existing partition helper remains the source of truth.

### Decision: Replace immediate-action checkboxes with action buttons

Available rows use an explicit Assign button with a directional icon; Assigned rows use an explicit Remove button. Button accessible names include the selected Agent display name, while service calls continue to use the stable Agent id. Pending actions disable only the affected row and show progress text/icon without optimistic movement.

A switch or checkbox was rejected because it suggests a cheap local state toggle and obscures the fact that CLI assignment can perform filesystem work. Drag-and-drop was rejected because it hides the primary action and complicates keyboard and touch operation.

### Decision: Separate assignment from lifecycle administration

All Skills keeps enablement, preview, edit, and guarded delete. Selected-Agent rows keep preview plus assignment state/action, but omit edit and delete. This makes the Agent view a relationship editor and reduces destructive-action density; users can return to All Skills for lifecycle changes.

### Decision: Preserve existing mutation and error ownership

The board delegates one row action to the existing binding mutation. It does not optimistically move a row. Success invalidates the existing overview query; failure leaves the row in its original panel and displays the backend error on that row. CLI and API labels remain distinct through existing binding-state derivation and localization.

## Risks / Trade-offs

- [Risk] Two columns can become narrow with long localized labels. → Use a wide-layout breakpoint, min-width-safe content, wrapping actions, and stacked narrow layout tests.
- [Risk] Removing edit/delete from Agent rows may require one extra navigation step. → Keep preview available and make the All Skills lifecycle responsibility explicit in the view description.
- [Risk] Assigned and Available lists can still become long. → Preserve the shared search/filter toolbar and bounded row density; virtualization remains unnecessary for the current catalog size.
- [Risk] Users may expect batch selection from a transfer-board visual. → Use action-oriented rows rather than selectable checkboxes and explicitly keep batch operations out of scope.

## Migration Plan

No data migration is required. Deploy the frontend component and localization changes together. Rollback restores the previous selected-Agent list component without changing persisted assignments.

## Open Questions

None. Batch assignment can be proposed separately if catalog scale and service-level atomicity justify it.
