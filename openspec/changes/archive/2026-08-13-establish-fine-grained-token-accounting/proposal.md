## Why

VaneHub currently stores at most one usage row per assistant message, which cannot accurately represent OnePiece tool continuations, compaction and memory calls, CLI retries, or cumulative interactive-terminal snapshots. A provider-aware invocation ledger is needed now so reported Token totals remain idempotent, explainable, and comparable across built-in CLIs and the native OnePiece Agent without presenting estimates as billing records.

## What Changes

- Add an immutable, invocation-grained Token accounting ledger for managed CLI calls, interactive CLI observations, and OnePiece provider API calls.
- Preserve provider-reported input, output, cached-input, cache-write, reasoning-output, and authoritative total fields together with their overlap semantics instead of irreversibly folding or naively summing them.
- Correlate usage with generation, run, session, message, Agent, provider, model, attempt, interaction kind, and purpose, including tool continuations, retries, context compaction, and memory extraction.
- Ingest OnePiece usage from Anthropic Messages and supported OpenAI-compatible streaming responses on every provider request, including multi-round tool execution.
- Convert cumulative interactive-terminal observations into idempotent deltas with reset/rotation epochs while retaining event-level usage where provider logs expose it.
- Keep reported, reported-derived, and estimated measurements visibly separate; retain character estimates as a fallback when no valid provider usage is available.
- Project the new ledger into first-version session and global usage contracts with breakdowns by Agent, provider, model, purpose, and accounting quality.
- Apply the unified logging and redaction boundary to ingestion diagnostics without persisting prompts, model responses, credentials, or raw provider payloads.
- Affect both runtimes: the Tauri desktop runtime performs real ingestion and SQLite persistence, while the Web/mock adapter provides deterministic contract-compatible invocation and aggregate fixtures.

## Capabilities

### New Capabilities

- `token-accounting`: Defines the provider-neutral invocation ledger, usage observations, normalization semantics, idempotency, cumulative-snapshot reconciliation, privacy, and projections.

### Modified Capabilities

- `usage-statistics`: Changes aggregation from one row per assistant response to invocation-derived projections and adds provider, model, purpose, quality, and call-count breakdowns.
- `api-agent-runtime`: Requires every OnePiece/API-Agent provider request, including internal and tool-continuation calls, to capture reported usage when available.
- `onepiece-native-agent`: Makes fine-grained Token accounting part of OnePiece runtime parity across its supported provider Profiles.
- `agent-terminal-runtime`: Replaces whole-session cumulative replacement semantics with event-level observations or idempotent cumulative deltas and reset handling.
- `settings-usage-statistics-ui`: Adds clear consumption, internal-purpose, coverage, source-quality, provider, and model presentation without implying provider billing reconciliation.

## Impact

- Native session and Agent runtime application models, ports, provider adapters, terminal ingestion, SQLite schema/migrations, query projections, Tauri commands, and DTO mappers.
- Frontend `AgentService`, Tauri and Web/mock adapters, shared chat/usage types, Usage Statistics settings, and compact session usage presentation.
- Because Token accounting has not shipped in a stable release, the invocation ledger replaces `usage_records` outright; no historical usage backfill, legacy projection, or dual-read period is required.
- No frontend component gains direct Tauri or SQLite access. Provider-specific parsing remains in native infrastructure adapters, normalized accounting remains behind application ports, and React continues to depend only on the frontend service boundary.
- Unified logging receives bounded safe ingestion metadata and correlation identifiers only; no feature-local log or raw provider transcript is introduced.
