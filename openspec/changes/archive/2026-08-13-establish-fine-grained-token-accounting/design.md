## Context

See `proposal.md` for motivation. Today `usage_records` is keyed by `message_id` and stores one reported-token or estimated-character row. Managed CLI completion parsers can populate that row, while terminal ingestion overwrites one cumulative row per VaneHub session. The OnePiece/API path performs multiple HTTP requests for tool continuations and optional internal compaction or memory work, but both supported stream translators currently complete without carrying usage.

Provider fields are not uniformly additive. Cached input and reasoning output may be subsets of provider input/output totals or exclusive billing categories, and some sources expose only a cumulative session snapshot. The design must preserve those distinctions, remain local-first and redacted, and keep React, runtime adapters, native provider parsing, and SQLite behind their existing boundaries.

## Goals / Non-Goals

**Goals:**

- Make one provider request or one reconciled terminal interval the accounting grain.
- Preserve provider-reported dimensions and authoritative totals without semantic double counting.
- Make ingestion replay-safe across streams, polling, restart, log revision, and source rotation.
- Count OnePiece tool, retry, compaction, and memory calls while separating user-response and internal purposes.
- Establish the invocation ledger and its query contracts as the only first-version accounting model.

**Non-Goals:**

- Provider invoice reconciliation, currency conversion, pricing catalogs, budgets, quotas, or spend enforcement.
- Persisting raw requests, responses, prompts, terminal transcripts, headers, credentials, or provider payloads.
- Claiming exact per-turn time attribution when a provider exposes only cumulative totals without event timestamps.
- Replacing the character-based context-compaction trigger; accounting and compaction policy remain separate.
- Scraping interactive ANSI/TUI output or introducing a new tokenizer dependency to fabricate precision.

## Decisions

### 1. Introduce invocation and observation ledgers as the only accounting source

Add `model_invocations`, `token_usage_observations`, and `usage_ingestion_cursors`. An invocation owns immutable correlation and configuration snapshots; an observation owns measurement facts and provenance. All writers and readers move directly to these tables, and the unreleased `usage_records` model is removed without backfill or a compatibility projection.

An invocation includes generation/run/operation/session/message identities, stable Agent id, provider/Profile/endpoint/model snapshot, interaction kind, purpose, attempt, status and timestamps. `message_id` is nullable because internal, failed, and pre-message work can consume Tokens. An observation includes quality, measurement kind, all normalized dimensions, authoritative total, overlap semantics, source/schema version, source key, event time and observation time.

Alternative: widen `usage_records` and keep one row per message. Rejected because it cannot represent multiple paid requests, internal calls, failed attempts, or terminal intervals without lossy pre-aggregation.

### 2. Treat provider total as authoritative and dimensions as annotated facts

The normalized model stores `input_tokens`, `output_tokens`, `cached_input_tokens`, `cache_write_input_tokens`, `reasoning_output_tokens`, and `provider_total_tokens` independently. Cache and reasoning relationships are declared as `subset`, `exclusive`, or `unknown` by a versioned adapter. The aggregate headline uses a valid provider total first; otherwise only an adapter with known semantics may derive a total.

Presentation fields such as fresh input are computed only when semantics permit. Reasoning remains a first-class dimension and is never folded into output merely to preserve an unreleased DTO shape.

Alternative: define every category as mutually exclusive globally. Rejected because provider protocols do not share that guarantee and upgrades can change field meaning.

### 3. Use stable source keys plus supersession instead of mutable destructive replacement

Every observation receives a deterministic source key scoped by adapter, provider session/request identity, provider event/message identity, revision and accounting epoch. Duplicate ingestion resolves to the same logical observation. A later corrected observation supersedes the earlier revision; projections select only the active revision while retaining auditability.

Estimates use a separate source key and are superseded when matching reported data arrives. An estimate never overwrites reported data.

Alternative: rely only on database row ids and `INSERT OR IGNORE`. Rejected because it cannot distinguish replay from a corrected provider revision or link an estimate upgrade.

### 4. Reconcile cumulative terminal sources through persisted cursors and epochs

Claude Code and Gemini terminal sources should use stable per-message events where verified. Codex `token_count` snapshots and any OpenCode deployment exposing only session totals use a persisted cursor. A larger compatible snapshot produces one non-negative `reported-derived` interval; an unchanged snapshot produces nothing. Counter decrease, provider-session change, source replacement, or incompatible revision closes the epoch and opens a new one.

The event timestamp is used when present. Otherwise the delta is attributed to observation time and labeled derived; previously emitted intervals are immutable, so an entire cumulative total never moves into a later day during refresh.

Alternative: continue overwriting one cumulative message row. Rejected because range filters, response counts, and daily trends drift as the row timestamp and total change.

### 5. Meter OnePiece at the HTTP request boundary

