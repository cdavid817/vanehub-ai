## Context

See `proposal.md` - Why. Constraints that shape the approach:

- `listSessionDirectory` gives the Files tab `DirectoryEntry { name, path, kind, size }` where `path` is already session-relative — the exact shape a reference needs. No path conversion happens anywhere in this change.
- The repository already has this drag-source pattern twice: the session sidebar sets `text/plain`, the work board sets a custom `text/work-item` type. Both use `dataTransfer.setData` on `onDragStart`.
- `addFileReference(candidate, range)` takes `{ name, path }` and a range, derives identity, and deduplicates. Attaching from a drop is the same call with an empty range.
- `ChatInputBox.tsx` is at 183 lines against the 300-line cap; `files-tab.tsx` is at 131.
- There is no clipboard or paste handling anywhere in the frontend today, and no attachment capability of any kind.

## Goals / Non-Goals

**Goals:**

- Get a visible path into the composer without retyping it.
- Leave ordinary typing, pasting, and dragging behavior untouched.
- Stay inside existing capabilities — no new command, no new service method, no path resolution.

**Non-Goals:**

- External sources (OS, editor, file manager). See Decisions.
- Line ranges from a drag or a paste. A gesture that conveys a file does not convey a region.
- Image or binary attachment of any kind.

## Decisions

### Paste needs a copy action, and that is why one is in scope

"In-app sources only" sounds like a restriction on paste, but it is really a statement that paste has no source at all until the application can put a path on the clipboard. Nothing in the Files tab produces one today. So the copy action is not scope creep bolted onto paste — it is the half of paste that makes the other half reachable, and shipping the handler without it would ship a feature with no way to trigger it.

### A custom clipboard type, not text sniffing

Paste attaches a reference only when the clipboard carries the application's own file-path type. The alternative — inspect pasted text and attach a reference when it looks like a path that exists in the workspace — was rejected on two grounds. It breaks a legitimate action: pasting `src/utils.rs` as *text* into a sentence is something users do, and there is no way to tell the two intents apart from the string. And it needs an existence check per paste, which means a native round-trip on a keystroke path that has none today.

The copy action writes both the custom type and `text/plain`. That keeps the clipboard useful everywhere else — pasting into a terminal, an editor, or the composer of a build without this feature all yield the path — and it means the feature degrades to today's behavior rather than to nothing.

### External drag and paste is excluded on a capability boundary, not a preference

A file dragged from the OS arrives as an absolute path. Turning that into a reference needs a session-relative path plus proof the file is inside the session root — a native capability that does not exist, and one that is genuinely security-relevant, since it is the check standing between a reference and an arbitrary file on disk. Adding it inside a UI convenience change would be the wrong place to design a containment boundary. Excluded here, viable later as its own change with its own scrutiny.

### Both gestures attach whole-file references

A drop and a copied path both carry a file, not a region. Attaching a range would mean inventing one — defaulting to the whole file is not a fallback here, it is the accurate reading of what the gesture said. A user wanting a region types the range or uses the preview picker.

### Handlers live with composer input, not on the textarea

`onDrop` and `onPaste` go through the same layer the rest of composer input handling uses, rather than as inline attributes. Two reasons: the composer is 117 lines from its cap and the preview-picker change also wants space there, and both handlers need the disabled/streaming guard that already lives alongside submission. Inline handlers would duplicate that condition in a second place.

### Drop affordance is state, not CSS-only

The composer must show it accepts the drop, which needs `onDragEnter`/`onDragLeave`/`onDragOver` and a piece of state. The subtlety worth naming: `dragleave` fires when the pointer crosses into a *child* element, so a naive implementation flickers. Track drag depth with a counter rather than a boolean, or check `relatedTarget` containment.

## Risks / Trade-offs

**Drop affordance flicker across child elements** → Depth counter or containment check, covered by a test that enters a child and asserts the affordance survives.

**A drag that starts in the Files tab and ends somewhere unexpected** → `dataTransfer` carries only a relative path string, so a drop on any other target yields text, not an action. Nothing destructive is reachable by dropping in the wrong place.

**The reference ceiling becomes easy to hit** → Dragging four files takes seconds where typing four mentions took a while. The existing limit and its error apply unchanged; whether that error is localized is a pre-existing question this change surfaces, not one it introduces.

**Clipboard behavior differs across the Tauri webview and browsers** → The custom-type mechanism is standard `DataTransfer`, but the webview is the runtime that matters and it is not what the test suite exercises. The Web adapter path is what tests cover; the desktop path needs the manual check in tasks.

**Paste attaching instead of inserting is surprising if the type check is wrong** → A false positive silently swallows a paste. The type is application-specific and only ever set by the copy action, so a false positive requires the application to have set it; the guard is that the copy action is the only writer.

## Migration Plan

No data migration, no persisted-format change, no schema change. Every behavior is additive and gated on either a drag originating in the Files tab or a clipboard type only this application writes. Reverting restores today's behavior exactly, and a path copied before the revert still pastes as text.
