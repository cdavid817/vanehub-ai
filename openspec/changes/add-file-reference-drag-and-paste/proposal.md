## Why

Attaching a file reference currently requires typing `@` and enough of a path for completion to find it. When the user is already looking at the file in the Files tab, that is retyping something the interface is displaying: the path is on screen, and the only way to get it into the composer is to spell it out again.

This change lets the Files tab hand a path directly to the composer — by dragging a file onto it, or by copying the path there and pasting it in.

## What Changes

- **Files tab rows become drag sources** — a file row can be dragged; a directory row cannot, since a directory is not referenceable.
- **The composer accepts a dropped file** — dropping a file row onto the composer attaches a whole-file reference. A visible drop affordance appears while a draggable file is over the composer, so the target is discoverable rather than guessed at.
- **Files tab rows offer "copy path"** — this is the necessary counterpart to paste, not a separate feature. Restricted to in-app sources, paste has no origin unless the application can put a path on the clipboard in the first place.
- **The composer accepts a pasted path** — pasting content that carries the application's own file-path clipboard type attaches a whole-file reference. The path is also written as plain text, so pasting into any other target, or into the composer after the feature is reverted, still yields the path as text.
- **Ordinary paste is untouched** — content without that clipboard type pastes as text exactly as it does today. The composer does not inspect pasted text for anything that looks like a path.

Both entry points attach a whole-file reference. Neither carries a line range: a drag and a copied path convey a file, not a region, and inventing a range they do not carry would be a guess. A range is expressed by typing one or, once available, by picking one in the preview.

Not in scope, and deliberately: dragging or pasting files from outside the application (the operating system, an editor, a file manager) — those arrive as absolute paths that may lie outside the session root, which needs a native path-resolution and containment capability that does not exist; pasting images, which needs an attachment capability that does not exist either; and guessing whether arbitrary pasted text is a file path, which would break ordinary pasting of text that happens to look like one.

## Capabilities

### New Capabilities

None. The behavior belongs to the existing chat file reference capability.

### Modified Capabilities

- `chat-experience`: One added requirement covering drag-and-drop and copy-paste as ways to attach a reference — what can be dragged, what each gesture attaches, and what is left to ordinary text handling. Expressed as an addition rather than a modification, so this change stays independent of the preview-picker change, which touches the same capability.

Note that the Files tab gains a drag source and a copy action, but the requirement describing what the Files tab *shows* does not change, so `session-workspace-tabs` needs no delta.

## Impact

**Runtimes:** Both. Drag, clipboard, and reference attachment are all frontend concerns over data both adapters already provide. No native change and no new command.

**Adapter boundary:** Unchanged. `listSessionDirectory` already supplies the Files tab with session-relative paths, which is exactly what a reference needs, so no path resolution happens anywhere in this change. React components gain no direct `invoke()` usage.

**Frontend:**
- `files-tab.tsx` (131 lines) gains `draggable` and a drag-start handler on file rows plus a copy-path action. The repository already has this drag-source shape in the session sidebar and the work board, both of which set a custom clipboard type.
- The composer gains drop and paste handlers. `ChatInputBox.tsx` is at 183 of the 300-line cap; the handlers belong with the rest of composer input handling rather than inline on the textarea, so the file grows by wiring rather than logic.
- Attachment routes through the existing `addFileReference`, so identity, deduplication, and the five-reference ceiling behave the same as for a typed mention.
- New localized strings for the copy action, its confirmation, and the drop affordance.

**Interaction with the reference ceiling:** dragging is fast enough that hitting `MAX_FILE_REFERENCES` becomes easy. The existing over-limit behavior applies unchanged; if it surfaces as a raw error rather than localized feedback, that is a pre-existing defect this change makes easier to reach, worth filing separately rather than fixing here.

**No breaking changes.** Ordinary paste, ordinary drag behavior elsewhere, and every existing way of attaching a reference are unaffected.
