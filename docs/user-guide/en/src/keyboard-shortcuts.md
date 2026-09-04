# Keyboard shortcuts

A reference for real, currently-shipped keyboard support. This page only lists shortcuts and interaction models that are verified against the current interface and covered by passing tests — not a wishlist.

## Global

| Shortcut | Action |
| --- | --- |
| **Ctrl/Cmd+K** | Open the Command Center — search commands and jump to a destination |
| **/** | Focus the current page's own search box, when you are not already typing in a field |
| **Enter** | Send the current message (in the composer) |
| **Shift+Enter** | Insert a newline (in the composer) |
| **Escape** | Close the open dialog, sheet, or menu |

## Tab strips

Every tab strip in the app — the session workspace's primary tabs, the Runtime Panel, Mission Control's section navigation, several Settings pages' internal tab groups, and more — shares one model:

| Key | Action |
| --- | --- |
| **Arrow Left / Right** | Move to the previous/next tab |
| **Home / End** | Jump to the first/last tab |
| **Arrow Up / Down** | Also supported on a small number of vertical tab strips (for example Personalization's view switch) |

## Menus

Row-action menus, the workspace overflow menu, the seat switcher, and similar popups:

| Key | Action |
| --- | --- |
| **Arrow Down / Enter / Space** (on the trigger) | Open the menu, focused on its first item |
| **Arrow Up / Down** | Move between items |
| **Home / End** | Jump to the first/last item |
| **Enter / Space** | Activate the focused item |
| **Escape** | Close the menu and return focus to its trigger |

## Search-style pickers

The Command Center, Settings search, the quick-open dialog, and the composer's `@`-mention completion keep real focus in the text field while you move a virtual selection below it:

| Key | Action |
| --- | --- |
| **Arrow Up / Down** | Move the highlighted result |
| **Enter** | Choose the highlighted result |
| **Escape** | Close without choosing |

## Dialogs and sheets

Every dialog and slide-over sheet traps **Tab** focus inside itself — Tab from the last focusable control wraps back to the first — closes on **Escape**, and returns focus to whatever control opened it once it closes.

## Non-keyboard gestures that still have a keyboard path

Drag-and-drop is never the only way to do something: Work Board card movement has an equivalent "Move to…" menu, and dragging a file reference into the composer has both a paste path and the `@`-mention completion above. Neither carries a dedicated hotkey of its own — reach them through the menu or completion list instead.

## Related

- Why these models were built this way, and what is intentionally out of scope → [Accessibility notes](accessibility.md)
- The conversation input box's own controls → [User interface](user-interface.md#send-a-message)
