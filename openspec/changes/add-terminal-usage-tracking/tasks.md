## 1. Provider-native usage lookup (`agent_runtime` infrastructure)

- [x] 1.1 New `infrastructure/terminal_usage_ingestion.rs` module: `TerminalUsageTotals` plus per-CLI aggregation — `aggregate_claude_usage` (JSONL `assistant` events), `read_opencode_session_totals` (SQLite `session` table), `aggregate_codex_usage` (rollout `token_count` events, keeping only the last cumulative value).
- [x] 1.2 Post-hoc, after-the-fact lookup helpers in `infrastructure/providers/session_capture.rs`: `find_opencode_session_since` and `find_codex_rollout_since`, matching by working directory and a start-time floor rather than depending on the live `ProviderSessionCapture` poll.
- [x] 1.3 Fixture-based unit tests per CLI pinned to the exact verified JSON/SQLite shapes, including reasoning-token folding, all-zero totals, and missing-session/missing-file edge cases.

## 2. Stable message id and upsert-in-place

- [x] 2.1 Add `create_terminal_usage_placeholder`; refactor `persist_terminal_usage` and all three `ingest_*_terminal_usage` functions to accept an existing `message_id` instead of creating a fresh message on every call.
- [x] 2.2 Confirm repeated `complete_message` calls on the same id are safe by reading the actual implementation rather than assuming: `upsert_usage`'s `ON CONFLICT(message_id) DO UPDATE` (`sessions/infrastructure/usage.rs`) and `MessageStatus::can_transition_to`'s explicit `self == next` allowance (`sessions/domain/message.rs`), including `Completed -> Completed`.
- [x] 2.3 Unit tests with a fake `AgentSessionGateway` proving one `create_message` call followed by two `complete_message` calls on the same id updates in place (latest totals win) rather than accumulating duplicate rows.

## 3. Terminal lifecycle wiring (`infrastructure/terminal_process.rs`)

- [x] 3.1 Create the placeholder usage message once, synchronously, before the PTY reader thread starts, for `claude-code` / `opencode` / `codex-cli` sessions only.
- [x] 3.2 Add a periodic usage poll (`TERMINAL_USAGE_POLL_INTERVAL`, 5 seconds) on an independent timer thread, decoupled from PTY output activity.
  - [x] 3.2.1 **Found during manual end-to-end testing**: the first version gated the poll on PTY output arrival (reusing the session-discovery poll's throttle pattern). Verified against a real opencode session that this misses a real gap — the session's SQLite row existed several seconds before its token columns were populated, entirely within an idle period with no PTY output at all, so an output-gated poll could go arbitrarily long without checking. Replaced with a dedicated thread that ticks on its own fixed cadence regardless of terminal activity, signaled to stop via an `AtomicBool` once the terminal's read loop exits. See design.md Decision 4.
- [x] 3.3 Reuse the same stored message id for the exit-time read (previously created a fresh message at exit; now a final catch-up write to the same row).
- [x] 3.4 Factor `run_terminal_usage_ingestion` (per-CLI dispatch) and `record_terminal_usage_log` (Info at exit / Debug on periodic success / Warn on error) so the periodic and exit-time call sites share one implementation and can't drift apart.

## 4. Windows working-directory prerequisite fix

- [x] 4.1 Diagnose and verify directly (a live PowerShell session and a Python-subprocess-wrapped `cmd.exe` run) that a Windows extended-length path prefix (`\\?\`) is rejected or silently mishandled by `CreateProcess`'s `lpCurrentDirectory`, PowerShell's `Set-Location`, and `cmd.exe`'s `cd /d`.
- [x] 4.2 Relocate `normalize_windows_extended_length_path` from `contexts::workspaces::domain` to `platform::filesystem`, since it is now a cross-context utility.
- [x] 4.3 Apply it at the two previously-missed call sites: `terminal_process.rs`'s outer PTY spawn and wrapper generation (interactive terminals), and `process_adapter.rs`'s `command.current_dir(...)` (managed/Floating-Assistant pipeline).

## 5. Frontend live refresh

- [x] 5.1 `session-info-panel.tsx`: the `session-usage-summary` query gains `refetchInterval: 5000` while `activeSession.lifecycleState === "running"`, `false` otherwise.
- [x] 5.2 `agent-terminal-tab.tsx` invalidates the `session-usage-summary` query on the terminal's `stopped`/`failed` state event, so the final write is picked up immediately rather than waiting for the next 5-second tick.

## 6. Verification

- [x] 6.1 `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --skip mcp::infrastructure --skip connection_adapter` — 732 passed, 0 failed, 2 ignored (pre-existing). The two skipped modules contain a pre-existing, unrelated flaky test (confirmed by tracing: never touched by this change) that hangs intermittently regardless of these changes.
- [x] 6.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` — clean.
- [x] 6.3 `npm run test -- --run` — 94 files / 304 tests passed.
- [x] 6.4 `npm run lint` — clean.
- [x] 6.5 `npm run build` — succeeded (pre-existing chunk-size warnings only).
- [x] 6.6 `openspec validate add-terminal-usage-tracking --strict` — valid.
- [ ] 6.7 **Needs the user's own machine**: manual end-to-end smoke test confirming the Token Usage tab visibly updates for a running (not yet stopped) claude-code, opencode, and codex-cli embedded-terminal session. Not done here — this sandbox cannot drive the running Tauri desktop UI.
- [ ] 6.8 Gemini CLI terminal-mode usage is out of scope for this change (see proposal Non-Goals) and remains unimplemented; needs a real authenticated `chats/*.jsonl` sample before a follow-up change can add it.
