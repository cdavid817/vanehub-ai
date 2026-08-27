## Why

The composer shows useful participant and file results after `@`, but keyboard users cannot move through or select those results without leaving the textarea. This makes a common multi-Agent routing action slower and prevents the completion surface from behaving like an accessible combobox.

## What Changes

- Add keyboard navigation across the visible unified `@` completion results.
- Keep the completion initially unselected; the first `ArrowDown` or `ArrowUp` selects the first visible result.
- Let subsequent arrow keys move through the results, let `Enter` activate the selected result, and let `Escape` dismiss keyboard selection without changing the draft.
- Expose the active result through accessible option state and keep mouse selection unchanged.
- Preserve existing IME composition, message submission, file preview, and participant insertion behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-experience`: Define keyboard behavior and accessibility state for unified participant and file `@` completion.

## Impact

- Affects the shared React composer used by both desktop and Web/mock runtimes.
- Changes frontend presentation and interaction only; no Tauri command, service interface, runtime adapter, database, or dependency changes are required.
- Adds focused component tests and browser E2E coverage for keyboard-only completion.
