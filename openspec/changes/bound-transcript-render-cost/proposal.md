## Why

A long conversation keeps every message it has loaded in the DOM. The list starts at fifty and "load earlier" adds fifty more each time, and nothing bounds what the browser does with the ones that have scrolled away: style resolution, layout and paint all scale with everything ever loaded rather than with what is on screen.

That is the shape of "the session feels slower the longer it gets" that remains after Run observation and streaming persistence stopped scaling with history. A transcript showing the same three visible messages costs more at message six hundred than at message fifty, and the extra cost buys nothing a reader can see.

## What Changes

- Let the browser skip style, layout and paint for transcript rows that are off-screen, while leaving them in the document.
- Give each skipped row a remembered size so the scrollbar keeps its position and length instead of resizing as rows enter and leave view.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-experience`: Rendering work for the transcript is bounded by what is visible rather than by how much history has been loaded.

## Impact

- One transcript row class in the shared stylesheet and the message row that carries it.
- No component, service, or native change; no dependency; no database migration.

## Alternatives Considered

**Virtualizing the message list** — mounting only the visible rows through `@tanstack/react-virtual`, which is already a dependency and already wrapped by `MeasuredVirtualList`.

It would additionally bound DOM node count and memory, which this change does not. It was not taken here because the interaction it would have to get right is the one this work exists to improve. Transcript rows vary from a single line to a rendered document, so no `estimateSize` is close for most of them; prepending fifty unmeasured rows on "load earlier" therefore moves the total size by an estimate first and corrects it as each row measures, and the reader's position moves with it. The list also follows a streaming row whose height changes several times a second, on top of the existing scroll anchoring.

Containment reaches the same visible-cost result without touching scroll behaviour at all: heights stay real, offsets stay real, and the browser decides what to skip. Virtualization remains available if node count or memory is later measured to be the binding constraint — it is a different problem from this one, and worth doing against evidence rather than in the same step.
