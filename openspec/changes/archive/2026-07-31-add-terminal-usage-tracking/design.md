## Context

`PortablePtyAgentTerminalRuntime::open_or_attach()` (`infrastructure/terminal_process.rs`) spawns interactive CLI sessions behind a raw PTY and streams their output straight to the frontend terminal renderer. Unlike the managed/non-interactive pipeline (`process_adapter.rs`, used by the Floating Assistant), it never runs the child process's output through `ProviderOutputParser`, so `add-reported-usage-ingestion`'s parsing work does not reach interactive sessions at all — confirmed by tracing the actual code path, not assumed from the proposal. Each CLI does, however, persist its own usage independently, in its own log/database, regardless of how VaneHub invoked it:

- claude-code appends one line per turn to `~/.claude/projects/<cwd-hash>/<session-id>.jsonl`, with `usage` nested in the `assistant` event's `message` object (not a `result` event, which only exists in the managed `-p --output-format stream-json` mode).
- opencode keeps a `session` table in `~/.local/share/opencode/opencode.db` (SQLite) with running per-session token columns.
- codex-cli writes a rollout JSONL file per session with periodic `token_count` events carrying a cumulative `total_token_usage`.
- gemini-cli's equivalent (`~/.gemini/tmp/<project-id>/chats/session-*.jsonl`, project id resolved via `~/.gemini/projects.json`) was identified by reading `ChatRecordingService`'s bundled source, but never verified against a real sample — this sandbox has no gemini-cli auth, and no real session has produced one yet.

## Goals / Non-Goals

**Goals:**
- Persist reported token usage for interactive terminal sessions of claude-code, opencode, and codex-cli, reusing the existing `usage_records` schema and `reported`/`tokens` accounting kind unchanged.
- Refresh that usage while the terminal is still open, not only once after the process exits, so the Token Usage tab is useful during a long session rather than only in hindsight.
- Keep exactly one usage row per terminal session regardless of how many times it is refreshed.
- Ship with zero SQLite schema/migration changes and zero Tauri command/DTO signature changes.

**Non-Goals:**
- gemini-cli terminal-mode usage — no verified real sample to build and test against; deferred to a follow-up change.
- Any change to the managed/non-interactive pipeline's own usage parsing (`add-reported-usage-ingestion`'s territory) beyond the shared cwd-normalization bugfix described below.
- Sub-5-second refresh latency, or a push-based (event-driven) update mechanism — polling on both ends (backend re-read, frontend refetch) is deliberately simple and "good enough," not real-time.
- Correcting for a CLI's own usage data being wrong or delayed on its side; VaneHub only reads what the CLI has already written.

## Decisions

1. **Read each CLI's own persisted log/database directly, rather than trying to make interactive sessions flow through `ProviderOutputParser`.** The PTY carries a TUI's rendered output (cursor movements, ANSI escapes, redraws), not the line-delimited JSON `ProviderOutputParser` expects — there is no reliable "usage event" to intercept in that stream even in principle. Every CLI already durably persists its own usage independent of how it was invoked, which is a strictly simpler and more reliable source.
   - Alternative considered: have each CLI invoked with an additional flag that also emits structured JSON alongside its interactive TUI. Rejected — none of the three CLIs support dual TUI+structured-output modes simultaneously; this would require running two processes or forgoing the interactive TUI entirely, defeating the purpose.

