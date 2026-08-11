## Why

Multi-Agent sessions can be created and their replies can be attributed, but the running-session surface still presents the first Agent as if it were the whole session. More critically, messages identify speakers by a mutable seat array index, so removing or reordering seats can silently relabel historical messages.

## What Changes

- Give every session seat a stable identity and preserve joined/left participants as session history rather than renumbering them.
- Persist each Agent message against that stable seat identity and retain a renderable role/Agent snapshot so historical attribution survives roster and role changes.
- Keep the conversation header focused on session identity and runtime state, while session navigation retains native CLI identity and the information panel owns multi-Agent member details.
- Remove the decorative bottom status strip, isolate multi-Agent membership in its own information-panel card, and place the compact multi-Agent label with session metadata so narrow navigation never gains a horizontal scrollbar.
- Add a reversible conversation focus mode that temporarily collapses both secondary panels and nonessential upper workspace chrome so the shared thread receives the available workspace area.
- Add the roster editor to the session information panel so seats can join or leave through the frontend service boundary without selecting who speaks next.
- Integrate role `@` completion into the composer and distinguish participant mentions from file references.
- Keep all human and Agent messages in one chronological shared thread and mark seat-owned execution traces with stable participant identity.
- Keep single-Agent sessions visually and behaviorally compatible with the existing experience.
- Migrate legacy `seat_index` message attribution without making existing sessions unreadable.
- Keep message insertion compatible with a shared database that already enforces unique per-session message sequence values.
- Resolve Windows Codex npm shims to their packaged native executable before passing multiline role briefing arguments.
- Reconcile asynchronous desktop creation results with the canonical session record before publishing the active conversation, so every selected participant is visible immediately.
- Refine the desktop shell into a contiguous chat-first workspace inspired by familiar messaging clients: dense navigation, quiet dividers, a fixed conversation header, readable message measure, and an attached composer without copying consumer-chat branding.
- Standardize every supported CLI conversation header without duplicated member identity, make the session overflow menu the sole information-panel visibility control, and make message submission feel immediate through optimistic user-message rendering.
- Refine the attached composer into one spacious messaging-style surface with an integrated bottom toolbar, and preserve the visible conversation position while focus mode or overflow controls resize the workspace.
- Let the message canvas use the released conversation width with adaptive edge gutters while each bubble retains a readable maximum width, avoiding oversized empty margins after workspace panels collapse.
- Wire stable turn-holder identity into the production member-information roster and retain the captured role colour as a compact, accessible secondary speaker cue.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `multi-agent-group-chat`: Require stable participant identity, durable historical attribution, roster presence, and service-backed membership changes.
- `chat-experience`: Define the shared-thread session header, participant-aware composer completion, and immutable speaker rendering behavior.
- `session-management`: Persist stable session participants and expose a runtime-neutral membership update operation with legacy migration behavior.
- `main-layout-ui`: Represent multi-Agent sessions with roster identity in navigation and the information panel instead of only the mirrored first Agent, and provide a reversible conversation focus mode.

## Impact

- **Desktop runtime:** SQLite schema and migration, Sessions domain/application models, Tauri commands and DTO mapping, and Agent Runtime seat routing/status events.
- **Web runtime:** The Web/mock adapter implements the same participant and membership contracts without SQLite.
- **Frontend:** Session/chat contracts, service interfaces, composer completion, role-semantic roster icons, shared CLI chat header, optimistic message timeline, session sidebar, information panel, overflow menu, and workspace focus-mode controls.
- **Compatibility:** Existing single-Agent sessions and legacy messages remain readable. `agentId` continues to mirror the first active participant during the migration window.
- **Architecture:** React remains behind `AgentService`; no component calls Tauri directly. No new runtime or UI dependency is introduced.
