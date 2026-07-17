## Context

The existing Usage Statistics implementation aggregates `messages.token_input` and `messages.token_output`. The desktop CLI runtime currently fills those columns from prompt and response character counts, while the Web adapter creates compatible mock values. The page therefore exposes a useful activity approximation but cannot identify reported tokens, cache usage, daily movement, or Agent attribution with an honest accounting unit.

The desktop runtime already receives structured output from Claude Code, Codex CLI, Gemini CLI, and OpenCode, but its parsers currently emit only content, thinking, tool use, session id, completion, and failure events. The `ChatConfig` provider/model fields are not persisted by the native send path. The upgrade crosses CLI parsing, session completion, SQLite migration, aggregation, service adapters, and settings UI, while remaining subject to unified logging, i18n, two-style visual parity, and the React/native service boundary.

## Goals / Non-Goals

**Goals:**

- Record one normalized usage row for each VaneHub-managed assistant response that has reported or estimated usage.
- Keep provider-reported token values separate from character-count estimates in storage, service contracts, aggregation, and presentation.
- Normalize supported provider token semantics into fresh input, output, cache read, and cache creation categories without double counting.
- Preserve existing history as explicitly estimated character usage.
- Return summary, coverage, local-calendar daily trend, counted sessions, and stable-Agent-id breakdowns for bounded ranges.
- Keep desktop and Web/mock behavior contract-compatible behind `AgentService`.
- Present the monitoring page in Simplified Chinese and English with semantic styling for both registered visual styles.

**Non-Goals:**

- Provider billing reconciliation, monetary cost estimation, or pricing configuration.
- Request/response detail logs, prompt or response content retention in accounting records, or usage export.
- Importing sessions created outside VaneHub or scanning provider history directories.
- Provider/model filtering, long-term daily rollup maintenance, or a new charting/UI dependency.
- Claiming that every supported CLI version emits complete usage data.

## Decisions

### Decision: Persist normalized usage separately from chat messages

Add a versioned `usage_records` table keyed by `message_id`, with `session_id`, stable `agent_id`, optional `provider_id` and `model_id`, input/output/cache counts, `accounting_kind`, `unit`, `source`, and `occurred_at`. Foreign keys tie the record to the owning message/session so session deletion removes accounting data, and indexes cover occurrence time and Agent aggregation.

`accounting_kind = reported` requires `unit = tokens`; `accounting_kind = estimated` requires `unit = characters`. Non-negative constraints protect aggregate invariants. No prompt, response, raw CLI event, credential, or error payload is stored in this table.

This is preferred over adding more columns to `messages` because usage accounting has different evolution, indexing, and retention concerns from chat display content. A request-log table like `cc-switch` is intentionally deferred because VaneHub does not need latency, status, pricing, or auditing dimensions in this change.

### Decision: Backfill existing values as estimated characters

The migration inserts one estimated record for every existing assistant message with a positive stored input or output value. It joins sessions to capture the stable Agent id, uses the message creation time, and labels the source as legacy character counting. The migration is idempotent through the `message_id` primary key.

Existing `messages.token_input` and `messages.token_output` columns remain available during this change for chat contract compatibility, but new usage aggregation reads `usage_records`. Rollback may stop reading the new table without deleting it; destructive down-migration is not required.

### Decision: Normalize usage through stateful per-generation parsing

Extend the native parser result with a normalized usage event and maintain a usage accumulator for each running generation. The accumulator handles repeated terminal events and provider-specific cumulative counters, and the terminal path upserts at most one record for the assistant message.

- Claude Code: read message usage, preserve fresh input/output/cache-read/cache-creation categories, and retain the strongest final observation for a provider message.
- Codex CLI: prefer last-turn usage when available; otherwise derive a non-negative delta from cumulative counters. Cached input is separated from inclusive input when the reported format requires it.
- Gemini CLI: map prompt, candidate plus reasoning/thought, and cached counts into the normalized categories.
- OpenCode: map input, output plus reasoning, and cache read/write values from structured message data.

If a successful response ends without reported usage, the runtime records input and output character counts as `estimated/characters`. Failed or cancelled responses are recorded only when the CLI has already reported token usage; fabricated estimates are not created for incomplete responses. A later reported observation may replace an estimate for the same message, but an estimate may never overwrite reported data.

This approach is preferred over tokenizer libraries because provider tokenization and cache semantics vary by model, and upstream-reported data is the closest available representation of actual consumption.

### Decision: Expose separate reported and estimated aggregates

Expand the service result to contain:

- reported token totals for fresh input, output, cache read, cache creation, and their sum;
- estimated input, output, and total character counts;
- reported, estimated, and total response counts plus a reported coverage percentage;
- counted sessions;
- daily points carrying the same separated totals;
- per-Agent rows keyed by stable Agent id with the same totals and counts;
- range and generation timestamp metadata.

Reported total tokens equal the sum of all four normalized token categories. Estimated characters are never added to token totals. Zero-data ranges return zero-valued totals and empty breakdown arrays.

### Decision: Use runtime-local calendar boundaries consistently

`today`, `last7Days`, and `last30Days` are calendar ranges in the user's runtime-local time zone, not rolling hour windows and not UTC-midnight ranges. The native implementation uses the operating system local time conversion for filtering and daily buckets; the Web adapter uses browser-local `Date` semantics. Aggregation helpers receive a controllable clock in tests so date-boundary behavior remains deterministic. `all` includes all usage records.

This corrects the existing desktop/Web mismatch while avoiding a new time-zone dependency. Desktop and browser sessions running on different machines may naturally use their respective local zones.

### Decision: Keep the synchronous command bounded and the UI nonblocking

The native command performs indexed, read-only aggregate queries and does not scan files, start CLIs, or access the network. The page requests data through `AgentService.getUsageStatistics`, React Query, the Tauri adapter, and the declared command. The Web adapter aggregates mock records into the identical contract.

The page retains stale data during refresh, offers manual refresh, and polls only while mounted at a modest interval. If real-world database size later makes the query variable-duration, the query can move to the operation/task foundation without changing React's runtime isolation.

### Decision: Adapt the reference layout to VaneHub primitives

Use the `cc-switch` dashboard only as an information-architecture reference: compact header controls, summary, trend, and categorical breakdown. Build the trend with repository-native React/SVG and split the page into focused components so no source file exceeds the project limit. Styling uses shared page parts and semantic tokens with no page-specific theme branches, hard-coded palette, inline style, or new component/chart library.

All visible text, accessible labels, empty/error/loading states, accounting explanations, numbers, dates, and times follow active-locale i18n behavior. Both `futuristic` and `minimal` styles are visually checked at desktop and narrow widths.

### Decision: Log parser diagnostics without retaining raw events

Unsupported or malformed usage shapes may emit rate-limited `debug` or `warn` diagnostics through unified logging with session and Agent context. Logs must not contain raw prompts, responses, complete CLI events, credentials, or token-like secrets, and redaction occurs before persistence. Missing usage is a supported fallback condition and must not produce noisy per-line errors.

## Risks / Trade-offs

- [Risk] CLI output schemas can change between versions. → Keep Agent-specific parsing isolated, tolerate unknown fields, use representative fixtures, and fall back to explicitly estimated characters when no valid reported usage exists.
- [Risk] Cumulative token events can double count resumed or repeated turns. → Use a per-generation accumulator, prefer turn-level counters, compute saturating deltas for cumulative formats, and upsert by assistant message id.
- [Risk] Provider input-token semantics differ on whether cache tokens are inclusive. → Normalize per Agent before persistence and test inclusive and exclusive fixtures separately.
- [Risk] Historical values are not tokens despite legacy column names. → Backfill them with `estimated/characters` labels and never combine them with reported totals.
- [Risk] Local time and daylight-saving transitions can make range tests fragile. → Convert each timestamp through the runtime local calendar and inject deterministic clocks/time-zone fixtures into aggregation tests.
- [Risk] Aggregate queries may slow down after substantial history growth. → Index occurrence and Agent dimensions, avoid loading message bodies, and defer rollups until measured data warrants them.
- [Risk] A polished dashboard may still be mistaken for billing data. → Keep a persistent localized accounting note and avoid costs, currency, quota, or billing-grade language.

## Migration Plan

1. Add the `usage_records` migration, constraints, foreign keys, and indexes.
2. Backfill positive legacy assistant-message values as estimated character records in the same versioned migration.
3. Add normalized parser/accumulator behavior and dual-write compatible message completion behavior.
4. Switch native and Web usage aggregation to the expanded separated contract.
5. Update adapters and the page after both runtime implementations compile against the new contract.
6. Verify migration and aggregation against empty, legacy-only, reported-only, and mixed databases.

Rollback reverts runtime and frontend readers to the prior message-based summary. The additive table may remain unused so no user data is destroyed. A later cleanup migration may remove it only after explicit product approval.

## Open Questions

No product-scope question blocks implementation. Before finalizing Agent parsers, implementation should validate representative structured-output fixtures against the installed supported CLI versions; an unrecognized format must retain the estimated fallback rather than delaying the rest of the capability.
