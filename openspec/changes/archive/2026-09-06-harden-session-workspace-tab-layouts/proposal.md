## Why

A visual sweep of every session workspace tab found layouts that were sized by the browser window while the panel they live in sits between two collapsible side columns. Side-by-side layouts were forced into panels far too narrow for them, columns were reserved for drawers that were not open, the trace waterfall scaled the whole viewport width onto a bar column that is 18rem narrower and so pushed bars and the last tick off screen, an unknown-count badge drew an unexplained glyph next to every tab of a real session, the desktop terminal fell back to a CJK monospace face, and the information panel showed a raw ISO timestamp.

## What Changes

- Give every tab panel a container-query root and size the changes, documents, terminal-records, and traces layouts by panel width; stack with bounded list heights in narrow panels.
- Reserve a side column only while a drawer or comparison is open (terminal records, traces).
- Measure the waterfall's bar column on its own, align header and row padding, keep the first and last tick labels flush and unwrapped, and defer size writes to the next frame.
- Render nothing for an unknown tab count; the spoken description keeps the reason.
- Fold the terminal-records filter chips behind a "more filters" control that unfolds automatically while a filter is active; keep the search field visible.
- Compact the report metric tiles into three columns with smaller figures; six columns only in a wide panel.
- Format the information panel's run start time in the active locale; add Linux monospace faces to the terminal font stack.
- Align the stylesheet's narrow-width breakpoints with Tailwind's strict `max-[…]` variants so the information panel never wraps into a second grid row.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-workspace-tabs`: Add panel-width responsive layout and badge presentation requirements.

## Impact

- `src/session-workspace/{session-tabs,session-tab-bar,changes-tab,documents-tab,terminal-tab,review-center,execution-timeline-tab,trace-waterfall,trace-span-row,trace-run-list,execution-record-toolbar,report-figures,report-section,terminal-theme}.tsx`, `src/main-layout/session-evidence-summary.tsx`, `src/styles.css`, two locale keys, one e2e step. No service or native change.
