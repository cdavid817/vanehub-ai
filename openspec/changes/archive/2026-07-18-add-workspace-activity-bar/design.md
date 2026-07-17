## Context

The workspace route currently renders `TopBar`, a three-column grid, and `StatusBar`. The grid owns a 220px `SessionSidebar`, flexible chat content, and a 300px `SessionInfoPanel`; only the information panel has an explicit collapse state. `SessionSidebar` also owns the Settings and Help utility row, so global navigation is coupled to the visibility of session navigation.

This change affects the shared React workspace rendered by both Tauri and Web runtimes. It is presentation and local UI state only: session data continues to flow through the existing Agent service boundary, Settings continues to use the existing `/settings` route, and no native or adapter capability is needed for the Scheduled Tasks placeholder.

## Goals / Non-Goals

**Goals:**

- Introduce a stable, compact activity bar with top and bottom icon groups.
- Let the Session activity entry toggle the existing session sidebar without losing its mounted state.
- Return the collapsed sidebar width to the active chat workspace with the existing 200ms layout transition rhythm.
- Preserve current Settings navigation and session-management behavior in desktop and browser modes.
- Make every icon-only entry localized, keyboard accessible, and visually consistent in `futuristic` and `minimal` styles.
- Reserve a Scheduled Tasks entry that communicates its placeholder status without implying a working scheduler.

**Non-Goals:**

- Implement scheduled-task pages, routes, CRUD, recurrence rules, execution, history, persistence, or native scheduling.
- Persist the session-sidebar expanded state across reloads or application restarts.
- Change Agent service contracts, runtime adapters, Rust commands, SQLite schema, or session lifecycle behavior.
- Redesign the Settings center, Help content, session cards, information panel, top bar, or status bar beyond the navigation relocation required here.

## Decisions

### 1. Add a dedicated workspace activity-bar component

Create a presentation-focused component under `src/main-layout/` that renders two vertical groups: Session and Scheduled Tasks at the top, Settings and Help at the bottom. It accepts state and callbacks from `MainLayout`; it does not import runtime services or Tauri APIs.

Use existing `lucide-react` icons and shared semantic tokens. Each button has a stable square hit area, translated `title`/tooltip and accessible name, visible keyboard focus, and non-shifting hover/active styling. The Session entry exposes its expanded state with `aria-expanded` and an associated sidebar id.

Alternatives considered:

- Keeping utility actions inside `SessionSidebar` was rejected because those actions would disappear when the sidebar collapses.
- Embedding activity-bar logic directly in `MainLayout` was rejected because it would mix shell orchestration with reusable navigation presentation and push the layout file toward the project size limit.

### 2. Keep sidebar visibility as local shell state and preserve mounting

`MainLayout` owns `sessionSidebarCollapsed`, initialized to `false`. Activating the Session entry toggles this value. The sidebar remains rendered inside an overflow-clipped grid wrapper while the corresponding grid column transitions between `220px` and `0px`. The hidden wrapper becomes non-interactive and is marked hidden from assistive technology, while the mounted `SessionSidebar` retains its view mode and expanded-folder state.

The existing `infoPanelCollapsed` state remains independent, producing four desktop column combinations:

| Session sidebar | Information panel | Workspace columns |
|---|---|---|
| Expanded | Expanded | `220px / minmax(0, 1fr) / 300px` |
| Collapsed | Expanded | `0 / minmax(0, 1fr) / 300px` |
| Expanded | Collapsed | `220px / minmax(0, 1fr) / 0` |
| Collapsed | Collapsed | `0 / minmax(0, 1fr) / 0` |

Alternatives considered:

- Conditional rendering was rejected because it would reset the sidebar's local activity/group/archive and folder-expansion state.
- Persisting the preference in Settings or local storage was deferred because the requested behavior does not require cross-session persistence and adding persistence would enlarge the service and migration scope.

### 3. Place the activity bar beside the workspace grid

Keep `TopBar` above and `StatusBar` below the main workspace body. Inside that body, render the fixed-width activity bar at the far left and let the existing workspace grid fill the remaining width. Settings and Help are anchored at the bottom of the activity bar, which remains visible regardless of session- or information-panel collapse.

At widths up to 900px, retain the existing behavior that hides the information panel and uses session/sidebar plus main-content columns. At widths up to 640px, retain a single-column workspace: the expanded session sidebar appears before the chat content and the collapsed wrapper consumes no layout space. The activity bar remains fixed at the left at every supported width.

An overlay drawer was considered for narrow screens but deferred because the current product already uses a stacked sidebar at that breakpoint; preserving it avoids introducing focus trapping, backdrop behavior, and a second sidebar interaction model in this scoped change.

### 4. Relocate utilities without changing their destinations

Remove Settings and Help rendering from `SessionSidebar` and remove the now-unneeded Settings callback from that component. The activity bar invokes the existing `MainLayout.onOpenSettings`, which continues to navigate to `/settings`. Help is relocated with its current scope; this change does not introduce a new Help route or external URL.

The Scheduled Tasks button does not navigate or become selected. Activating it raises a localized, non-blocking “coming soon” notification through the existing frontend notification context. No service boundary call is made, and no mock/native behavior can diverge.

Alternatives considered:

- Adding an empty Scheduled Tasks route was rejected because it would imply a supported product surface and create routing/testing commitments outside the placeholder requirement.
- Disabling the button was rejected because disabled controls do not consistently expose tooltips or keyboard feedback; an enabled placeholder with explicit feedback is more discoverable and accessible.

### 5. Verify behavior at component and workspace levels

Add focused component coverage for icon labels, top/bottom grouping, Session toggle callbacks, expanded accessibility state, Settings callback, and Scheduled Tasks feedback. Extend workspace/E2E coverage to verify that collapsing preserves sidebar state, releases horizontal space, and can be reversed. Existing localization guardrails cover synchronized Chinese and English strings; visual QA covers both registered styles and representative desktop, 900px, and 640px widths.

No adapter conformance or Rust test changes are required because the design does not cross the frontend service boundary.

## Risks / Trade-offs

- [A zero-width mounted sidebar can leak focusable descendants into keyboard navigation] → Apply an inert/non-interactive hidden state to the wrapper and verify keyboard traversal while keeping the React subtree mounted.
- [Adding a fixed activity bar reduces the chat width at all sizes] → Keep the bar compact and allow the 220px session column to collapse completely.
- [Users may mistake the Scheduled Tasks icon for a completed feature] → Use a localized tooltip that identifies it as coming soon and repeat that message in non-blocking click feedback without navigating.
- [Independent left and right collapse states increase CSS combinations] → Drive grid columns from explicit data attributes and cover all four desktop combinations in layout tests.
- [The narrow layout still stacks the expanded session sidebar above chat] → Preserve the existing bounded mobile sidebar height and make the new Session toggle permanently reachable from the activity bar.
- [Local-only collapse state resets after reload] → Accept the predictable default-expanded behavior for this change; persistence can be proposed separately if user feedback warrants it.

## Migration Plan

1. Add the activity-bar component and new localized labels without changing runtime interfaces.
2. Move Settings and Help ownership from `SessionSidebar` to `MainLayout`/the activity bar.
3. Introduce the session collapse state and update workspace grid styles for independent left/right collapse combinations and existing breakpoints.
4. Add component, localization, responsive, and E2E regression coverage; verify both visual styles.
5. Roll back by restoring the utility row and original grid definitions; there is no persisted data or backend migration to reverse.

## Open Questions

None for this change. Scheduled-task functionality and any Help destination require separate proposals when their product behavior is defined.