Add an application accounting port to the API runtime and allocate an invocation before each provider request. The main response loop increments request sequence across tool continuations. Compaction and memory helpers receive explicit purposes and create their own invocations. Reported usage is committed with terminal status even when the visible generation later fails or is cancelled.

For Anthropic streams, the adapter accumulates input/cache data from message start and output usage from message delta/final events, then emits one normalized observation at message stop. For OpenAI-compatible Chat Completions, the provider-directory endpoint record declares a usage strategy. Known-compatible endpoints request stream usage and parse the final usage-bearing chunk; unknown endpoints parse opportunistically but do not trigger a second paid request solely to change a usage option.

Alternative: attach only the final request's usage to `GenerationProcessEvent::Completed`. Rejected because it loses earlier tool rounds and internal calls and couples accounting durability to UI message completion.

### 6. Keep provider parsing in infrastructure and accounting policy in application/domain

CLI/API adapters parse wire shapes and emit a provider-neutral usage candidate with declared semantics. The accounting application service validates counts, constructs identities, handles idempotency/supersession, and writes through repository ports. Session statistics consume projections rather than provider-specific structs.

React uses extended `AgentService` query contracts. The Tauri frontend adapter invokes new or versioned native commands; the Web/mock adapter maintains deterministic invocation fixtures. No React component calls Tauri directly, and no frontend code reads provider logs or SQLite.

Alternative: teach usage UI about provider JSON shapes. Rejected because it violates service isolation and makes Web/runtime parity untestable.

### 7. Cut over once without a legacy accounting path

Token accounting has not shipped in a stable release, so there is no supported historical contract to migrate. The implementation removes `usage_records`, its message-keyed writers, compatibility queries, and frontend fallbacks after their ledger replacements are in place. Existing development-only rows are intentionally discarded.

The cutover is atomic at the feature level: new writes target only the ledger and all statistics read only active ledger observations. There is no dual-write, dual-read, temporary projector, or legacy DTO mapper.

Alternative: retain a pre-release compatibility bridge. Rejected because it adds permanent complexity and test burden without a released consumer or supported data contract.

### 8. Query bounded dimensions and preserve privacy

Summary queries accept range plus optional stable Agent, provider, model, purpose, quality and status filters. Breakdown cardinality, page size and invocation-detail retention are bounded. Provider/Profile/model snapshots are safe identifiers validated by existing configuration boundaries; credentials, payloads and content never enter accounting tables.

Ingestion diagnostics use unified logging with reason codes, safe correlation ids, counts and adapter versions. Raw malformed lines, SSE data, terminal paths, prompt text and response content are excluded.

Alternative: store raw usage payload JSON for future parsing. Rejected because it expands sensitive-data risk and makes provider payloads a second transcript store.

## Risks / Trade-offs

- [Risk] Provider CLI and API event shapes drift between releases. → Pin adapter fixtures to verified shapes, version normalization semantics, degrade to explicit reduced coverage, and emit bounded diagnostics.
- [Risk] A provider's documented total conflicts with category math. → Preserve both, use the provider total for headline reporting, and surface a semantic-mismatch reason without silently rewriting facts.
- [Risk] Cumulative observations arrive out of order or after process restart. → Serialize cursor updates transactionally per source epoch and reject stale snapshot order.
- [Risk] Development databases lose pre-release usage rows. → Accept the reset explicitly and keep session/message data outside accounting untouched.
- [Risk] Invocation volume increases SQLite size and query cost. → Add correlation/time/dimension indexes, bounded detail pagination, aggregate directly in SQL, and consider rollups only after measured need.
- [Risk] OpenAI-compatible endpoints differ in stream-usage support. → Encode reviewed strategy in the provider directory and never issue a speculative paid retry.
- [Trade-off] Reported-derived terminal intervals are less precise than event-level usage. → Keep the quality visible and attribute by provider event time when available, otherwise observation time.

## Implementation Plan

1. Add ledger, cursor and indexes without importing `usage_records` data.
2. Add application accounting ports and define the first-version projection/query contracts directly from the ledger.
3. Meter OnePiece/API calls and managed CLI completion events into the ledger behind adapter-level tests.
4. Move interactive terminal ingestion to event observations or cursor-derived deltas and verify restart, rotation and final-refresh races.
5. Switch desktop and Web/mock summary/detail contracts and UI to ledger projections, then remove `usage_records`, its writers, readers and frontend fallback aggregation.
6. Run the full repository validation suite and provider fixture/live smoke checks documented in tasks.

## Open Questions

- Exact retention duration for invocation-level detail can be selected from measured database growth without changing aggregate correctness or public quality semantics.
- Antigravity interactive usage remains unsupported until a stable provider-native persisted source is verified; its managed invocation path still participates in the ledger.
