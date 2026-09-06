## 1. Layout

- [x] 1.1 Container-query root on tab panels; changes, documents, terminal records, review, and traces sized by panel width with bounded stacked lists.
- [x] 1.2 Side columns only while a drawer or comparison is open.
- [x] 1.3 Waterfall bar column measured on its own, header and row padding aligned, edge ticks flush and unwrapped, size writes deferred a frame.
- [x] 1.4 Stylesheet narrow breakpoints aligned with Tailwind's strict variants.

## 2. Presentation

- [x] 2.1 No glyph for an unknown tab count.
- [x] 2.2 Terminal-records filters folded behind a localized "more filters" control.
- [x] 2.3 Compact three-column report tiles.
- [x] 2.4 Localized run start time in the information panel; Linux monospace faces in the terminal font stack.

- [x] 2.5 Files toolbar on its own full-width row above the tree and preview, grouped by separators, showing the selected path.
- [x] 2.6 Plain-text document preview padded like the Markdown preview inside the viewer card.

## 3. Width and height loops

- [x] 3.1 Timeline column and waterfall viewport with zero minimum width, so the row minimum width cannot lock the panel wider than its container.
- [x] 3.2 Waterfall list wrapper with a definite height, so a remounted virtual list measures its viewport and renders rows.
- [x] 3.3 Settings page scroll container positioned, so screen-reader-only labels stay inside its clip and cannot scroll the window.

## 4. Follow-up defects from the desktop client

- [x] 4.1 File tree indented by empty steps with a hairline guide instead of printed middle dots.
- [x] 4.2 xterm viewport painted from the terminal background variable, closing the dark ring around a light terminal in WebKitGTK.
- [x] 4.3 Shell surface sends sizes only to a running Shell, sends one size when the Shell starts accepting input, and drops refusals that land after the surface is gone.
- [x] 4.4 Work board columns with a zero minimum height, a narrower minimum width, and a visible horizontal scroll track.
- [x] 4.5 Terminal usage poll records nothing for a tick that persisted nothing.
- [x] 4.6 Session rows constrained to the sidebar column (WebKitGTK sized the `<button>` to a long title and pushed it under the conversation panel); sweep session title lengthened so the audit covers it.
- [x] 4.7 Badges never shrink or wrap; work board card action row rebuilt as two aligned 32px groups (arrows around a stretching stage select, edit and archive right-aligned).

## 5. Verification

- [x] 5.1 Unit suites, lint, build, and the workspace, observability, review, terminal-history, and waterfall-remount Playwright specs.
- [x] 5.2 Desktop UI ratio sweep (`npm run test:desktop:ui-ratios`): every session tab and Basic Configuration at 1440×900, 1366×768, 1280×820, 1180×740, and the 1100×700 window floor, with the layout audit reporting zero findings on Linux (WebKitGTK). Windows and macOS NOT RUN.
