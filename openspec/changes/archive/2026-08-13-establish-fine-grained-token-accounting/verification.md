# Verification Record

## Provider fixtures

- Managed CLI fixtures cover Claude Code, Codex CLI, Gemini CLI, OpenCode and Antigravity success, cache-only, zero, malformed, revised, missing, failed, cancelled and retry shapes.
- Terminal ledger fixtures cover Claude and Gemini stable revisions, Codex and OpenCode cumulative reset epochs, stale ordering, source rotation and private-path hashing.
- OnePiece fixtures cover Anthropic start/delta usage and reviewed OpenAI-compatible final stream usage. OpenAI, OpenRouter and DeepSeek chat endpoints opt in to `include_usage`; other endpoints remain opportunistic and never trigger a paid retry.
- API runtime tests cover zero/one/multiple tool rounds, failure, cancellation, compaction, memory extraction and immutable Profile/model/endpoint snapshots.

## Bounded authenticated smoke runs

Executed on 2026-08-13 with the fixed prompt `Reply exactly OK.` and without persisting raw transcripts:

| Path | Version | Result | Safe usage fact |
| --- | --- | --- | --- |
| Claude Code | 2.1.229 | succeeded | one turn; input 27,643; output 2; cache read/write 0 |
| Codex CLI | 0.147.0 | timed out after 90 seconds | no terminal usage event observed |
| Gemini CLI | 0.53.0 | timed out after 90 seconds | no terminal usage event observed |
| OpenCode | 1.18.11 | succeeded | one step; input 12,445; output 3; reasoning/cache 0; provider total 12,448 |
| Antigravity | unavailable | unsupported | invocation-only coverage records the unsupported source |
| OnePiece | application provider fixture | fixture verified | Anthropic and reviewed OpenAI-compatible usage dimensions preserved |

No prompt/response body, API key, header, raw protocol frame, private path, tool payload, provider session id or transcript is committed here.

## Schema and development-data reset

- Migration 22 is a repeatable no-op retained only to keep migration history dense.
- Migration 62 drops the pre-release `usage_records` table, creates the ledger/cursor schema, and imports no old accounting rows.
- Re-running migrations leaves the ledger schema intact. The migration fixture proves non-accounting agents, sessions and messages survive the reset.
- Foreign keys isolate unknown sessions, cascade invocations and observations when a session is deleted, and retain failed/cancelled invocation usage.
- Accounting columns and DTOs contain correlation, normalized counts, safe revisions and hashes only; tests reject raw-content field classes.

## Query and integrity verification

- Source-key replay is idempotent; corrected revisions supersede prior observations; reported data upgrades estimates; cumulative resets require a new epoch.
- Projection selects only active observations, so parent/message summaries and child requests cannot both contribute.
- Daily aggregation uses `event_at` before `observed_at`, preserving historical terminal usage dates.
- High-cardinality tests bound breakdowns and details pagination and exercise concurrent ingestion/query access.

## Known coverage limits

- CLI schemas are external contracts and can change; unsupported or semantically inconsistent shapes emit bounded reason codes and reduce coverage rather than fabricating totals.
- Interactive Antigravity usage remains unsupported until a verified machine-readable source exists.
- Codex and Gemini authenticated smoke runs did not complete within the bounded timeout on this workstation; fixture, parser and reconciliation coverage remains active.
- Usage is operational accounting, not a billing statement.
