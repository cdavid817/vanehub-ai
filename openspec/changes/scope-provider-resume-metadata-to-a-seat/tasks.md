## 1. Characterize the current behaviour

- [x] 1.1 Add a failing native test that a seat whose Agent has never spoken is invoked without a resume id, against a session whose `runtime_session_id` was captured by a different seat.
- [x] 1.2 Add a failing native test that a reported runtime session id is captured against the seat that owns the generation, not against the session alone.
- [x] 1.3 Add a failing native test that a single-seat session still resumes the id stored for it, so the migration path is pinned before it is written.

## 2. Seat-scoped persistence

- [x] 2.1 Add a provider thread id to the seat model and to the seat payload persisted on `sessions.seats`.
- [x] 2.2 ~~Add the SQLite migration that copies each session's existing `runtime_session_id` onto its first seat~~ — **not needed, and not written.** Seats are a JSON column with a tolerant decoder (`session_seat.rs`), so the new field needs no DDL, and a seat stored without it reads as `None`. The compatibility case is handled by reading instead of rewriting: `resume_thread_for` falls back to the session's id for the first seat only, which is the one seat that can own it since `agent_id` mirrors it. Rewriting rows that already answer correctly would be risk without benefit.
- [x] 2.3 Add tests covering a seat with its own thread, a seat without one, a blank stored value, and a session whose id predates the field. Persistence covered by `a_seats_provider_thread_survives_the_round_trip_independently_of_other_seats`; resolution by the six `seat_thread_tests`.
- [x] 2.4 Extend the sessions port with a seat-scoped update, keeping `update_runtime_session_id` for the single-seat path.

## 3. Capture and replay per seat

- [x] 3.1 Route `GenerationProcessEvent::RuntimeSessionId` through the generation's seat ownership so capture is keyed by seat. The session's own slot is still written when the speaking seat is the first one, because that is where a pre-seat session keeps looking and the terminal path and telemetry read it too.
- [x] 3.2 Resolve the resume id from the speaking seat rather than from the session. Resolved at the call site, where seat ownership is known, and passed on `GenerationProcessRequest::resume_thread_id`; the adapter no longer reads `session.runtime_session_id` at all.
- [x] 3.3 ~~Do the same in the terminal path~~ — **verified unnecessary, no change made.** A terminal is always opened for `session.agent_id` (`terminal_service.rs:91`), which mirrors the first seat, so the session's id is already that Agent's own thread. There is no cross-Agent leak to fix, and changing it for symmetry would add risk for nothing.
- [x] 3.4 Confirm `ProviderCapability::Resume` is evaluated against the speaking seat's Agent. It already is: `request.agent` is the seat's Agent, confirmed on the live run where the codex-cli seat's turn invoked `codex`.

## 4. A rejected resume does not fail the turn

- [ ] 4.1 Add a failing test that a provider rejecting a resume for an unknown thread leads to a new thread and a completed turn.
- [ ] 4.2 Classify the provider's unknown-thread rejection, discard the stored id for that seat, and retry once on a new thread.
- [ ] 4.3 Record the rejection through the unified log rather than surfacing it as an Agent failure, and keep the retry bounded to one attempt so a genuinely broken provider still terminates.

## 5. Gates and live verification

- [x] 5.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run lint:ci`, `npm run test`, `npm run build`, `openspec validate --specs --strict`.
- [x] 5.2 Run `openspec validate scope-provider-resume-metadata-to-a-seat --strict`.
- [x] 5.3 Raise the budgets the change exceeds, with reasons: `agent_runtime/infrastructure` +12, `agent_runtime/application/tests.rs` +21, `sessions/infrastructure/tests.rs` +1.
- [ ] 5.4 Run `tests/desktop/specs/domain-multi-agent.e2e.mjs` against a host with two installed CLI Agents; the handoff case must pass rather than report BLOCKED, and its file header must be updated to record the fix.
- [ ] 5.5 Confirm a single-Agent session still resumes across turns, so the compatibility path is exercised and not only reasoned about.
