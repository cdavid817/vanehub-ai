## Why

A UX audit of the shipped workspace (`docs/ux-audit-report.md`) found that the modal surfaces users touch most often behave worse than the ones they touch least. The repository already owns a correct modal primitive, `src/components/ui/application-dialog.tsx`, which implements Escape-to-close, a Tab focus trap, focus return, and ARIA labelling. Twenty-five Settings files use it. The four modals in the main workspace — create session, scheduled tasks, batch-delete confirmation, and the CLI conflict dialog — are hand-rolled and implement none of it. Pressing Escape in the create-session dialog does nothing, which the audit confirmed against a running build.

Two workspace flows also fall back to `window.prompt` for text entry. In a Tauri WebView the native prompt ignores the application theme, cannot be localized, cannot validate input, and cannot be styled to match the surrounding dialog system.

Finally, the Loop Center first-run state renders three empty panels and a single centered sentence, with its only creation entry point a 24px icon button in a panel header. Every other empty surface in the product (chat welcome, notification center, work board) pairs an icon, a title, an explanation, and where applicable an action.

## What Changes

- Establish modal dialog behavior as a design-system requirement rather than a per-page choice, and route the four hand-rolled workspace modals through the shared `ApplicationDialog` primitive.
- Replace the two `window.prompt` call sites with in-application dialogs that carry validation, localization, and theme parity.
- Give the Loop Center a first-run empty state consistent with the rest of the product, including a primary creation action.
Explicitly out of scope: no new business capability is introduced. A workspace-wide search capability, a keyboard shortcut system, and configuration export were recorded as gaps in the audit's suggestion list and are not addressed here.

Two surfaces are also deliberately left alone. `src/loop-center/loop-definition-dialog.tsx` hand-rolls its modal but implements its own Escape handling and autofocus, so it is an inconsistency rather than a defect. `src/main-layout/session-context-panel.tsx` renders a positioned context menu rather than a modal; it has no Escape handling, but menu dismissal is a different contract from the one this change establishes.

Two other audit findings were resolved directly rather than here, because both already satisfy their existing requirements and needed no spec change. The top bar search entry, previously an input with no value binding or submit path, now reveals and focuses the session sidebar search that does run a query — the "equivalent accessible icon-triggered control" the responsive-width scenario already allows. The activity bar Help entry now opens the existing About settings page, which keeps the entry available without introducing a new destination.

## Capabilities

### Modified Capabilities

- `visual-design-system`: Add a modal dialog behavior requirement covering dismissal, focus containment, focus return, and labelling, plus a prohibition on browser-native text entry, so that new modals inherit both instead of re-deriving them.
- `main-layout-ui`: Require the create-session dialog, the scheduled-tasks dialog, and the batch-delete confirmation to use the shared dialog behavior, and require in-application text entry for category creation.
- `loop-management-ui`: Require a first-run empty state with an explanation and a primary creation action.

## Impact

- Frontend only. No Tauri command, service interface, DTO, or persistence change is intended.
- `src/main-layout/create-session-dialog-content.tsx`, `src/main-layout/scheduled-tasks-dialog.tsx`, `src/main-layout/session-sidebar.tsx`, `src/main-layout/main-layout.tsx`, `src/settings/pages/cli-conflict-dialog.tsx`, and `src/loop-center/loop-center.tsx` change.
- `src/components/ui/application-dialog.tsx` may gain optional sizing or footer affordances to absorb the create-session layout; its existing behavior contract does not change.
- Locale resources gain keys for the category-creation dialog and the Loop Center empty state across all five registered locales.
- Behavioral risk concentrates on the create-session dialog, which is the entry point for every session and is covered by existing e2e specs and documentation screenshots; both must be re-run.
