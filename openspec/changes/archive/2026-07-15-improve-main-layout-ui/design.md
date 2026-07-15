## Context

The current workspace shell is composed by `src/main-layout/main-layout.tsx`, with sidebar, canvas, and info panel behavior split across child components. The visible issues are layout-oriented: fixed minimum heights, six sidebar tool shortcuts competing with session navigation, no collapse state for the info panel, and no independent scroll containment for long lists or panel content.

This change is intentionally frontend-only. Both desktop and browser modes render the same React shell, so the implementation should keep behavior in React state and Tailwind classes without adding runtime adapter calls, Tauri `invoke()` usage, Rust commands, database changes, or dependencies.

## Goals / Non-Goals

**Goals:**

- Implement a balanced three-panel workspace layout with expanded columns `220px / 1fr / 300px` and collapsed columns `220px / 1fr / 0px`.
- Keep the sidebar, main content, and info panel aligned to the same available height between the top bar and status bar.
- Move the six sidebar tool shortcuts out of the sidebar footer while keeping Settings, visual style switching, and Help pinned at the bottom.
- Add session card agent markers, activity/group view modes, activity counts, folder expand/collapse state, and internal sidebar scrolling.
- Remove fixed canvas minimum height behavior and make the canvas flex inside the central panel while keeping the composer fixed and usable.
- Add an info panel collapse/expand interaction with a 200ms CSS transition and preserved mounted state.
- Replace info panel tabs with keep-alive Agent Info, Files, and Changes tabs, including Agent Info progress summary and panel-internal scrolling.

**Non-Goals:**

- No service interface, Tauri adapter, Web adapter, Rust command, SQLite, or CLI launch behavior changes.
- No new data persistence for selected sidebar mode, expanded folders, selected info tab, or collapsed panel state.
- No new settings page implementation beyond keeping Settings as the entry point for the removed tool shortcuts.
- No dependency additions and no UI component library changes.

## Decisions

### Keep layout state in `main-layout.tsx`

Use local React state in `MainLayout` for sidebar view mode, expanded folders, active info tab, and info panel collapse state. This matches the requested single-file scope and avoids turning a visual refactor into a service or persistence change.

Alternative considered: move state into the existing child components. That would preserve current component boundaries, but it would spread the requested one-file refactor across multiple files and make the three-panel grid behavior harder to reason about.

### Use derived UI groupings from the existing workspace snapshot

Derive session activity buckets and folder groups from the existing `workspace.conversations` data in render-time helpers inside `main-layout.tsx`. Agent type markers should be inferred from known fields where available and fall back to a neutral marker for unknown or future agent types.

Alternative considered: extend workspace service data with explicit activity and folder fields first. That may be useful later, but this change is scoped to pure frontend UI and should not modify service contracts.

### Preserve components with hidden inactive panels

For info panel tabs, render all three tab bodies and toggle visibility with CSS classes instead of conditionally mounting only the active tab. The same principle applies to panel collapse: collapse the column width and visually hide/clamp the panel, but keep the info panel subtree mounted so form inputs and selected tab state remain intact.

Alternative considered: conditional rendering for inactive tabs and the collapsed panel. That is simpler, but it violates the requirement to preserve local tab and form state.

### Contain overflow at panel boundaries

Make the workspace grid a `min-h-0` flex child between top and status bars, then give the sidebar list and info panel body their own `overflow-y-auto` containers. The central canvas should use `flex-1 min-h-0 overflow-hidden`, while the composer uses a non-shrinking footer area.

Alternative considered: let the document scroll. That fails the requirement that long sidebar and info-panel content scroll internally without moving the whole main interface.

### Use Tailwind-only transitions and dimensions

Represent expanded and collapsed panel widths through conditional Tailwind arbitrary grid classes and use `transition-[grid-template-columns] duration-200` for the shell. Use `duration-200` on the info panel opacity/translate/width affordances to keep the collapse and expand motion aligned.

Alternative considered: inline styles for dynamic grid templates. The project style constraints disallow inline styles, and the required values are fixed enough for Tailwind classes.

## Risks / Trade-offs

- [Risk] Keeping all tab content mounted increases DOM size compared with conditional rendering. -> Mitigation: tab content is small and local to the info panel; this is acceptable for state preservation.
- [Risk] Inferring agent type from current mock/session fields may be imperfect until real workspace session metadata exists. -> Mitigation: provide a neutral fallback marker and keep the mapping additive for future service-backed fields.
- [Risk] Folding child component behavior into `main-layout.tsx` can push the file toward the 300-line guideline. -> Mitigation: keep helper arrays and rendering blocks compact; if the final implementation exceeds the limit, split presentation helpers only if required by lint or maintainability.
- [Risk] Collapsing the info panel to `0px` can leave borders, padding, or focusable controls visible if only the grid column changes. -> Mitigation: pair grid width collapse with overflow clipping, opacity/pointer-events changes, and a separate edge expand button.

## Migration Plan

Implement the UI refactor in `src/main-layout/main-layout.tsx` without changing runtime data sources. Validate in browser/Web mode through the normal frontend build and visual smoke testing. Rollback is a single-file revert because no service, backend, database, or dependency migration is involved.

## Open Questions

- The six removed sidebar tool shortcuts will route through settings as a unified destination, while Settings, visual style switching, and Help remain pinned in the sidebar footer.
- If future workspace data exposes explicit session folder and agent type fields, the derived mock grouping should be replaced by those fields without changing this UI contract.
