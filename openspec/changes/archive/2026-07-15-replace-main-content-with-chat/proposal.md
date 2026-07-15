## Why

The workspace main content area currently presents a flowchart canvas, but the requested primary workflow is chat-centered. Replacing the canvas with a chat view makes the central workspace align with conversation-first agent coordination.

## What Changes

- Remove the flowchart-style main content canvas from the workspace center panel.
- Render the main content area as a chat transcript using existing workspace chat message data.
- Keep the bottom composer fixed and usable inside the central panel.
- Preserve the existing sidebar, information panel, layout proportions, runtime boundaries, and service interfaces.

## Capabilities

### New Capabilities

### Modified Capabilities

- `main-layout-ui`: Changes the central workspace content from a flow canvas to a chat-first main content area.

## Impact

- Affects the shared React workspace UI in both Tauri desktop and browser Web runtimes.
- Primary implementation target is `src/main-layout/main-layout.tsx`.
- No backend, Tauri command, Rust, SQLite, service adapter, or dependency changes are expected.
