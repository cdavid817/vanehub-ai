## Why

Interactive (embedded-terminal) CLI sessions for claude-code, opencode, and codex-cli never persist reported token usage: `add-reported-usage-ingestion` explicitly scoped interactive terminals out, since raw PTY passthrough never goes through VaneHub's managed-invocation output parser. This means the Session Info Panel's Token Usage tab is permanently empty for any session run through the embedded terminal — the majority of real usage, as opposed to the Floating Assistant's managed/non-interactive pipeline — and even once a CLI is taught to report usage, reading it only after the user clicks Stop leaves the panel showing nothing while a session is actively running.

## What Changes

- Extend reported-token-usage persistence to interactive/embedded-terminal CLI sessions for claude-code, opencode, and codex-cli, by reading each CLI's own persisted session log/database directly (interactive terminals bypass VaneHub's managed output parser entirely, so there is no output stream to parse there).
  - claude-code: tail its own session JSONL (`~/.claude/projects/<cwd-hash>/<session-id>.jsonl`) and aggregate the `usage` field of every `assistant` event.
  - opencode: locate the session opencode created for this working directory in its own SQLite database (`~/.local/share/opencode/opencode.db`) and read its running per-session token totals.
  - codex-cli: locate the rollout file codex wrote for this working directory and read the last cumulative `token_count` event.
- While an interactive terminal session is open, periodically re-read the CLI's own usage data and refresh the persisted usage record in place, instead of only reading once after the process exits — so the Token Usage tab updates live while the session runs instead of only after the user stops it.
- Fix a related pre-existing bug: a Windows extended-length path prefix (`\\?\`) is not accepted by `CreateProcess`'s working-directory parameter, which could start an interactive terminal (and, separately, the managed/Floating-Assistant process) in the wrong working directory. This is a prerequisite for the terminal-mode CLIs above, which locate their own session files by matching working directory.
- **Non-goal (explicitly out of scope)**: Gemini CLI. Its terminal-mode session log location (`~/.gemini/tmp/<project-id>/chats/session-*.jsonl`) has not been verified against a real authenticated sample; left for a follow-up change.

## Capabilities

### New Capabilities
(none — this change extends an existing capability)

### Modified Capabilities
- `usage-statistics`: extends the "Persist reported tokens" requirement to cover interactive/embedded-terminal CLI sessions (previously scoped only to VaneHub's managed/non-interactive invocation pipeline), and adds a requirement that a running session's usage summary SHALL refresh periodically while its terminal remains open, rather than only once the session stops.

## Impact

- Desktop (Tauri) runtime only; embedded terminals do not exist in the Web/mock runtime.
- Rust, `agent_runtime` context: new `infrastructure/terminal_usage_ingestion.rs`; `infrastructure/terminal_process.rs` (placeholder usage message, periodic poll, and exit-time read, all sharing one message id so repeated writes update the same row); `infrastructure/providers/session_capture.rs` (new post-hoc, after-the-fact lookup helpers, reusing the existing live-discovery module); `infrastructure/process_adapter.rs` (working-directory normalization fix, shared with the managed/Floating-Assistant pipeline).
- Rust, `platform` context: `platform/filesystem/mod.rs` gains the relocated `normalize_windows_extended_length_path` helper (moved from `workspaces::domain`, since it is a general-purpose utility now used by both the interactive-terminal and workspace paths).
- Frontend: `src/main-layout/session-info-panel.tsx` (usage-summary query refetches periodically while the session is running); `src/session-workspace/agent-terminal-tab.tsx` (invalidates the usage-summary query on terminal stop/fail).
- No SQLite migration, no Tauri command signature change. Reuses the existing `usage_records` schema and the `reported`/`tokens` accounting kind introduced by `add-reported-usage-ingestion` (a sibling, not-yet-archived change this one builds on).
- Gemini CLI's terminal-mode usage is excluded from this change (see Non-Goals) and remains without reported usage for interactive sessions until a follow-up change.
