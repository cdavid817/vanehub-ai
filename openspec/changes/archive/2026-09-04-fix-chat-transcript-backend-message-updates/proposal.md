# Render backend-originated messages in the open chat transcript

## Why

With a multi-agent session open on screen, messages created through the backend — a programmatic `send_message` over IPC, and the seat replies it triggered — never appeared in the chat transcript: the message list stayed on the "开始新的对话" empty state for the entire round, while the turn status bar, seat presence states, and the session status badge all updated live beside it. The transcript only reflects flows the composer itself initiated, so any runtime-originated conversation (IM connector, scheduled task, desktop automation, another window) is invisible until a manual reload. Observed and screenshot-documented during desktop multi-agent verification on 2026-08-24.

## What Changes

- Make the open session's message list reflect messages created through the runtime regardless of origin: user messages injected via the service boundary and assistant turns dispatched by the seat coordinator appear, stream, and settle in the transcript without a reload.
- Keep the existing composer-originated behavior and message identity/speaker rendering unchanged; this closes the gap for the non-composer origins only.
- Add regression coverage at two levels: a component test for the transcript's reaction to a runtime-created message event, and a desktop e2e assertion that a programmatically sent message becomes visible in the open transcript.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `chat-experience`: Require the conversation view to display and stream messages that originate outside the composer while the session is open.

## Impact

- Frontend only: the chat message list's data flow (event subscription / query invalidation) in `src/` — no Tauri command or schema change expected; the backend already persists these messages and emits their events.
- Both service adapters must stay behaviorally aligned: the Web/mock adapter's transcript must follow the same origin-agnostic contract.
- Desktop e2e: extends existing multi-agent UI coverage; the `ui-multi-agent` layer already proves composer-originated rendering, so the new assertion targets the runtime-originated path.
