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

## 4. A dead thread does not stay dead

Implemented as recovery on the next turn rather than a retry of the failing one, and the spec
delta was amended to match rather than left describing behaviour that does not exist.

Two findings drove that. There is no retry machinery to hook: `GenerationProcessFailureKind::Retryable`
is produced by the providers and nothing acts on it, and the failure path is an event sink that
finalizes the message rather than owning the process lifecycle. And classifying the rejection by
provider wording (`no rollout found for thread id …` is codex's) is per-CLI and would be a list to
maintain forever.

So the signal is behavioural: a turn that passed a resume id and failed having said nothing. The
cost of being wrong is bounded and already a supported state -- `seat_turn.rs:32-34` gives prior
conversation to a seat that cannot resume -- so what a false positive loses is the CLI's own cached
context, not the thread the user sees. That is cheap next to a seat that fails identically forever
and explains nothing.

A same-turn retry remains worth building, but as its own change: the capability would serve every
`Retryable` failure, not just this one.

- [x] 4.1 Add failing tests for the discard, for a turn that speaks before failing, and for a turn
  that resumed nothing -- the last two are what stop the rule being over-eager.
- [x] 4.2 Discard the stored id for the seat that failed, clearing the session's copy too when the
  speaking seat is the first one, since that is where a pre-seat session keeps looking.
- [x] 4.3 Record the discard through the unified log at `warn`. No retry to bound, because the turn
  is not retried.

## 5. Gates and live verification

- [x] 5.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run lint:ci`, `npm run test`, `npm run build`, `openspec validate --specs --strict`.
- [x] 5.2 Run `openspec validate scope-provider-resume-metadata-to-a-seat --strict`.
- [x] 5.3 Raise the budgets the change exceeds, with reasons: `agent_runtime/infrastructure` +12, `agent_runtime/application/tests.rs` +21, `sessions/infrastructure/tests.rs` +1.
- [x] 5.4 Ran `tests/desktop/specs/domain-multi-agent.e2e.mjs` against claude-code and codex-cli seated together: 2 passing, no BLOCKED. The run's unified log shows one `claude` and one `codex` invocation in the same session, `executing codex with 13 arguments` against 15 before, and no `thread/resume` at all. File header updated.
- [x] 5.5 Confirmed on a two-turn single-Agent session: turn one invokes with 7 arguments, turn two with 9 -- the two extra being the resume pair -- and the Agent recalls a word given in turn one. The compatibility path is exercised, not reasoned about.
