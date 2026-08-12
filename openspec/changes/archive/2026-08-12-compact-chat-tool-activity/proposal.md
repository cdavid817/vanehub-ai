## Why

OnePiece and other streaming Agents can emit multiple status snapshots for one tool call, but the frontend currently appends every snapshot as a separate row. Long reasoning turns therefore become dominated by repetitive, equally weighted tool cards whose raw English statuses and generic tool names make active work, failures, and approval requests hard to find.

## What Changes

- Reconcile `tool_use` events by stable tool-use id in both shared stream reduction and the Web/mock runtime so status transitions update one logical call.
- Replace the flat tool-card list with a compact localized activity summary that prioritizes approval requests and active calls while keeping failure evidence discoverable.
- Collapse successful and recoverable failure history by default, visually aggregate consecutive identical failures, and automatically disclose failures only when the assistant turn itself fails.
- Make the complete tool-activity region collapsible while keeping localized counts visible, forcing approval controls open, and collapsing terminal success history unless the user already chose a state.
- Retain keyboard-accessible details for every call and never hide approval controls.
- Show a concise command, path, or action preview when safe structured input is available, with bounded JSON input/output details on demand.
- Add unit, localization, and browser coverage for reconciliation, ordering, disclosure, and approval behavior in desktop and Web-rendered chat surfaces.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-experience`: Tool status snapshots become one logical activity and tool-heavy turns gain localized, prioritized, progressively disclosed presentation.

## Impact

- Frontend stream reconciliation in `src/services/chat-events.ts` and the Web/mock adapter.
- Chat tool rendering components, translations, component tests, and Playwright chat coverage.
- Both desktop and Web runtimes benefit through the shared React message surface; no native command, database schema, backend event contract, dependency, or service-boundary change is required.
