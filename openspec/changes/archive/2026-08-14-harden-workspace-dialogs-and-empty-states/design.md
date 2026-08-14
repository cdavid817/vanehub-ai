## Context

The audit surfaced a split that is easy to explain and easy to regress: Settings pages were built after `ApplicationDialog` existed, workspace modals were built before it. Nothing in the specs said modals must share behavior, so the split has no mechanical guard and would reappear with the next hand-rolled modal.

## Goals / Non-Goals

Goals:

- One modal behavior contract, expressed once, that new modals inherit by construction.
- Remove browser-native prompts from the desktop surface.
- Bring the Loop Center first-run state to the bar the rest of the product already meets.

Non-Goals:

- No global search, keyboard shortcut system, or configuration export. Those are capability gaps, not defects in shipped behavior.
- No re-ordering of Settings navigation. That order is a deliberate product decision asserted by `tests/e2e/settings-navigation-order.spec.ts`.
- No routing change. Making workspace destinations addressable is a separate architectural question.

## Decisions

### Route workspace modals through the existing primitive rather than restating behavior per modal

`ApplicationDialog` already implements the behavior correctly, is under test, and is used by 25 files. Re-implementing dismissal and focus handling in four more places would multiply the surface that can drift. The alternative — leaving the workspace modals alone and only documenting the requirement — was rejected because the audit evidence is a user-visible defect, not a style inconsistency.

The create-session dialog is the awkward case. It is a three-row grid (`auto / minmax(0,1fr) / auto`) with a scrolling middle region and a pinned footer, while `ApplicationDialog` renders a single scrolling body under a fixed header. Two options:

1. Extend `ApplicationDialog` with an optional footer slot and let it own the grid.
2. Keep the create-session layout and wrap it, accepting a nested scroll region.

Option 1 is chosen. A pinned footer is a general dialog need — the scheduled-tasks dialog and the batch-delete confirmation both want one — and a nested scroll region inside a dialog that already scrolls produces two competing scrollbars. The extension is additive: existing callers that pass no footer render exactly as they do today.

### Keep the CLI conflict dialog in scope even though it lives under Settings

It is reached from a Settings page but is a hand-rolled modal with the same defect, and it appears during CLI setup, which is a first-run path. Leaving it would preserve the exact inconsistency this change exists to remove.

### Category creation becomes a dialog, not an inline editor

An inline editor in the session context menu would avoid a modal entirely, but the creation flow has to both create the category and assign the session to it, and it can fail on a duplicate name. A dialog can show that validation error next to the field; a context-menu inline editor that disappears on blur cannot.

## Risks / Trade-offs

The create-session dialog is the single highest-traffic modal in the product and is asserted by `tests/docs/documentation-screenshots.spec.ts` through a `.fixed.inset-0 .ucd-panel` selector and a minimum bounding box. Moving it onto `ApplicationDialog` changes both the class structure and the rendered size, so the screenshot inventory and any selector-based e2e assertions must be updated in the same change rather than after it. This is the main reason this work is a spec-driven change and not a direct edit.

Adding a footer slot to `ApplicationDialog` widens a primitive that 25 files depend on. The mitigation is that the parameter is optional and defaults to today's rendering, so the blast radius is a type-level addition rather than a behavioral one.

## Open Questions

- Should the Loop Center empty state's primary action open the existing Loop definition dialog directly, or first present a short explanation of what a Loop is? The audit did not establish whether users failing to find the `+` button understand the concept or merely the affordance.
