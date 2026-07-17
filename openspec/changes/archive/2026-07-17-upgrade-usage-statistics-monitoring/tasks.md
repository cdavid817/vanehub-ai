## 1. Usage Contract and Migration Model

- [x] 1.1 Replace the frontend usage summary types with separated reported-token totals, estimated-character totals, coverage, daily trend, and stable-Agent-id breakdown contracts.
- [x] 1.2 Add Rust normalized usage record, accounting kind/unit, aggregate response, trend point, and Agent breakdown models with explicit non-negative validation.
- [x] 1.3 Add a versioned SQLite `usage_records` migration with message/session ownership, privacy-safe fields, accounting constraints, and occurrence/Agent indexes.
- [x] 1.4 Backfill positive legacy assistant-message usage as idempotent `estimated/characters` records and add migration tests for mixed, empty, and repeated migration runs.

## 2. Native CLI Usage Capture

- [x] 2.1 Extend the per-generation parser contract and accumulator to carry normalized usage without sharing state across sessions.
- [x] 2.2 Parse and test Claude Code fresh input, output, cache-read, cache-creation, repeated observation, and malformed usage fixtures.
- [x] 2.3 Parse and test Codex CLI turn-level and cumulative token events, cache-inclusive input normalization, saturating deltas, and duplicate terminal fixtures.
- [x] 2.4 Parse and test Gemini CLI input, candidate/reasoning output, cached token, model, zero-value, and malformed usage fixtures.
- [x] 2.5 Parse and test OpenCode input, output/reasoning, cache read/write, provider/model, zero-value, and malformed usage fixtures.
- [x] 2.6 Upsert one usage record per terminal assistant response, prefer reported data over estimates, retain reported usage on failed/cancelled runs, and estimate characters only for successful runs without reported usage.
- [x] 2.7 Route rate-limited usage parser diagnostics through unified logging with redaction and tests that raw prompts, responses, CLI events, and secrets are not persisted.

## 3. Native Aggregation and Command

- [x] 3.1 Implement deterministic runtime-local boundaries for today, last seven days, last thirty days, and all time, including local-date bucket tests.
- [x] 3.2 Implement indexed SQLite summary and coverage aggregation that never adds estimated characters to reported token totals.
- [x] 3.3 Implement daily trend and stable-Agent-id breakdown aggregation without loading message bodies.
- [x] 3.4 Return zero-valued summaries and empty breakdowns for no-data ranges and add reported-only, estimated-only, mixed, multi-session, and deletion-cascade tests.
- [x] 3.5 Update and register the bounded read-only `get_usage_statistics` Tauri command with structured unsupported-range errors.

## 4. Frontend Service and Runtime Adapters

- [x] 4.1 Update `AgentService.getUsageStatistics` and the Tauri adapter to use the expanded monitoring contract without exposing `invoke()` to React components.
- [x] 4.2 Replace Web/mock message-total aggregation with normalized mock usage records and matching local-calendar summary, coverage, trend, and Agent aggregation.
- [x] 4.3 Add adapter parity tests that feed equivalent fixtures to desktop-compatible and Web aggregation helpers and compare accounting quality, dates, totals, coverage, and Agent attribution.
- [x] 4.4 Preserve previously loaded React Query data during manual and mounted polling refreshes, and stop polling when the page is unmounted.

## 5. Usage Statistics Page

- [x] 5.1 Split the page into focused range/refresh controls, summary and coverage cards, native SVG trend, Agent breakdown, and accounting-note components while keeping each file within the project size limit.
- [x] 5.2 Render reported tokens and estimated characters as visibly separate units with localized zero, loading, refreshing, empty, and error states.
- [x] 5.3 Implement responsive trend and Agent breakdown layouts using shared settings primitives, lucide icons, and semantic tokens without inline styles, hard-coded palettes, or page-specific theme branches.
- [x] 5.4 Add synchronized zh-CN and en resources for all visible text, accessible labels, limitations, and locale-aware number/date/time formatting.
- [x] 5.5 Add component tests for reported-only, estimated-only, mixed, empty, error, range-change, refresh-preservation, and stable-Agent-id rendering behavior.

## 6. Visual and End-to-End Verification

- [x] 6.1 Add or update Playwright coverage for navigating to Usage Statistics, changing ranges, refreshing, and rendering representative monitoring data.
- [x] 6.2 Inspect desktop and narrow widths in both `futuristic` and `minimal` styles and verify no overlap, clipping, blank panels, inaccessible controls, or unreadable contrast.
- [x] 6.3 Run i18n resource parity and visible-text guardrail tests and correct every new or changed settings literal.

## 7. Full Validation

- [x] 7.1 Run `npm run lint` and `npm run test`.
- [x] 7.2 Run `npm run build`.
- [x] 7.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 7.4 Run `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 7.5 Run `openspec validate "upgrade-usage-statistics-monitoring" --strict` and `openspec validate --specs --strict`.

## 8. Verification Follow-ups

- [x] 8.1 Correct multi-stage cumulative usage accounting and cover increasing, duplicate, and regressed counter sequences.
- [x] 8.2 Split all-time and bounded native SQL so bounded ranges use occurrence-time index seeks, with query-plan regression coverage.
- [x] 8.3 Complete zh-CN/en accounting-limitations copy for billing, request-detail, and Provider/model-filter boundaries.
- [x] 8.4 Replace hard-coded adapter-parity expectations with a shared normalized fixture consumed by Rust and TypeScript tests.
- [x] 8.5 Add QueryObserver lifecycle coverage for stale-data preservation across range changes and observer cleanup.
- [x] 8.6 Seed representative Web monitoring records and assert real summary, trend, and stable-Agent rows in Playwright.
- [x] 8.7 Re-run the complete frontend, Rust, Playwright, and strict OpenSpec validation suite after follow-up changes.
