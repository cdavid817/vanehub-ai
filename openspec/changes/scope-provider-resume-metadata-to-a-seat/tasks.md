## 1. Characterize the current behaviour

- [ ] 1.1 Add a failing native test that a seat whose Agent has never spoken is invoked without a resume id, against a session whose `runtime_session_id` was captured by a different seat.
- [ ] 1.2 Add a failing native test that a reported runtime session id is captured against the seat that owns the generation, not against the session alone.
- [ ] 1.3 Add a failing native test that a single-seat session still resumes the id stored for it, so the migration path is pinned before it is written.

## 2. Seat-scoped persistence

- [ ] 2.1 Add a provider thread id to the seat model and to the seat payload persisted on `sessions.seats`.
- [ ] 2.2 Add the SQLite migration that copies each session's existing `runtime_session_id` onto its first seat, leaving the session column in place as the single-seat compatibility value.
- [ ] 2.3 Add migration tests covering a single-seat session, a multi-seat session, a session with no captured id, and a session whose first seat has already departed.
- [ ] 2.4 Extend the sessions port with a seat-scoped update, keeping `update_runtime_session_id` for the single-seat path until every caller has moved.

## 3. Capture and replay per seat

- [ ] 3.1 Route `GenerationProcessEvent::RuntimeSessionId` through the generation's seat ownership so capture is keyed by seat (`service.rs:4262-4265`).
- [ ] 3.2 Choose the resume id from the speaking seat rather than from the session in the generation path (`process_adapter.rs:172-179`).
- [ ] 3.3 Do the same in the terminal path (`terminal_process.rs:324-334`, `495-508`), which reads the session-level id along the same lines.
- [ ] 3.4 Verify the `ProviderCapability::Resume` requirement is evaluated against the speaking seat's Agent, not the session's mirrored `agent_id`.

## 4. A rejected resume does not fail the turn

- [ ] 4.1 Add a failing test that a provider rejecting a resume for an unknown thread leads to a new thread and a completed turn.
- [ ] 4.2 Classify the provider's unknown-thread rejection, discard the stored id for that seat, and retry once on a new thread.
- [ ] 4.3 Record the rejection through the unified log rather than surfacing it as an Agent failure, and keep the retry bounded to one attempt so a genuinely broken provider still terminates.

## 5. Gates and live verification

- [ ] 5.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run lint:ci`, `npm run test`, `npm run build`, `openspec validate --specs --strict`.
- [ ] 5.2 Run `openspec validate scope-provider-resume-metadata-to-a-seat --strict`.
- [ ] 5.3 Raise the `agent_runtime/infrastructure` line budget in the same commit if the change exceeds it, with the reason recorded alongside the existing entries.
- [ ] 5.4 Run `tests/desktop/specs/domain-multi-agent.e2e.mjs` against a host with two installed CLI Agents; the handoff case must pass rather than report BLOCKED, and its file header must be updated to record the fix.
- [ ] 5.5 Confirm a single-Agent session still resumes across turns, so the compatibility path is exercised and not only reasoned about.
