## 1. Accounting Domain and Contracts

- [x] 1.1 Add invocation identity, interaction-kind, purpose, attempt, status, measurement-quality, measurement-kind, overlap-semantics, normalized-dimension, provenance, cursor, and projection models without provider-specific wire fields.
- [x] 1.2 Define application ports for starting/finalizing invocations, recording/superseding observations, advancing cumulative cursors transactionally, and querying summaries/details.
- [x] 1.3 Define stable source-key, epoch, normalization-version, estimate-upgrade, and authoritative-total rules with unit tests for duplicate, corrected, stale, zero, negative, overflow, and semantic-mismatch inputs.
- [x] 1.4 Extend execution correlation so generation, run, operation, session, nullable message, Agent, provider/Profile/endpoint/model snapshot, request sequence, attempt, interaction kind, and purpose are available at every accounting boundary.
- [x] 1.5 Add architecture tests that keep provider parsing in infrastructure, accounting policy behind application ports, SQLite in native infrastructure, and Tauri invocation out of React components.

## 2. SQLite Ledger, Cursor, and Migration

- [x] 2.1 Add a migration for `model_invocations` with immutable correlation/configuration snapshots, purpose, attempt, status, and lifecycle timestamps.
- [x] 2.2 Add `token_usage_observations` with non-negative normalized dimensions, authoritative total, overlap semantics, quality, measurement kind, source/schema version, stable source key, supersession, event/observation timestamps, and safe provenance.
- [x] 2.3 Add `usage_ingestion_cursors` with source identity, provider-session epoch, last cumulative vector, ordering metadata, and compare-and-swap revision.
- [x] 2.4 Add foreign keys, uniqueness constraints, and indexes for time range, correlation, Agent, provider, model, purpose, quality, status, source key, supersession, and cursor lookup.
- [x] 2.5 Implement transactional repositories for invocation lifecycle, idempotent observation insertion, supersession, estimate upgrade, cursor advancement, reset epochs, and stale-snapshot rejection.
- [x] 2.6 Remove `usage_records`, its message-keyed writers and readers after ledger cutover; do not backfill pre-release usage data.
- [x] 2.7 Verify schema repeatability, cascade behavior, unknown/deleted-session isolation, failed invocation usage, development-data reset behavior, and absence of raw content or credentials.

## 3. Projection and Query Services

- [x] 3.1 Implement one ledger-only projection policy that selects active observation revisions, suppresses superseded estimates, and prefers valid provider totals.
- [x] 3.2 Implement message and generation projections that aggregate unique initial, tool-continuation, retry, and failed/cancelled invocations while retaining request detail.
- [x] 3.3 Implement session and global range summaries with reported, reported-derived, and estimated separation, coverage, call/generation/session counts, and user-response versus internal-purpose totals.
- [x] 3.4 Add bounded breakdowns and filters for stable Agent id, provider, model, purpose, quality, and invocation status with consistent local-calendar semantics.
- [x] 3.5 Add paginated invocation-detail queries that expose safe identities, dimensions, quality, status, purpose and timestamps without raw provider payloads.
- [x] 3.6 Define first-version summary DTOs directly from ledger semantics and add parity fixtures for desktop and Web/mock aggregation.
- [x] 3.7 Add repository and application tests for authoritative totals, subset/exclusive/unknown semantics, cache-only usage, reasoning dimensions, mixed quality, filters, pagination, empty data, and inaccessible sessions.

## 4. Managed CLI Usage Adapters

- [x] 4.1 Change managed CLI completion ingestion to create one invocation observation and stop writing the message-keyed legacy summary row.
- [x] 4.2 Preserve Claude Code input, output, cache-read, cache-creation and provider total fields with a versioned verified semantic mapping.
- [x] 4.3 Preserve Codex CLI input, cached-input, cache-write, output, reasoning-output and provider total fields without unconditional reasoning/cache addition.
- [x] 4.4 Preserve Gemini CLI input, output, cached-input, reasoning/model detail when available, and provider total with versioned semantics.
- [x] 4.5 Preserve OpenCode input, output, reasoning, cache-read, cache-write and total fields while retaining step identity and revisions.
- [x] 4.6 Move managed Antigravity usage into the ledger while preserving its verified result status and field mapping.
- [x] 4.7 Add fixture tests from bounded verified provider event shapes for successful, cache-only, all-zero, malformed, revised, missing-usage, failed, cancelled and retry cases.
- [x] 4.8 Emit bounded unified diagnostics when a CLI schema is unsupported or semantically inconsistent and verify prompt, response, argument, credential and raw event content is excluded.

## 5. Interactive Terminal Reconciliation

- [x] 5.1 Refactor terminal usage polling to ingest through accounting ports without creating placeholder chat messages or parsing PTY transcript output.
- [x] 5.2 Materialize Claude Code terminal JSONL by stable provider message/revision identity and emit one active observation per turn.
- [x] 5.3 Materialize Gemini CLI terminal chat snapshots/updates by stable message identity and emit one active observation per turn.
- [x] 5.4 Convert Codex terminal cumulative `token_count` snapshots into transactional reported-derived deltas with source epochs and stale-order rejection.
- [x] 5.5 Use verified OpenCode per-message/part usage when available and otherwise convert session totals through the cumulative cursor contract.
- [x] 5.6 Detect provider-session change, counter decrease, source replacement and log rotation; open a new epoch without negative or duplicated deltas.
- [x] 5.7 Preserve periodic-worker join-before-final-refresh ordering and test concurrent poll, exit, stop, reopen, restart, unchanged snapshot, late snapshot and source-rotation races.
- [x] 5.8 Mark unverified interactive sources such as Antigravity unsupported rather than fabricating usage, and expose reduced coverage through projections.

