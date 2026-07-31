## Context

`agent_runtime` infrastructure reads each CLI subprocess's stdout line by line through `ProviderOutputParser` (`infrastructure/providers/output.rs`), which dispatches to `parse_claude_line` (claude-code) or `parse_structured_json_line` (codex-cli, gemini-cli, opencode) and emits `ProviderOutputEvent`s. Every one of the four CLIs already emits a "turn complete" line carrying real usage data in the exact JSON output mode `invocation.rs` already requests, but the parser reduces that line to a payload-less `ProviderOutputEvent::Completed`:

- claude-code: `{"type":"result", ..., "usage":{"input_tokens":N,"output_tokens":N,"cache_creation_input_tokens":N,"cache_read_input_tokens":N}}`
- codex-cli: `{"type":"turn.completed","usage":{"input_tokens":N,"cached_input_tokens":N,"cache_write_input_tokens":N,"output_tokens":N,"reasoning_output_tokens":N}}`
- gemini-cli: `{"type":"result", ..., "stats":{"input_tokens":N,"output_tokens":N,"cached":N,"total_tokens":N,"models":{...}}}`
- opencode: `{"type":"step_finish", "part":{"type":"step-finish","tokens":{"total":N,"input":N,"output":N,"reasoning":N,"cache":{"read":N,"write":N}},"cost":N}}`

`agent_runtime/application/service.rs::complete_claimed()` is the sole production writer of usage today, and it unconditionally builds a character-count-based `AgentUsageRecord` (`source: "character-count"`). That record is mapped by `sessions_gateway.rs::session_usage()` to `SessionUsageAccountingKind::Estimated` with no alternative path — `Reported` is `#[expect(dead_code)]`. `sessions` persistence (`SqliteSessionsRepository::complete_message` → `upsert_usage`) already supports both accounting kinds against the existing `usage_records` schema; only the writer side needs to change.

## Goals / Non-Goals

**Goals:**
- Parse and persist real reported token usage for all four CLIs from their already-enabled JSON output mode — no new CLI flags or invocation changes.
- Preserve the existing invariant of at most one normalized usage record per assistant response, with `reported` taking precedence over `estimated` and estimates never overwriting a reported record.
- Ship with zero SQLite schema/migration changes and zero Tauri command/DTO signature changes.
- Leave the global usage-statistics aggregate (`SqliteSessionsRepository::statistics`) untouched — it already aggregates whatever `usage_records` holds, so reported rows flow through automatically.
- Retire the frontend fallback that currently re-labels character-count estimates as "reported" once the backend reliably persists genuine reported usage.

**Non-Goals:**
- No cost/USD tracking (opencode's `cost`, claude-code's `total_cost_usd`) — deferred to a future change.
- No change to interactive/embedded-terminal CLI sessions (`build_interactive_invocation`). Those are raw PTY passthrough for TUI rendering and never go through `ProviderOutputParser`, so they are structurally unaffected either way.
- No change to the Web/mock runtime, which fabricates its own mock usage independent of real CLI parsing.
- No redesign of the Token Usage panel beyond removing the now-redundant fallback.
- No new displayed bucket for reasoning/thinking tokens (see Decisions).

## Decisions

0. **Correction found during implementation: there are two "Completed" events, not one, and they are structurally disconnected today.** `ProviderOutputEvent::Completed` (the per-line signal parsed in `output.rs`, where CLI usage actually lives) is currently discarded outright in `process_adapter.rs::ProcessMonitor::run()` (`ProviderOutputEvent::Completed | ProviderOutputEvent::Empty => None`). The event that actually reaches `application/service.rs::completed()` is `GenerationProcessEvent::Completed`, which is synthesized purely from the child process's OS exit code after the stdout-reading loop ends, independent of whether a `result`/`turn.completed`/`step_finish` line was ever seen. Carrying usage therefore requires two hops, not one:
   - `ProviderOutputEvent::Completed(Option<ProviderReportedUsage>)` (infrastructure, per-CLI raw shape) — set in `output.rs`.
   - `ProcessMonitor::run()` captures that payload into a local (mirroring how it already tracks `terminal_error`/`emitted_content` across the read loop) and attaches a normalized copy to the terminal `GenerationProcessEvent::Completed(Option<ReportedUsageTotals>)` (application-layer type, defined in `application/models.rs`) it constructs from the exit code.
   - A small `normalize_provider_usage()` in `process_adapter.rs` converts `ProviderReportedUsage` → `ReportedUsageTotals` at that boundary, mirroring the existing `normalize_provider_tool()` conversion right next to it. This keeps `application/models.rs` free of an infrastructure-defined type, consistent with the project's layering rule that application code must not depend on concrete I/O/adapter types.

1. **Carry usage on the existing `Completed` event rather than a new event, at both layers above.** Every CLI emits its usage atomically with its own completion line, so there is no ordering benefit to a separate `Usage` event at either layer — it would only add a sequencing assumption (usage-before-completed) neither the line parser nor the process monitor needs to make, and extra branching for an event that only matters at completion time anyway.

