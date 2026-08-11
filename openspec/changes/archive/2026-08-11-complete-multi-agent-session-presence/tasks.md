## 1. Stable Participant Contracts

- [x] 1.1 Extend frontend, contract, command DTO, and Rust session-seat models with stable seat identity, role presentation snapshots, and join/leave lifecycle fields.
- [x] 1.2 Replace new message writes and turn-status payloads with stable speaker seat ids while retaining legacy seat-index read compatibility.
- [x] 1.3 Add normalization helpers and tests for active roster filtering, deterministic legacy ids, immutable speaker resolution, and first-active-Agent mirroring.

## 2. Desktop Persistence and Migration

- [x] 2.1 Extend the Sessions SQLite schema with stable message speaker identity and normalize legacy stored seat JSON without making malformed sessions unreadable.
- [x] 2.2 Backfill valid legacy seat-index attribution to stable seat ids and cover valid, invalid, departed, and one-seat migrations with Rust tests.
- [x] 2.3 Update Agent Runtime routing, roster briefing, generation ownership, and coordinator validation to use active stable seat identities.
- [x] 2.4 Make message insertion allocate a unique per-session sequence when an additive shared-database recovery schema is already present.
- [x] 2.5 Resolve recognized Windows Codex npm shims to the packaged native executable for multiline role briefing launches.

## 3. Membership Service Boundary

- [x] 3.1 Add an optimistic-concurrency participant-membership mutation to the Sessions application/API and expose it through a dedicated Tauri command.
- [x] 3.2 Add the matching AgentService method and implement identical validation and output behavior in the Tauri and Web adapters.
- [x] 3.3 Cover no-active-seat rejection, stale revision conflict, participant reuse, departure history, appended participant ids, and first-Agent mirroring.
- [x] 3.4 Resolve successful asynchronous session creation to the canonical session service record before publishing active-session state.

## 4. Shared-Thread Presence UI

- [x] 4.1 Add reusable roster identity and presence components for bounded avatar stacks, header chips, active/idle states, and overflow counts.
- [x] 4.2 Render multi-Agent roster identity and an explicit session-card label in session cards and the chat header while preserving the single-Agent layout.
- [x] 4.3 Wire the roster editor into the Basic information panel with service-backed add/leave behavior, conflict recovery, departed history, and no dispatch control.
- [x] 4.4 Resolve message speakers and session-scoped execution traces by stable seat id.
- [x] 4.5 Add a localized conversation focus-mode control that temporarily collapses both secondary panels and restores their prior states on exit.
- [x] 4.6 Contract the global header and collapse the session workspace tab bar in focus mode while keeping the restore control visible.
- [x] 4.7 Render semantic built-in role icons, captured custom avatars, and Agent-brand fallbacks across multi-Agent roster presence.
- [x] 4.8 Remove the decorative bottom status strip, isolate multi-Agent membership in its own conditional information card, and move the navigation label into bounded metadata.
- [x] 4.9 Keep native CLI identity in session navigation and render chat-header participants with role-first, CLI-second hierarchy.
- [x] 4.10 Refine the desktop session shell, navigation rows, conversation header, message measure, bubbles, and attached composer into contiguous chat-first geometry.
- [x] 4.11 Standardize role-first header identity for every CLI, add accessible session overflow controls, preserve focus-mode alignment, omit zero terminal badges, and restore continuous workspace dividers.
- [x] 4.12 Add reversible optimistic user-message rendering so valid sends acknowledge immediately and rejected sends restore their draft and references.
- [x] 4.13 Stabilize participant role, CLI, and speaking-state geometry across focus-mode transitions and remove layout-affecting workspace grid animation.
- [x] 4.14 Replace child-dependent bottom borders with one window-wide top-layer workspace divider.
- [x] 4.15 Remove duplicated conversation-header member identity, make overflow the sole information-panel visibility control, and keep member information in a multi-Agent-only card.
- [x] 4.16 Promote Member Information to a conditional peer tab beside Basic Info, Token Usage, and Skill.
- [x] 4.17 Refine the attached composer into one spacious messaging-style input surface with an integrated bottom toolbar.
- [x] 4.18 Preserve the message viewport across focus-mode and overflow-menu workspace resizing.
- [x] 4.19 Expand the message canvas with adaptive edge gutters while preserving readable per-bubble widths.
- [x] 4.20 Wire the stable current turn-holder seat id into the production member-information roster.
- [x] 4.21 Render captured role colour as a secondary message-speaker cue and align departed-history specification with the Member Information tab.

