## Context

The archived group-chat change introduced ordered `SessionSeat` values and message `seatIndex` attribution. Routing, speaker bubbles, turn status, and seat-scoped workspace tabs are in production, but the roster editor and participant mention component are not wired into the running-session UI. Session navigation, the chat header, and the information panel still read the compatibility `agentId` mirror.

An array index cannot identify historical speakers once the roster is mutable. Expert roles are also reusable settings assets, so resolving historical messages through the current role registry can change or erase their presentation. The change crosses the React service boundary, both frontend adapters, Sessions persistence, Agent Runtime routing, and SQLite migration.

## Goals / Non-Goals

**Goals:**
- Make participant and historical message identity stable across membership and role changes.
- Finish the existing shared-thread presentation without introducing a second conversation surface.
- Let users temporarily prioritize the shared thread without losing their surrounding workspace state.
- Keep desktop and Web adapters contract-identical.
- Preserve single-Agent presentation and the `agentId` compatibility mirror.

**Non-Goals:**
- Parallel Agent fan-out or multiple simultaneous turn holders.
- Replacing serial line-leading mention routing.
- Role-scoped memory, permissions, or billing.
- Reworking seat-scoped terminal, Shell, log, or trace behavior beyond stable identity mapping.

## Decisions

### Participants are append-only session snapshots

`SessionSeat` gains a stable `seatId`, captured role presentation, `joinedAt`, and nullable `leftAt`. Leaving marks a participant inactive; it does not remove or renumber the record. Active-roster readers filter `leftAt == null`, while historical renderers can resolve every participant ever present.

Using `agentId` as identity is rejected because the same Agent may hold two roles in one session. Continuing to use array position is rejected because deletion changes meaning. Copying the complete role instruction into messages is rejected because messages need presentation identity, while runtime prompt snapshots remain session-participant state.

### Messages reference `speakerSeatId`

New messages store `speakerSeatId`. The existing numeric `seatIndex` remains a temporary read-compatibility field during migration but is no longer written by new code. A schema migration adds `speaker_seat_id`, backfills it from the persisted participant order, and leaves invalid indexes null.

Web fixtures use the same stable-id derivation for legacy records. New seat ids come from the existing Sessions id-generation boundary so UI code never invents native identities.

### Membership changes are optimistic-concurrency mutations

The service receives the session id, expected `updatedAt`, and desired active seat assignments. The application layer compares the revision, reuses stable ids for unchanged active seats, marks removed participants as departed, appends new participants with snapshots, and updates the `agentId` mirror atomically.

This replaces a blind `save seats` operation, which could overwrite changes from another window. React calls only `AgentService`; the Tauri adapter invokes a dedicated session command and the Web adapter applies the same rules in memory.

### Presence is compact, role-semantic, and non-dispatching

The session card keeps the primary CLI's native brand icon and places a localized multi-Agent label in the bounded metadata row instead of replacing navigation identity with role avatars or competing with the title. The navigation list clips horizontal overflow as a final guard against browser scrollbars. The conversation header omits participant chips and CLI labels so it remains focused on session identity, runtime state, and actions. Built-in role ids supply their known names in the member-information card when an older seat has no captured role snapshot. Built-in roles use distinct Lucide symbols for architecture, implementation, and review; captured custom avatars remain visible, while participants without a role fall back to their Agent brand. The conditional Member Information tab hosts the full roster and membership editor in a visually independent card after the session has held multiple participants. None of these controls selects the next speaker.

For multi-Agent sessions, Member Information becomes its own information-panel tab alongside Basic Info, Token Usage, and Skill instead of remaining nested inside Basic Info. Single-Agent sessions omit both the tab and its pane, so the tab row does not reserve an empty slot. The tab grid derives a bounded three-, four-, or five-column layout from the visible tab set, including the existing conditional Code Index tab.

Separate per-Agent chat columns are rejected because they obscure causal ordering and conflict with serial handoff. Multiplying workspace tabs by participant count remains rejected.