2. **Per-CLI field mapping stays inside `output.rs`.** `parse_claude_line` and `parse_structured_json_line` already special-case JSON *shape* per CLI family; extracting `usage`/`stats`/`tokens` is one more shape detail of the same line, not a domain concern. Each parser produces a common `ProviderReportedUsage { input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens }` (a new, small `agent_runtime` infrastructure type). `application/service.rs::complete_claimed()` consumes the application-layer `ReportedUsageTotals` (see Decision 0) and is the one place that decides reported-vs-estimated and builds the `sessions`-owned `AgentUsageRecord` — `agent_runtime` never reaches into `sessions`' domain types directly; `AgentUsageRecord` gains `cache_read_count`, `cache_creation_count`, and a small `agent_runtime`-local `AgentUsageAccountingKind { Reported, Estimated }` enum, which `infrastructure/sessions_gateway.rs::session_usage()` — the existing translation seam — maps onto `sessions`' own `SessionUsageAccountingKind`/`SessionUsageUnit`, preserving the `openspec/project.md` cross-context boundary.

3. **Fold reasoning/thinking output tokens into `output_tokens` at parse time**, rather than threading a fifth field through the pipeline. This matches how providers actually bill reasoning tokens (as output), and avoids a schema/DTO change for a number the UI has nowhere to show yet.
   - Alternative considered: add a `reasoning_tokens` column end-to-end now. Rejected for this change — no consumer for it yet, and it would require a migration and DTO change the proposal explicitly scopes out; revisit if the panel later wants to break it out.

4. **Treat an all-zero usage payload as "no reported usage" and fall back to the estimated path**, rather than persisting a reported zero. Verified empirically: a CLI can return a well-formed, fully-populated `usage` object that is all zeros on a degenerate turn (observed directly from claude-code on an auth-failure response that still carried `"usage":{"input_tokens":0,"output_tokens":0,...}` alongside `"is_error":true`). Persisting that as `reported` would create a permanently-stuck, misleadingly-precise zero that can never later upgrade to a real number for that response.
   - Alternative considered: gate solely on each CLI's own error/status flag. Rejected as the *primary* signal — shapes differ per CLI, and CLI-reported errors are already separately routed to `ProviderOutputEvent::Failed` before reaching `Completed` in the existing parser. The all-zero check is a structural backstop for the remaining degenerate-but-not-technically-"failed" cases, not a replacement for existing error routing.

5. **No Tauri command or DTO changes.** `dto::SessionUsageSummary` / `get_session_usage_summary` already round-trip whatever `SqliteSessionsRepository::summary_for_session` computes from `usage_records`. This change only affects which `accounting_kind` gets written, so the read path, mapper, and DTO are untouched.

6. **Sequence frontend fallback removal last.** `summaryWithLiveReportedTokens` in `session-info-panel.tsx` is already a no-op whenever `summary.reported.totalTokens > 0`. Keep it in place (as a safety net) until the backend path has test coverage for all four CLIs and has been manually smoke-tested end-to-end in the running app, then remove the function and the `messages`-based re-derivation, letting the panel trust the backend summary directly.

## Risks / Trade-offs

- [Risk] CLI JSON schemas are unversioned and can change across CLI releases, silently regressing back to always-estimated. → [Mitigation] Missing-pointer paths fall back to `Estimated` (today's behavior) instead of erroring, so drift degrades gracefully; add one fixture-based unit test per CLI pinned to the exact verified JSON shape so a breaking CLI upgrade fails an obvious test rather than silently regressing unnoticed.
- [Risk] gemini-cli's usage shape was confirmed by reading the installed npm package's bundled source (`StreamJsonFormatter.convertToStreamStats`), not by a live authenticated run — no local credentials were available during investigation. → [Mitigation] `tasks.md` includes an explicit manual verification step (one authenticated `gemini -p ... -o stream-json` run) before considering the gemini-cli branch done, matching the empirical rigor already applied to the other three CLIs.
- [Risk] Folding reasoning tokens into `output_count` inflates "output" relative to a strict input/output mental model. → [Mitigation] Matches provider billing semantics; called out explicitly in the proposal and spec delta rather than decided silently.
- [Risk] Removing the frontend fallback could reintroduce a "no data" regression if some CLI/edge case has a backend gap the tests missed. → [Mitigation] Fallback removal is the last task, gated on backend tests for all four CLIs plus manual end-to-end smoke-testing per CLI in the running app.

## Migration Plan

- No SQLite migration — reuses existing `usage_records` columns and CHECK constraints.
- No feature flag needed: the frontend fallback already backs off automatically once `reported.totalTokens > 0`, so the new backend path can take effect session-by-session as it ships, with no coordinated flip. Existing historical `estimated` rows are untouched.
- Rollback: reverting the `agent_runtime` parsing/service commit alone restores today's estimate-only behavior; no data cleanup is needed since previously-written `estimated` rows remain valid regardless.

## Open Questions

- Should reasoning tokens eventually get their own displayed bucket (requiring a follow-up schema and spec change), or is folding into output acceptable long-term? Deferred — not blocking this change.
- Should a genuinely zero-token response (e.g., cancelled before any output) be distinguished from a degenerate all-zero error payload? Current design treats them the same, falling back to the estimated path in both cases, which seems acceptable since a truly empty response also has ~0 estimated characters.