## 5. Composer Completion

- [x] 5.1 Refactor composer suggestions into typed participant and file results with distinct visual treatment and insertion behavior.
- [x] 5.2 Wire active-seat mention completion into the production composer, enforce unique routing handles, and exclude departed participants.
- [x] 5.3 Add synchronized locale strings and accessible labels for roster presence, membership actions, completion kinds, conflicts, and participant states.

## 6. Verification Coverage

- [x] 6.1 Add frontend unit and component tests for immutable attribution, roster surfaces, membership editing, typed completion, and single-Agent compatibility.
- [x] 6.2 Add Rust application, persistence, command-contract, routing, and migration tests for stable participant identity and membership updates.
- [x] 6.3 Add Playwright coverage for creating a multi-Agent session, viewing roster presence, adding/leaving a participant, mentioning a role, and preserving historical attribution.
- [x] 6.4 Add component and Playwright coverage for entering focus mode, expanding the conversation surface, and restoring the preceding panel states.
- [x] 6.5 Cover compact focus chrome and participant role-icon fallbacks with component and Playwright tests.
- [x] 6.6 Cover conditional membership-card rendering, status-strip removal, and navigation overflow behavior.
- [x] 6.7 Add a Rust regression test for repeated message inserts against the additive unique sequence schema.
- [x] 6.8 Cover native session-card identity and built-in role-name fallback without captured snapshots.
- [x] 6.9 Add a Windows Codex npm layout regression test for structured generation executable resolution.
- [x] 6.10 Cover desktop-style partial creation payload reconciliation so a newly created multi-Agent conversation immediately retains its complete roster.
- [x] 6.11 Cover contiguous desktop workspace geometry, dense session selection, readable message width, and attached composer behavior.
- [x] 6.12 Cover shared CLI header hierarchy, overflow-controlled panel visibility, focus-mode alignment, hidden zero badges, divider continuity, and optimistic-send rollback.
- [x] 6.13 Cover reserved speaking-state columns, single-row roster geometry, and non-animated focus grid tracks.
- [x] 6.14 Cover full-width bottom-divider geometry across expanded, focused, and narrow workspace layouts.
- [x] 6.15 Cover header identity removal, absence of panel-local visibility buttons, and conditional member-information rendering.
- [x] 6.16 Cover multi-Agent member-tab selection, single-Agent tab omission, and bounded dynamic tab columns.
- [x] 6.17 Cover integrated composer geometry and stable message scrolling across workspace visibility changes.
- [x] 6.18 Cover message-canvas width usage before and after focus-mode and overflow-menu panel changes.
- [x] 6.19 Cover stable speaking-state presentation and captured message role-colour rendering.

## 7. Required Validation

- [x] 7.1 Run `openspec validate complete-multi-agent-session-presence --strict` and `openspec validate --specs --strict`.
- [x] 7.2 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 7.3 Run `npx playwright test`.
- [x] 7.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
  - `cargo fmt`, `cargo clippy`, and `cargo check` pass. The new additive-sequence regression test passes directly. Every Rust library group, binary, integration target, and doctest passes when run separately; the aggregate `cargo test` remains blocked on this Windows host, first by the running Tauri preview's executable lock and then by the previously observed no-diagnostic aggregate hang in an isolated target directory.