### Conversation focus mode overlays panel visibility

Focus mode derives the effective sidebar and information-panel visibility from one temporary local state. It does not overwrite either panel's independent collapsed state, so leaving focus mode restores the layout the user had before entering it. The global top bar contracts to a compact surface containing the brand anchor and localized restore control, while the session workspace tab bar is temporarily removed from layout without changing the selected tab. This releases vertical space without removing the escape route.

Persisting focus mode across application launches is rejected because reopening into a context-free layout can make sessions and membership controls appear missing. Hiding the activity bar or removing the top-bar escape control is also rejected because the user needs a stable recovery route and access to primary navigation.

### The workspace omits decorative bottom status chrome

The static bottom strip duplicates status already available in contextual surfaces and permanently consumes conversation height. The workspace shell removes it rather than replacing it with another fixed footer. Runtime or turn state remains visible in the session card, chat header, and turn-status surface.

### The desktop workspace uses contiguous chat geometry

The primary desktop session surface uses adjacent activity, session-list, conversation, and information regions separated by quiet one-pixel dividers. Outer card gutters, repeated rounded panel shells, decorative grid texture, heavy shadows, and persistent glass blur are removed from this path. The session list uses dense rows with a stronger selected surface, while the conversation owns a stable header, an adaptive message canvas, and a composer attached to the bottom edge.

The message canvas expands with the conversation region and uses small breakpoint-aware edge gutters instead of a fixed centered maximum width. Individual message bubbles retain their own readable maximum width, so focus mode exposes useful alignment space without turning long prose into edge-to-edge lines. Assistant content remains anchored toward the leading edge and user content toward the trailing edge, matching desktop messaging geometry.

The reference is the spatial grammar of mature desktop messaging clients, not their brand treatment. VaneHub retains semantic theme tokens, CLI identity, multi-Agent role presence, workspace tabs, dark mode, and visible keyboard focus. Copying WeChat green, consumer avatars, or phone-specific interaction patterns is rejected because those would obscure the developer-tool identity and break theme parity.

The attached composer uses one quiet bordered surface: references and completion remain associated with a spacious editor, while runtime selectors and send actions sit in a bottom toolbar inside the same boundary. The control set remains VaneHub-specific; only the reference client's clear input hierarchy and restrained geometry are adopted.

### Conversation chrome is shared across CLI families

Every supported CLI uses one conversation-header structure without Agent- or CLI-specific identity rows. A stable right-aligned overflow menu owns secondary visibility actions for the session list, information panel, and workspace tab row, and it is the sole information-panel collapse/expand entry point. Focus mode reuses the same title, runtime state, and actions DOM order while panels collapse.

Participant chips use fixed grid columns for the semantic icon, the role-first text stack, and a permanently reserved speaking-state slot. The roster stays on one horizontally bounded row, and focus mode changes workspace grid tracks without transitioning their dimensions. Panel opacity and translation may still communicate visibility, but layout-affecting animation is avoided so intermediate widths cannot wrap or swap participant content.

The message scroller records its preceding scroll height and preserves the user's bottom offset when workspace width changes reflow message content. A reader already near the latest message remains pinned to the bottom. This keeps message order and the visible reading position stable when focus mode or an overflow-menu visibility action changes adjacent panels.

The production member-information roster receives the current turn holder's stable seat id from the same turn-status state used by the chat status bar. It compares stable ids rather than array positions and communicates speaking state through text, status shape, and accessible state; colour remains supplemental. Message speaker metadata renders the captured role colour as a small validated SVG swatch beside the role label, avoiding inline styles while preserving arbitrary captured role colours.

The workspace draws explicit one-pixel boundaries between the session list, conversation, information panel, and bottom window edge. Resize hit targets may overlap a divider but must not erase it. Zero-valued tab badges are omitted because an empty count adds noise without new state.

