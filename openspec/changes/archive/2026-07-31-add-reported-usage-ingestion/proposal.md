## Why

All four supported CLIs (claude-code, codex-cli, gemini-cli, opencode) already report real per-response token usage in the JSON output mode VaneHub already invokes them with, but the native runtime throws it away. `SessionUsageAccountingKind::Reported` has never had a production writer (it is explicitly marked `dead_code`), so the Token Usage panel's "Reported" section is permanently empty for every session, and the system silently substitutes character-count estimates where accurate numbers were available all along. This directly contradicts the already-approved `usage-statistics` spec's "Persist reported tokens" requirement, which has simply never been implemented.

## What Changes

- Extend the provider output parsing pipeline so the "turn complete" event each CLI already emits (`result` for claude-code and gemini-cli, `turn.completed` for codex-cli, `step_finish`/`step-finish` for opencode) carries its parsed usage payload instead of being reduced to a bare completion signal.
- Add per-CLI field mapping from each CLI's native usage shape into the existing `ReportedTokenTotals` shape (input/output/cache_read/cache_creation), folding CLI-reported reasoning/thinking output tokens into the output token count.
- Change assistant-response completion handling so a session persists an `accounting_kind: reported` usage record when the originating CLI returned valid, non-degenerate usage, and only falls back to the existing character-count `estimated` record when it did not.
- Define "non-degenerate" precisely: a usage payload that is present but all-zero (e.g., a CLI error response that still emits a zero-filled usage block) is treated as absent, not as a valid zero-token reported response.
- Remove the `dead_code` allowance on `SessionUsageAccountingKind::Reported` now that it has a real producer.
- Retire the frontend `summaryWithLiveReportedTokens` fallback in the Session Info Panel that currently re-labels character-count estimates as "reported" data, now that the backend reliably persists genuine reported usage.
- **Non-goal (explicitly out of scope)**: real USD cost tracking (opencode's `cost`, claude-code's `total_cost_usd`) is not part of this change and is left for a future proposal.

## Capabilities

### New Capabilities
(none — this change implements and clarifies an existing capability)

### Modified Capabilities
- `usage-statistics`: clarifies which per-CLI completion events and fields count as "reported" usage, defines that an all-zero/degenerate usage payload is treated as absent (falls back to estimated accounting) rather than as a valid reported zero, and defines that CLI-reported reasoning/thinking tokens are folded into the reported output token count rather than dropped or tracked separately.

## Impact

- Desktop (Tauri) runtime only. This is native CLI subprocess output parsing; the Web/mock runtime fabricates its own mock usage data independently and is unaffected.
- Rust, `agent_runtime` context: `infrastructure/providers/output.rs` (and its tests), `infrastructure/providers/mod.rs` (event shape), `application/service.rs::complete_claimed()` (and its tests).
- Rust, `sessions` context: `application/models.rs` (drop the `dead_code` allowance only — no schema or migration change, `usage_records` already has the needed columns).
- Frontend: `src/main-layout/session-info-panel.tsx` and its test file (remove/simplify the live-message reported-usage fallback).
- No SQLite migration, no Tauri command signature change, no frontend service interface (`agent-service.ts`) change.
