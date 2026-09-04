# Accessibility notes

What is actually true today, verified against the current interface and its test coverage — not an aspirational statement or a compliance claim.

## Keyboard operability

Every interactive pattern in the app — tab strips, menus, search-style pickers, dialogs, and sheets — is fully keyboard-operable with a consistent model per pattern. See [Keyboard shortcuts](keyboard-shortcuts.md) for the full reference. Two ARIA patterns the interface simply does not use anywhere are not force-built to look complete: there is no `role="tree"`/`role="grid"` content, and no `role="toolbar"` roving-tabindex group (the various `*-toolbar` panels are ordinary independent Tab stops, which is the correct shape for a bar mixing a text search box with buttons).

## Focus management

- **Closing a dialog or sheet returns focus** to whatever control opened it, not to the top of the page — the same shared focus trap underlies every dialog and sheet in the app.
- **Sticky chrome does not hide focused content.** Sticky headers, the composer, the Runtime Panel, and Loop Center's acceptance panel all reserve enough scroll margin that scrolling a focused control into view does not leave it hidden underneath them.

## Status is never color-only

Status, priority, pass/fail, trust, attention, and regression indicators always carry a text label alongside their color — this is a structural guarantee (the shared `Badge`/`StatusBadge` components require a label, not just a convention that individual call sites happen to follow) rather than something to double check per screen.

## Reduced motion

The interface honors the operating system's **prefers-reduced-motion** setting and skips non-essential animation for readers who have it turned on.

## Automated scan coverage — a real but partial picture

An automated axe-core accessibility scan runs against **Evaluation Center, Mission Control, Work Board, and Goal Center**, in both the Futuristic and Minimal themes. It does **not** currently cover the Session Workspace (the app's own default, most-visited destination), Projects, Loop Center, Scheduled Tasks, or any of Settings' pages — and even within the four covered destinations, only one populated state per destination is scanned, not every reachable sub-state. Treat the four covered destinations as spot-checked, not the whole application as certified.

## Language

The interface defaults to following the host system's locale rather than always starting in one fixed language; change it under **Settings → Basic Configuration** at any time. See [User interface](user-interface.md#basic-configuration).
