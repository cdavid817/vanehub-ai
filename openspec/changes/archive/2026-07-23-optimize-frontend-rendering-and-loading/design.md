## Context

The Prompt Hooks page currently maps every filtered hook to a large interactive card, and the Logs tab maps every loaded log entry to an article. Both surfaces therefore grow the DOM in direct proportion to loaded data. Settings pages, Loop Center, and session tab modules are also statically imported, so Vite includes their module graphs in initial chunks even though the runtime already delays some component mounting.

The change applies to the shared React frontend and therefore behaves the same in Tauri desktop and Web/mock modes. Existing data access remains behind `agentService`; no Tauri adapter, Web adapter, Rust command, SQLite, or native logging changes are needed.

## Goals / Non-Goals

**Goals:**

- Bound mounted Prompt Hook cards and Agent log rows to the visible viewport plus a small overscan.
- Preserve current ordering, filtering, pagination, operations, accessibility, and keep-alive state.
- Let users seek logs by timestamp without manually loading every intermediate page.
- Move heavy, non-default frontend modules out of the initial bundle and load them on first use.
- Provide localized loading and recoverable module-load failure states.

**Non-Goals:**

- Changing Prompt Hook or log service contracts, storage, retention, redaction, or export behavior.
- Adding server-side timestamp queries or unbounded background log retrieval.
- Virtualizing small Prompt Hook inventories where ordinary document flow is simpler.
- Unmounting visited settings pages or session tabs to reclaim memory.
- Introducing a new application state-management library.

## Decisions

### Use TanStack Virtual for variable-height windowing

Add `@tanstack/react-virtual` and use vertical virtualizers with stable item keys and measured element heights. Prompt Hooks are virtualized as responsive rows containing one card on narrow layouts and two cards on `xl` layouts; changing the column count remeasures rows without changing hook order. Logs are virtualized as individual variable-height articles because messages and structured context can differ substantially in height.

Prompt Hooks keep the existing non-virtual grid when the filtered result contains 500 or fewer items. Above 500, the scroll region mounts only visible rows plus four overscan rows on each side. Logs use ten overscan rows on each side and represent the existing load-more control as a terminal virtual item. Static visual styling remains in Tailwind classes; runtime geometry is limited to the positioning values required by the virtualizer.

Alternatives considered:

- `content-visibility: auto` leaves every React component and DOM node mounted, so it does not address component or DOM growth.
- Fixed-height virtualization is simpler but clips or wastes space for cards and log contexts with variable content.
- Replacing pagination with an unbounded stream would increase native I/O and conflict with the existing bounded retrieval contract.

### Keep filtering and pagination outside the virtualizer

Existing hooks continue to produce the filtered, sorted Prompt Hook collection and loaded de-duplicated log collection. Virtualizers receive those arrays as immutable ordered inputs and use hook/log ids as item keys. Filter or grouping changes scroll the Prompt Hook viewport to the start. Log level or search changes clear pagination and reset the log viewport before loading the first matching page.

This keeps rendering optimization separate from domain and service behavior and prevents virtual indices from becoming persistent identifiers.

### Define bounded timestamp seek semantics

The Logs toolbar gains a localized date-time input and locate action. Because the existing service is cursor-based and newest-first, the client searches loaded entries for the first entry whose timestamp is at or before the requested timestamp. If the target is older than the loaded tail and another cursor exists, one locate action may retrieve at most ten additional pages sequentially, de-duplicating entries through the existing id rule after each page.

When a target is found, the virtualizer scrolls it into view and programmatically focuses its article without changing active filters. If ten pages are exhausted while more data remains, the UI reports that the target is not yet loaded and offers the same action to continue. If the requested time is outside the fully known range, the UI reports that no matching entry exists. Newer-than-newest targets do not silently select an unrelated row.

Alternatives considered:

- A native timestamp query would be faster for deep history but expands both runtime adapters and Rust commands beyond this rendering-focused change.
- Automatically reading until exhaustion could block the UI and issue unexpectedly large native reads.

### Separate code loading from component lifetime

Use `React.lazy()` at existing feature registries and destination boundaries:

- Settings page registrations provide lazy components for heavy pages such as Agents and Prompt Hooks. The settings shell still adds a page to its visited set on first activation and keeps the resolved component mounted with CSS visibility changes.
- Loop Center is dynamically imported when the Loops destination is first selected. Returning to Sessions preserves the already-mounted session workspace.
- Session Chat remains the eager default panel. Non-default tab modules are lazy imports activated through the existing mounted-tab set and remain mounted until the active session changes.

Named exports are adapted to lazy default exports in import callbacks rather than changing public component exports. Each boundary uses `Suspense` with a size-stable localized loading state and the existing `react-error-boundary` dependency for a localized retry action. A retry recreates the lazy import boundary; it does not reset unrelated visited pages or tabs.

Alternatives considered:

- Lazy-loading every small component would create excessive chunks and loading boundaries with little startup benefit.
- Conditional rendering without dynamic `import()` delays mounting but does not remove code from initial chunks.
- Unmounting hidden visited surfaces saves more memory but violates established state-preservation requirements.

### Verify behavior and bundle boundaries

Component tests cover threshold selection, stable keys, range rendering, filter resets, timestamp pagination limits, and lazy state preservation. Playwright covers scrolling to offscreen Prompt Hooks/log entries, timestamp focus, retryable loading states, and desktop-sized/narrow layouts. The production build output is inspected by an automated assertion or deterministic manifest test to confirm designated feature modules are emitted outside the initial entry chunk.

## Risks / Trade-offs

- [Measured variable-height content can shift after expansion or localization] → Call the virtualizer measurement hook on rendered rows and preserve stable ids so scroll correction remains deterministic.
- [Responsive Prompt Hook columns can invalidate row measurements] → Recompute row grouping and remeasure when the breakpoint changes.
- [Deep timestamp seeks require repeated user actions after ten pages] → Show explicit progress/range feedback and keep the action repeatable without discarding loaded entries.
- [Lazy imports can fail because of stale assets or transient loading errors] → Use a local error boundary with retry and keep the surrounding shell operational.
- [Visited lazy modules remain mounted and still consume memory] → Preserve current product semantics; this change targets startup cost and large-list DOM growth rather than full memory eviction.
- [Virtualized content is absent from the DOM until scrolled] → Preserve semantic list metadata, keyboard scrolling, focus the located log row, and verify screen-reader labels and item positions.

## Migration Plan

1. Add the virtualization dependency and reusable loading/error/virtual-list helpers.
2. Convert Prompt Hooks and Logs while preserving their current service requests and user operations.
3. Introduce lazy boundaries for settings pages, Loop Center, and non-default session tabs.
4. Add regression and bundle-splitting tests, then run the full frontend, Rust, and OpenSpec validation suites.
5. Roll back by restoring eager imports and ordinary mapped lists; no stored data or backend migration is involved.

## Open Questions

None.
