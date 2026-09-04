## Context

See `proposal.md` for motivation. `ChatInputBox` currently renders participant and file completions in one visual panel, but each child button owns only pointer activation. The textarea continues to own keyboard input, so navigation must be coordinated at the composer level without moving focus away from the draft or bypassing the existing mention insertion and file-preview paths.

## Goals / Non-Goals

**Goals:**

- Treat participant and file results as one ordered keyboard list.
- Preserve the textarea as the focused editor while exposing an accessible active option.
- Reuse the existing participant insertion and file selection functions for activation.
- Reset stale selection whenever the visible result identity changes or completion closes.

**Non-Goals:**

- Changing candidate ranking, search, limits, or mention parsing.
- Adding a new service or native runtime contract.
- Changing slash-command completion behavior in this change.

## Decisions

### Keep keyboard state in the composer

`ChatInputBox` will own a nullable active index over a flattened participant-then-file result order. This is the only layer that can preserve one sequence across both result kinds and invoke the existing type-specific selection behavior. Moving focus into buttons was rejected because it interrupts typing and makes continued query editing awkward.

### Start with no active result

Opening or refreshing completion leaves the active index unset. Either first arrow direction selects index zero, matching the requested first-key behavior. Later arrows clamp at the first and last result rather than wrapping, so repeated input never jumps unexpectedly across the participant/file boundary.

### Intercept only owned keys

The textarea prevents default behavior for arrow keys only while results are visible, and for Enter/Escape only while an active result exists. IME composition events are never intercepted. Enter with no active completion preserves message submission; Shift+Enter preserves newline behavior.

### Use option semantics without moving DOM focus

The completion panel will expose a listbox and stable option ids. The textarea will reference the selected option with `aria-activedescendant`; each result exposes `aria-selected` and a non-color-only active treatment. Mouse activation remains unchanged.

## Risks / Trade-offs

- [Candidate results refresh while navigating] → Reset or clamp selection against stable result identities so a stale index cannot activate a different result silently.
- [Arrow keys normally move the caret] → Intercept them only while `@` results are visible; Escape clears selection first and restores normal caret navigation.
- [File selection opens a preview dialog] → Route Enter through the existing file selection function so preview and typed-range behavior stay identical to pointer activation.

## Migration Plan

This is an additive frontend interaction. Deploy with the shared composer bundle; rollback is limited to the composer and completion presentation changes and requires no data migration.