## 6. OnePiece and API-Agent Provider Metering

- [x] 6.1 Allocate an immutable invocation snapshot before every API provider request and finalize it for success, failure, cancellation and retry outcomes.
- [x] 6.2 Increment request sequence and record separate `assistant-initial` and `tool-continuation` invocations across the bounded tool loop.
- [x] 6.3 Pass explicit `context-compaction` and `memory-extraction` purposes into internal summarization calls and account for each request independently.
- [x] 6.4 Extend the Anthropic stream adapter to accumulate message-start input/cache usage and message-delta/final output usage into one request observation.
- [x] 6.5 Extend provider-directory endpoint records with a reviewed streaming-usage strategy for supported OpenAI-compatible endpoints.
- [x] 6.6 Enable and parse final OpenAI-compatible stream usage only according to the endpoint strategy, with opportunistic parsing but no speculative paid retry.
- [x] 6.7 Preserve OnePiece Profile, endpoint, provider and model snapshots when active configuration changes during a generation.
- [x] 6.8 Add API runtime tests covering zero, one and multiple tool rounds, compaction, memory extraction, Profile switch, missing usage, partial stream, HTTP error, cancellation, retry, and duplicate terminal events.
- [x] 6.9 Verify unified logs contain only safe correlation, counts, strategy, adapter version, status and reason codes, never request/response bodies, prompts, tool payloads, headers or credentials.

## 7. Native Commands and Frontend Service Boundary

- [x] 7.1 Define versioned shared DTOs for filters, totals, semantic dimensions, coverage, breakdowns, invocation details, pagination and generated timestamps.
- [x] 7.2 Add or extend Tauri commands and native mappers for global statistics, session summaries and invocation details without exposing repository or provider wire models.
- [x] 7.3 Extend `AgentService` and the Tauri adapter with the shared query contracts while keeping all `invoke()` calls inside the Tauri-specific service layer.
- [x] 7.4 Extend the Web/mock adapter with deterministic CLI, OnePiece, multi-call, internal-purpose, reported-derived, estimated, failed and unknown-dimension fixtures.
- [x] 7.5 Add frontend contract/parity tests for filtering, totals, quality coverage, pagination, empty/error states and local-calendar ranges.
- [x] 7.6 Remove frontend fallback paths that relabel estimated values or independently reaggregate message character counts.

## 8. Usage Statistics and Session UI

- [x] 8.1 Update shared usage presentation utilities to format authoritative totals and non-additive cache/reasoning dimensions without mixing Tokens and characters.
- [x] 8.2 Add Agent, provider, model, purpose, quality and status filters that apply consistently to summaries, trends, coverage and breakdowns.
- [x] 8.3 Display total model consumption, user-response consumption, internal compaction/memory consumption, call counts and quality coverage with bounded responsive breakdowns.
- [x] 8.4 Update the compact session usage surface to show mixed-purpose totals and a bounded detail entry point without loading raw provider data.
- [x] 8.5 Add localized zh-CN/en copy for reported, reported-derived, estimated, unknown semantics, unsupported sources and non-billing limitations.
- [x] 8.6 Add component tests for loading, preserved refresh data, filter combinations, mixed quality, empty/error states, unknown dimensions, narrow viewport, both visual styles and accessible controls.
- [x] 8.7 Run Playwright coverage for Usage Statistics filtering, OnePiece multi-call presentation and session detail behavior in deterministic Web/mock mode.

## 9. Compatibility, Integrity, and Security Verification

- [x] 9.1 Verify clean and development databases adopt the ledger without importing pre-release `usage_records`, while preserving non-accounting session/message data.
- [x] 9.2 Add end-to-end idempotency tests for provider event replay, terminal poll/reopen/restart, corrected revisions, estimate upgrade and cumulative reset.
- [x] 9.3 Add invariants proving parent projections and child invocations cannot both contribute, historical terminal deltas do not move dates, and stale snapshots cannot overwrite final observations.
- [x] 9.4 Add redaction and persistence tests proving accounting tables, DTOs, unified logs and Web storage never contain prompts, responses, credentials, headers, raw protocol frames, private paths or tool payloads.
- [x] 9.5 Add bounded query/performance tests for high invocation counts, dimension cardinality, indexes, detail pagination and concurrent ingestion/query workloads.
- [x] 9.6 Perform authenticated bounded smoke runs for available Claude Code, Codex CLI, Gemini CLI, OpenCode, Antigravity and OnePiece provider paths; record versions, safe usage facts and unsupported coverage without committing raw transcripts.

## 10. Required Validation

- [x] 10.1 Run `npm run lint:ci`.
- [x] 10.2 Run `npm run test` and `npm run test:coverage`.
- [x] 10.3 Run `npm run build`.
- [x] 10.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 10.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 10.6 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 10.7 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 10.8 Run `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, and `npm run docs:check`.
- [x] 10.9 Run `npx playwright test` for the UI behavior changes.
- [x] 10.10 Run `openspec validate establish-fine-grained-token-accounting --strict` and `openspec validate --specs --strict`.
- [x] 10.11 Record provider fixture/live verification results, development-data reset evidence, unsupported-source limitations, and schema verification in the change artifacts before archive.
