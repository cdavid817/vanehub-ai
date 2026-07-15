## Context

The archived main layout work established a three-panel shell with a flexible central content area and fixed composer. The current center panel still renders a decorative flowchart-style canvas with agent nodes and connecting paths.

## Goals / Non-Goals

**Goals:**

- Replace the center flowchart canvas with a chat transcript.
- Use existing `workspace.chatMessages` data from the workspace service snapshot.
- Keep the composer fixed, non-shrinking, and inside the main content panel.
- Keep the change frontend-only.

**Non-Goals:**

- No new chat persistence, sending behavior, agent routing, backend service method, or adapter change.
- No change to sidebar grouping, information panel behavior, or settings layout.

## Decisions

- Render chat messages directly in `src/main-layout/main-layout.tsx` to preserve the current single-file main layout implementation.
- Keep the chat transcript in an internal `overflow-y-auto` region so long conversations scroll inside the main content panel.
- Remove unused flowchart node rendering and imports after the center panel no longer needs them.

## Risks / Trade-offs

- [Risk] Existing mock agent node data becomes unused in this component. -> Mitigation: leave service data unchanged because it may still be useful for future views.
- [Risk] Chat rendering is visual-only. -> Mitigation: this request only changes the main content presentation; sending behavior remains out of scope.