The bottom window boundary is owned by one absolute, pointer-transparent overlay at the workspace-shell level. It spans the full inline extent above every child surface and uses the semantic border token, so activity, navigation, conversation, information, and alternate-destination backgrounds cannot cover separate line fragments.

### Message submission uses reversible optimistic rendering

The frontend inserts a temporary completed user message into the active message query before the native send command returns. The draft and references clear immediately. Success replaces the temporary state through the canonical message refresh; failure removes the temporary message and restores the submitted draft and file references. This improves perceived latency without weakening the service boundary or pretending that Agent generation itself completed early.

### Message insertion tolerates additive recovery schema

Development worktrees can share one application database. A newer worktree may therefore add `messages.session_sequence` with a per-session unique index before this change runs. The repository inspects the additive column and, when present, inserts an explicit `MAX(session_sequence) + 1` value in the same SQLite insert statement; older schemas retain the existing insert path. This avoids default-zero collisions without making the current domain depend on unreleased recovery fields or undoing another migration.

### Windows structured generation bypasses npm batch shims

Windows npm installations expose Codex through `codex.cmd`. Rust's safe batch launcher rejects multiline and control-bearing arguments before the CLI starts, which conflicts with the role briefing carried by `developer_instructions`. Structured generation resolves a recognized Codex shim to the packaged architecture-specific native `codex.exe` and continues to pass every argument separately. Unknown shims and missing native packages retain the existing fallback so discovery remains honest; no shell concatenation or relaxed injection validation is introduced.

### Composer completion has explicit result kinds

The composer completion model becomes a discriminated union of `participant` and `file` results. A line-leading `@` prioritizes active participant handles. File results retain their visual file identity and attachment behavior; exact routing handles win over file paths during native mention parsing. Departed participants are excluded.

### Asynchronous creation publishes the canonical session

The desktop creation operation result remains a bounded progress payload, not the authoritative session transport. After the operation succeeds, the frontend resolves its session id through `AgentService.getSession` before closing the dialog or updating active-session state. This guarantees that stable seat ids, role snapshots, and membership lifecycle fields come from the same canonical mapper used by list, get, and switch operations.

Teaching every UI consumer to tolerate partial operation payloads is rejected because it would spread two incompatible session shapes through React state. Expanding the operation payload to duplicate the full session DTO is also rejected because the operation context should not become a second session contract.

## Risks / Trade-offs

- **Legacy sessions contain malformed seat JSON** → Fall back to a deterministic one-participant projection and leave unresolvable legacy messages unattributed.
- **Participant snapshots duplicate role display data** → The small storage cost buys immutable history and decouples sessions from mutable settings assets.
- **Membership updates race with turn routing** → Apply the roster change transactionally and make the coordinator validate that a target remains active immediately before launch.
- **`@` conflicts with file references** → Use typed completion results and exact participant-handle matching; keep file attachment explicit in the selected result.
- **More identity fields increase compatibility code** → Keep `agentId` and `seatIndex` read support during this change, with one normalization layer in each adapter.

## Migration Plan

1. Add stable participant fields to stored seat JSON and add nullable `messages.speaker_seat_id`.
2. Normalize legacy session seats to deterministic ids and backfill valid indexed message attribution in one transaction.
3. Publish the extended contracts and membership operation through Rust commands and both frontend adapters.
4. Switch runtime routing, turn events, and message writes to stable seat ids.
5. Wire presence, membership, completion, and stable seat identity into the existing shared-thread and trace surfaces.
6. Reconcile successful asynchronous creation with the canonical session record before publishing it to the UI.
7. Convert the desktop session surface to contiguous chat geometry while preserving responsive collapse and workspace controls.
8. Verify legacy, single-Agent, multi-Agent, concurrent-window, and adapter-parity behavior.

Rollback keeps the additive columns and participant fields. Older readers continue using `agentId`, the active seat order, and legacy `seatIndex` values; no destructive down-migration is required.
