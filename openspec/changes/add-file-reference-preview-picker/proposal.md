## Why

A reference can now carry a line range, but the only way to express one is to type `:10-50` into the composer. That requires already knowing the line numbers, and finding them means leaving the composer for the Files or Documents tab, reading the file, remembering two numbers, and coming back. The capability exists and is effectively unreachable for anyone who does not already have the file open elsewhere.

This change gives the range its intended entry point: picking a file from `@` completion opens a preview of that file, the user clicks the lines they mean, and the selection becomes the reference. Choosing a file and choosing a region become one uninterrupted action.

## What Changes

- **Selecting an `@` candidate opens a preview instead of attaching immediately** — the composer requests the file through the existing service boundary and shows its content in a dialog. Typing a range by hand keeps working and bypasses the preview entirely.
- **Content is displayed with syntax highlighting and 1-based line numbers** — the same line numbering the prompt injection uses, so what the user selects and what the Agent receives are labelled identically.
- **Lines are selected by clicking** — clicking a line number sets one end of the range, clicking a second sets the other, and the order of the two clicks does not matter. Clicking a single line and confirming yields a one-line range. A visible highlight shows the pending selection.
- **Two confirmations** — "reference whole file" attaches a reference without a range, "reference selection" attaches one with the selected range. Dismissing the dialog attaches nothing.
- **Unreadable files degrade rather than block** — a file the runtime reports as oversized, binary, or missing shows that state in place of content. Oversized and binary still offer the whole-file action, since referencing them was already allowed; missing offers nothing and reports why.
- **No new eligibility list** — candidate search already bounds `@` results to source and configuration files, so every candidate is a preview candidate. Introducing a second extension list here would be a second source of truth for the same question.

Not in scope: drag-and-drop and clipboard paste, proposed separately; editing file content; previewing files that `@` completion cannot surface in the first place.

## Capabilities

### New Capabilities

None. The behavior belongs to the existing chat file reference capability.

### Modified Capabilities

- `chat-experience`: One added requirement covering the preview surface — what it shows, how a range is selected, what each confirmation attaches, and how unreadable files behave. Expressed as an addition rather than a modification of "Chat file reference line ranges": the preview does not change what a range *means*, it adds a way to produce one, and every downstream rule (1-based inclusive bounds, identity, injection, persistence) applies unchanged. Keeping the existing requirement untouched also keeps this change independent of the drag-and-paste change, which touches the same capability.

## Impact

**Runtimes:** Both. The preview reads through `readSessionFile`, which both adapters already implement, so browser mode gets the same dialog over its mock workspace. No native change at all.

**Adapter boundary:** Unchanged, and notably no new command and no new service method — this change is reachable entirely through capabilities that already exist. React components gain no direct `invoke()` usage.

**Frontend:**
- A new preview dialog component built on the existing `ApplicationDialog` primitive, which already provides focus trapping, Escape handling, and focus return.
- Syntax highlighting needs `highlight.js` promoted from a transitive dependency of `rehype-highlight` to a declared one; the package is already in the lockfile at the version in use, so nothing new is installed.
- `ChatInputBox.tsx` is at 183 of the 300-line limit and mention state already lives in a hook; the dialog is a sibling component, not inline markup, so the composer grows by roughly the wiring for one piece of pending state.
- New localized strings for the dialog title, the two actions, the selection hint, and the unreadable-file states, across all locale files.

**Performance:** A referenced file can be up to 1 MB. Rendering one clickable element per line for a file that large is the main risk this design has to answer for; see design.

**No breaking changes.** Typing `@path:10-50` still attaches directly, existing references are unaffected, and nothing about persistence or prompt assembly moves.
