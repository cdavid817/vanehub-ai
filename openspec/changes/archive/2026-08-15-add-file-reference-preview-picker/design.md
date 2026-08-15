## Context

See `proposal.md` - Why. Constraints that shape the approach:

- `readSessionFile` already returns `{ path, name, status, size, content }` with `status` in `text | binary | oversized | missing`, and both adapters implement it. The preview needs no new capability.
- `FILE_BYTE_LIMIT` is 1 MB, so `status: "text"` guarantees at most 1 MB of content — roughly 20-30k lines for source code.
- `ApplicationDialog` already provides focus trapping, Escape handling, Tab cycling, and focus return.
- `highlight.js` 11.11.1 and `lowlight` 3.3.0 are already in the lockfile as transitive dependencies of `rehype-highlight`, which the chat renderer uses.
- Mention parsing, candidate querying, and selection live in `use-composer-mention`; `ChatInputBox.tsx` is at 183 lines against the 300-line cap.
- Candidate search already restricts `@` results to source and configuration extensions, so the set of candidates and the set of previewable files are the same set.

## Goals / Non-Goals

**Goals:**

- Make a line range reachable without leaving the composer or knowing line numbers in advance.
- Keep the typed `:10-50` path working and unpreviewed.
- Stay inside existing service capabilities — no new command, no new service method.
- Keep the composer under its line cap with room for the drag-and-paste change.

**Non-Goals:**

- Editing, saving, or diffing file content.
- Search within the preview, folding, or multi-region selection in one pass. A second region is a second `@` mention, which reference identity already supports.
- Previewing files `@` cannot surface. Widening what can be referenced is a separate question from showing what already can be.

## Decisions

### Highlight with `highlight.js` directly, not through the markdown pipeline

The chat renderer highlights code by running Markdown through `rehype-highlight`. Reusing that here would mean wrapping file content in a fenced block, rendering it as Markdown, and then trying to attach per-line click handlers to the output — hostile to both line numbering and hit-testing, and it would run content through a Markdown parser that has no business seeing it.

Instead, call `highlight.js` on the raw string, split the highlighted output per line, and render each line as its own row. This is what makes a line a first-class clickable unit. `highlight.js` moves from transitive to declared dependency; the lockfile version does not change, so nothing new is installed. Language is chosen from the file extension with a plain-text fallback, matching the `detect: false, ignoreMissing: true` posture the chat renderer already takes.

Alternative considered: a dedicated code-viewer component library. Rejected — it would be a new UI dependency for one dialog, against the stack constraints, when the highlighting engine is already present.

### Windowed rendering rather than rendering every line

A 1 MB file is tens of thousands of lines, and one DOM row per line stalls the dialog on open. The repository already has `measured-virtual-list` for exactly this shape of problem, and the Prompt Hook inventory and log viewer both use windowing.

The spec deliberately states this as "stays interactive and any line is reachable" rather than naming a technique, so the requirement survives a change of mechanism. Reusing the existing component is the design-level choice, not a spec-level promise.

Consequence worth stating: a selection anchored on a line that later scrolls out of the window must survive, so the pending selection is state in the dialog keyed by line number — not a property of a rendered row.

### Anchor-then-extent, order-independent

Clicking sets an anchor; clicking again sets the other end and the range is normalised so the smaller number is the start. This means a user who clicks bottom-up gets the same range as one who clicks top-down, which is why the spec fixes the behavior rather than leaving it to the implementation. A third click starts over rather than extending — extending would need a modifier and a rule about which end moves, and there is no evidence yet which users would expect.

Confirming with only an anchor set yields a one-line range, which is what makes `@path:42` reachable by clicking too.

### The preview owns no reference state

The dialog returns a decision — whole file, a range, or nothing — and the existing `addFileReference` path attaches it. Identity derivation, deduplication, the five-reference ceiling, and the validation that rejects a malformed range all stay where they are. The dialog cannot produce a malformed range by construction, but routing through the same path means it cannot drift from the typed path either.

### Where the pending file lives

Opening the preview is a state transition in composer mention handling: a candidate was selected but not yet resolved. That state belongs in `use-composer-mention` alongside the rest of the mention lifecycle, not in `ChatInputBox`, which then only renders the dialog when the hook reports one pending. This keeps the composer's growth to a conditional render and leaves its line budget for drag-and-paste.

## Risks / Trade-offs

**A previously one-click action becomes two** → Attaching a whole-file reference now costs a confirmation it did not before. Mitigated by keeping the typed path unpreviewed and making "reference whole file" the primary action in the dialog, so the common case stays one deliberate click rather than an unexpected extra one. Worth revisiting if it proves annoying — a modifier-click to skip the preview is a small follow-up.

**Windowing plus click selection is where the bugs will be** → Anchor survives scrolling, selection highlight repaints correctly for recycled rows, a click near a window boundary lands on the intended line. Covered by tests that select across a window boundary rather than only within the first screen.

**Highlighting a 1 MB file is itself expensive** → `highlight.js` runs over the whole string before any windowing helps. If measurement shows this stalls, highlight per visible line instead of once for the file; the per-line rendering shape already permits that without changing the requirement.

**Escape now has two meanings in the composer** → With the dialog open, Escape dismisses it; without, it does whatever the composer does today. `ApplicationDialog` handles this by owning the key while mounted, which is why the dialog is a real dialog rather than an inline panel.

**Locale drift** → Five locale files gain several strings each. The existing `docs:check` and lint gates do not verify locale key parity, so a missed key surfaces as a raw key in the UI. Adding all five in one pass and asserting one string per locale in tests is the guard.

## Migration Plan

No data migration, no persisted-format change, no schema change. The dialog is additive; disabling it restores the previous attach-immediately behavior with no residue. Rollback is reverting the code.