2. **Post-hoc, after-the-fact lookup by working directory and start time, not the existing live `ProviderSessionCapture` poll.** `ProviderSessionCapture` (used for discovering codex-cli/opencode's own session id while output is flowing) only samples while fresh PTY output is arriving, gated at a 250ms interval tied to output events. In practice a session's own log/DB row can appear after the last visible output but before the terminal is closed, so relying on that same mechanism for usage would inherit its race. Instead, `find_opencode_session_since` / `find_codex_rollout_since` (new, in `infrastructure/providers/session_capture.rs`, alongside the existing discovery code they share helpers with) search by working-directory match plus a start-time floor, independent of whether any PTY output happened to be flowing at the right moment. claude-code does not need this lookup at all: VaneHub assigns its session id up front via `--session-id`, so the JSONL path is already known.
   - Alternative considered: extend `ProviderSessionCapture` itself to also carry usage. Rejected — conflates two different lifecycles (session-id discovery happens once; usage is read repeatedly) and would make the discovery poll's 250ms cadence do double duty as the usage-refresh cadence, which is far more I/O than usage data needs (see Decision 4).

3. **One stable message id per terminal session, created once and reused by every refresh, rather than a fresh message per read.** `upsert_usage`'s `ON CONFLICT(message_id) DO UPDATE` already makes repeated writes to the same id safe, and `MessageStatus::can_transition_to` explicitly permits `Completed -> Completed` (verified by reading both directly, not assumed) — so calling the existing `create_message` once at terminal start and `complete_message` on every subsequent refresh needs no new port method or schema change. The placeholder message is created for claude-code/opencode/codex-cli terminals unconditionally at start (empty content, `streaming` status) so the id is available regardless of whether the first usage read finds anything yet.
   - Alternative considered: a dedicated "upsert usage only, don't touch message lifecycle" gateway method. Rejected once the above was confirmed safe — it would duplicate what `complete_message` already does correctly, for no behavioral gain.

4. **A 5-second periodic re-read on an independent timer thread, plus one more read at process exit as a safety net.** `TERMINAL_USAGE_POLL_INTERVAL` gates a dedicated thread spawned alongside the PTY reader, decoupled from PTY activity entirely.
   - **Correction found during manual end-to-end testing**: the first version of this decision tied the poll to output arrival instead (reusing the session-discovery poll's own throttle-inside-`Ok(count) =>` pattern), reasoning that "a session producing no output also has nothing new to report." That reasoning was wrong, and testing against a real opencode session proved it directly: opencode's `session` row appeared in its SQLite database several seconds before its token columns were actually populated — a gap with no corresponding PTY output at all, since the CLI had already finished rendering its response and gone idle waiting for the next prompt. An output-gated poll stops polling exactly when a user is most likely to be watching the panel (right after a response finishes), and can miss the window between "row exists" and "totals populated" indefinitely if no further output ever arrives before the user acts. A truly independent timer has no such blind spot.
   - Alternative reconsidered and rejected again: reduce the risk by shortening the output-gated poll's interval instead of switching to an independent timer. Rejected — the failure mode isn't polling too infrequently, it's not polling *at all* during an idle gap of any length, which a shorter interval doesn't fix.

5. **Fix the Windows extended-length path (`\\?\`) cwd bug in the same change, applied to both the interactive-terminal and managed pipelines.** Verified directly (live PowerShell and a Python-subprocess-wrapped `cmd.exe` run) that `CreateProcess`'s `lpCurrentDirectory`, `cmd.exe`'s `cd /d`, and PowerShell's `Set-Location` all reject or silently mishandle a `\\?\`-prefixed path. Without this fix, an interactive terminal (or the managed/Floating-Assistant process) could start in the wrong directory, which would make the working-directory-matching lookups in Decision 2 fail outright — a launch correctness bug that happens to be a hard prerequisite for this change, not an unrelated drive-by fix. `normalize_windows_extended_length_path` (previously private to `workspaces::domain`) moved to `platform::filesystem` since it is now used by both `agent_runtime` and `workspaces` code, matching the project's rule that cross-context utilities belong in `platform`.

## Risks / Trade-offs

- [Risk] A CLI's own log/DB schema is unversioned and could change across releases, silently breaking a lookup or field mapping. → [Mitigation] Fixture-based unit tests pinned to the exact verified shape per CLI, mirroring `add-reported-usage-ingestion`'s approach; a missing/unreadable file or row degrades to "no usage found" (`Ok(false)`) rather than erroring the terminal session.
- [Risk] The 5-second poll re-scans a session's rollout/database file repeatedly for the life of a long session. → [Mitigation] Scoped to a single small file/row lookup per interval, gated on output actually arriving; verified negligible in manual testing, and only enabled for the three supported agent ids.
- [Risk] Creating a placeholder assistant message immediately at terminal start (before any real usage exists) adds an empty-content message row for the session's lifetime until the first non-zero read. → [Mitigation] Matches the pre-existing exit-time-only design's own placeholder shape (empty content, `assistant` role); the embedded terminal tab renders the raw PTY transcript, not the messages table, so this is not user-visible there, and no regression was reported for the original exit-time placeholder during earlier testing in this same effort.
- [Risk] gemini-cli remains without terminal-mode usage after this change. → [Mitigation] Explicitly scoped out (see proposal Non-Goals); the panel already handles "no reported usage yet" gracefully via the existing estimated-fallback path, so this is a gap, not a regression.

## Migration Plan

- No SQLite migration — reuses the `usage_records` schema and `reported`/`tokens` accounting kind `add-reported-usage-ingestion` already introduced.
- No feature flag: behavior is additive per agent id (`claude-code` | `opencode` | `codex-cli`), and any failure to create the placeholder message or locate a CLI's own session data degrades to "no usage recorded for this terminal session" — the same empty-panel state that exists today, not a new failure mode.
- Rollback: reverting the `terminal_usage_ingestion.rs` module and its call sites in `terminal_process.rs` restores today's "no terminal-mode usage" behavior; no data cleanup needed, since any already-persisted `usage_records` rows remain valid history.

## Open Questions

- Should the periodic-poll interval be user-configurable? Not requested; 5 seconds was chosen as a reasonable default balancing panel freshness against file/DB IO, and can be revisited if it proves too slow or too chatty in practice.
- Should gemini-cli's terminal-mode log location be verified and wired up as a fast-follow, or bundled with a broader gemini-cli investigation? Deferred — needs a real authenticated sample first.
