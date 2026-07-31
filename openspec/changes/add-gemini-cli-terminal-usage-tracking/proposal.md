## Why

`add-terminal-usage-tracking` extended reported-token persistence to interactive embedded-terminal sessions for claude-code, opencode, and codex-cli, but explicitly deferred gemini-cli as a Non-Goal — its terminal-mode session log location was identified only by reading the installed npm package's bundled source, without a real sample file to verify against, and the sandbox environment had no gemini-cli authentication to produce one. This change closes that gap: real OAuth credentials and a genuine `~/.gemini/` directory structure are now available, and this proposal documents the verified (real bundled-source-read, not guessed) recording format, extending the same terminal-mode reported-usage tracking to the fourth CLI.

## What Changes

- Add gemini-cli support to the interactive/embedded-terminal reported-usage tracking already shipped for claude-code, opencode, and codex-cli: read gemini-cli's own `ChatRecordingService` output (`~/.gemini/tmp/<project-slug>/chats/session-*.jsonl`) directly, the same post-hoc/periodic-poll pattern already used for the other three CLIs.
- Resolve the project-slug directory by reading `~/.gemini/projects.json` (a normalized-absolute-path → slug map gemini-cli itself maintains and auto-populates on first use in any directory) rather than assuming a fixed hash scheme — verified directly against the real, evolving gemini-cli source, which recently migrated from a legacy hash-based scheme to this slug-based one.
- Sum reported tokens across every `type: "gemini"` message record in the matched session file (each carries its own per-turn `tokens: {input, output, cached, thoughts, tool, total}` object — a genuinely different, richer shape than the managed/non-interactive pipeline's `stream-json` `stats` summary already mapped by `add-reported-usage-ingestion`), folding `thoughts` (reasoning) into the output count consistent with how codex-cli and opencode already fold their own reasoning/thinking tokens.
- **Non-goal (explicitly out of scope)**: gemini-cli's `tool` token count (tokens spent on tool-use prompt context) is not mapped to any existing bucket — folding it into `input` risks double-counting, since it is very likely already a breakdown/subset of `promptTokenCount` rather than additive, and there is no way to confirm this distinction from the source alone without a real multi-tool-call sample.

## Capabilities

### New Capabilities
(none — this change extends an existing capability)

### Modified Capabilities
(none — `usage-statistics`'s "Persist reported tokens for an interactive terminal session" scenario already reads generically across "a supported CLI"; no requirement wording changes, only implementation coverage expands to the fourth CLI)

## Impact

- Desktop (Tauri) runtime only; embedded terminals do not exist in the Web/mock runtime.
- Rust, `agent_runtime` context: `infrastructure/terminal_usage_ingestion.rs` (new `ingest_gemini_terminal_usage`/`aggregate_gemini_usage`), `infrastructure/providers/session_capture.rs` (new `find_gemini_chat_session_since`, mirroring the existing opencode/codex-cli post-hoc lookups), `infrastructure/terminal_process.rs` (dispatch wiring — `"gemini-cli"` joins the existing `claude-code | opencode | codex-cli` match arms for placeholder creation and `run_terminal_usage_ingestion`).
- No SQLite migration, no Tauri command signature change. Reuses the existing `usage_records` schema and `reported`/`tokens` accounting kind.
- **Verification gap carried forward, not newly introduced**: this implementation is grounded in reading the actual installed `@google/gemini-cli` package's bundled source (`ChatRecordingService`, `ProjectRegistry.getShortId`) — the same rigor `add-reported-usage-ingestion` already applied to gemini-cli's managed-pipeline mapping — but a live authenticated interactive run to produce a genuine `chats/*.jsonl` sample was attempted and did not complete in time; fixture-based tests are pinned to the exact shape read from source, not a live-captured sample. Flagged as a remaining manual verification task, matching the existing precedent for this CLI.
