## 1. Expert Role Foundation

- [x] 1.1 Define the expert role entity (id, display name, avatar, colour, responsibility, role instruction, Skill references, peer-review eligibility, cross-family preference) in shared types.
- [x] 1.2 Add role persistence in the Rust SQLite layer with create, read, update, delete, and list.
- [x] 1.3 Add the matching role methods to `AgentService` and implement them identically in the Tauri and Web/mock adapters. Tauri side invokes `list_expert_roles` / `save_expert_role` / `delete_expert_role`; those Rust commands land with task 1.2.
- [x] 1.4 Seed built-in starter roles covering architecture, code review, and implementation, marked read-only but copyable.
- [x] 1.5 Add validation rejecting a role without display name, responsibility, or instruction.

## 2. Expert Role Settings Page

- [x] 2.1 Add the Expert Roles settings page with list, create, edit, delete, and copy-from-built-in.
- [x] 2.2 Add avatar and colour selection so roles are visually distinguishable in a thread.
- [x] 2.3 Add the peer-review eligibility and cross-family preference controls.
- [x] 2.4 Register the navigation entry between Agent configuration and skills per the settings-center-ui delta.
- [x] 2.5 Add `expertRoles.*` keys to all five locales and confirm i18n parity.

## 3. Model Family Normalization

- [ ] 3.1 Add a normalization mapping built-in Agents to model families and inferring families for custom API Agents from their endpoint type.
- [ ] 3.2 Expose the normalized family on the Agent registry projection consumed by seat assignment.
- [ ] 3.3 Unit-test that free-form `provider` values such as `"OpenAI"` normalize correctly and that unknown providers degrade to an explicit unknown family rather than a wrong one.

## 4. Seats and Session Entity

- [ ] 4.1 Replace the session's single agent id with an ordered seat list in shared types, Rust domain, and both adapters.
- [ ] 4.2 Add a migration presenting pre-seat sessions as one-seat sessions with no role, and test that no existing session becomes unreadable.
- [ ] 4.3 Enable the `multi` option in `SessionAgentModeSelector` and remove its coming-soon hint.
- [ ] 4.4 Add seat assignment to the create-session dialog: add and remove seats, pick role and Agent per seat, reject unavailable Agents.
- [ ] 4.5 Add cross-family reviewer recommendation with the open degradation notice when no cross-family Agent exists.

## 5. Speaker Identity in the Thread

- [ ] 5.1 Add a speaker field to the chat message model across shared types, persistence, and both adapters.
- [ ] 5.2 Extend `MessageItem` to render role avatar, role colour, and the `role · Agent` label, reusing its existing avatar slot and header row.
- [ ] 5.3 Mark a cross-family reviewer seat in its message header.
- [ ] 5.4 Verify single-seat sessions render exactly as they do today.

## 6. Role and Roster Injection

- [ ] 6.1 Inject a seat's role instruction through the Agent CLI's native system-prompt channel, reusing existing CLI parameter plumbing.
- [ ] 6.2 Inject the roster of other seats with their role names, mentions, and model families.
- [ ] 6.3 Fall back to per-turn injection when an Agent exposes no native channel, and surface on the seat that its role is not compaction-immune.
- [ ] 6.4 Feed prior turns by resuming the seat's provider session when one exists, and by injecting attributed prior replies within a per-seat context budget otherwise.

## 7. Handoff Routing

- [ ] 7.1 Add mention parsing that strips fenced code blocks, matches only line-leading mentions, and filters self-mentions.
- [ ] 7.2 Route the turn to mentioned seats after a reply completes, serially, each seat seeing preceding replies.
- [ ] 7.3 Enforce maximum chain depth and maximum mentions per message, and surface why a chain ended.
- [ ] 7.4 Route a user message with no line-leading mention to the seat that most recently held the turn, falling back to the first seat.
- [ ] 7.5 Add `@` seat completion to the composer, showing role, Agent, and family, and making the line-leading rule discoverable.

## 8. Human Turn Handling

- [ ] 8.1 Add the three handoff intents and their state effects: informational leaves the turn with the Agents, blocking transfers it to the human, completion ends the round.
- [ ] 8.2 Ensure an informational handoff raises no blocking prompt and does not disable the composer.
- [ ] 8.3 Stop invoking further seats once a blocking handoff or completion has occurred.
- [ ] 8.4 Accumulate and display the waiting duration for a paused round.

## 9. Turn Status Surface

- [ ] 9.1 Add the persistent turn-status bar showing the current holder, chain position against its limit, and waiting duration when paused.
- [ ] 9.2 Add the seats view to the session info panel showing each seat's role, Agent, family, and state.
- [ ] 9.3 Confirm no control anywhere selects which seat speaks next.

## 10. Workspace Tab Scoping

- [ ] 10.1 Add a scope declaration to each workspace tab.
- [ ] 10.2 Add the in-tab seat switcher to terminal transcript, Shell, and logs.
- [ ] 10.3 Colour execution-trace entries by seat while keeping the tab session-scoped.
- [ ] 10.4 Hide seat switchers in single-seat sessions and confirm the tab count does not change with seats.

## 11. Verification

- [ ] 11.1 Run `npm ci` first and confirm `node_modules/.pnpm` is absent, so build verification is trustworthy.
- [ ] 11.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [ ] 11.3 Run `cargo test`, `cargo check`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [ ] 11.4 Run `openspec validate add-multi-agent-group-chat-session --strict` and `openspec validate --specs --strict`.
- [ ] 11.5 Add E2E coverage for seat assignment, attributed messages, handoff routing and its limits, the three intents, and tab seat switching, running Playwright against a dev server started locally with `PLAYWRIGHT_PORT` pinned.
- [ ] 11.6 Validate against real CLI Agents that they reliably emit line-leading mentions when given the roster; if they do not, revisit the roster wording before widening scope.
