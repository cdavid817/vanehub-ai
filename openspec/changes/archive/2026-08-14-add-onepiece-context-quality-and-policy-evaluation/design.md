## Context

See `proposal.md` for motivation. The four preceding OnePiece context changes established provider-neutral snapshots, an optimizer and verifier, token-aware triggering with suppression, and a content-free evidence card. Native unified logs already contain bounded diagnostics, but logs are not a product query contract, evidence cards cover only successful compactions, and there is no deterministic corpus for comparing a candidate policy with the active policy.

The implementation must keep React behind service interfaces, keep SQLite in Rust, preserve generation success when diagnostics fail, and avoid persisting raw prompts or provider payloads. Existing `agent-execution-observability` and `unified-log-management` facilities remain diagnostic sinks rather than sources of truth for this feature.

## Goals / Non-Goals

**Goals:**

- Produce one normalized assessment at the existing compaction decision boundary.
- Preserve content-free outcome history independently of chat-card retention.
- Make quality history and aggregates queryable through matching desktop and Web/mock contracts.
- Detect policy regressions deterministically before a policy version is promoted by a future code change.
- Present operational policy health and measurement limitations in the OnePiece settings surface.

**Non-Goals:**

- Judging semantic answer quality with an LLM or sending evaluation data off-device.
- Automatically promoting, rolling back, or remotely configuring a policy.
- Parsing unified logs as a history database.
- Manual compaction, evidence export, managed CLI context mutation, or provider-native prompt-cache edits.

## Decisions

### 1. Assess the final coordinator outcome once

The existing automatic-compaction coordinator will construct a `ContextQualityAssessment` after it resolves optimizer success, compatibility fallback, bypass, or failure. The assessment uses an attempt id derived from generation correlation and the compaction decision sequence. Successful evidence cards reuse that attempt id.

This central point avoids duplicate success records from optimizer and compatibility helpers and gives bypass/failure outcomes the same schema. Recording separate stage events was rejected because it would make attempt rates ambiguous and require UI-side reconstruction.

### 2. Use an allowlisted domain record, not serialized diagnostics

The record contains only stable ids/correlations, timestamps, bounded outcome/path/reason enums, measurement-quality enums, saturating counters, boolean verifier invariants, and policy/corpus versions. Raw context, generated summaries, tool data, provider errors, headers, paths, and credentials have no fields in the type.

Safe fingerprints may identify policy or fixture versions but never user content. Reusing rich-card JSON or log payloads was rejected because both are presentation/diagnostic contracts and could expand independently.

### 3. Persist a bounded SQLite ledger with best-effort writes

Native storage adds an append-only assessment table indexed by timestamp, session correlation, outcome, and policy version. Writes and opportunistic pruning execute behind a repository port. Retention supports 7, 30, and 90 days, defaults to 30, and is additionally capped at 10,000 rows; oldest rows are removed first.

Persistence failure emits a redacted unified warning but cannot change compaction or generation results. Storing the history only in chat messages was rejected because bypasses and failures have no evidence card and deleted conversations would distort policy health.

### 4. Aggregate in Rust and return typed bounded DTOs

History queries use cursor pagination and supported range values rather than arbitrary SQL-shaped filters. Summary queries return total evaluated, outcome/path/reason counts, token and character savings separately, measurement-quality coverage, policy versions, and earliest/latest timestamps. Rust performs aggregation so React never accesses SQLite or derives native metrics.

The frontend agent service gains typed history and summary operations. The Tauri adapter invokes native commands; the Web adapter maintains a deterministic capped in-memory ledger. Both use the same DTO semantics and typed safe errors.

### 5. Keep evaluation corpus content-safe and deterministic

Regression cases are code-owned structural fixtures composed from semantic classes, retention labels, protocol shapes, bounded sizes, and expected invariants rather than copied user prompts. Each case runs the active and candidate policy through the same planner/reducer/verifier pipeline with fixed versions and no provider call.

A candidate regresses if it loses required retention, breaks protocol structure, produces unsafe arithmetic, changes determinism, or turns a baseline success into failure. Savings and fallback changes are reported but cannot compensate for an invariant failure. Evaluation output is evidence only; policy activation remains an explicit reviewed code/config change.

### 6. Add a focused settings health panel

The OnePiece parameter page will compose a context-health section from service data. It shows compact summary cards, quality/path distributions, and a paginated recent-outcomes list, with range and retention controls. All labels use existing localization infrastructure and semantic Tailwind tokens; loading, empty, and error states remain accessible.

The first version does not add charts or a separate global observability destination. This keeps the UI close to the policy control while the dataset is local and OnePiece-specific.

### 7. Correct architecture documentation in the same change

The native-agent guide currently says evidence UI and persisted settings are deferred. It will be updated to describe the delivered fourth-stage behavior and identify quality evaluation and provider-native cache edits accurately. This avoids carrying a known false operational statement into the next archive.

## Risks / Trade-offs

- **[Risk] Bypass records grow faster than useful history.** → Emit one record only after the context reaches an automatic-compaction eligibility decision, then enforce time and count bounds.
- **[Risk] Structural scores are mistaken for answer quality.** → Name metrics explicitly, keep token/character qualities separate, and display a non-semantic-quality disclosure.
- **[Risk] SQLite writes add latency to generation.** → Use best-effort repository writes outside provider request construction and keep records/indexes small.
- **[Risk] Candidate evaluation diverges from production orchestration.** → Reuse the same domain planner, reducer, reinjection, and verifier entry points rather than duplicating algorithms in test utilities.
- **[Risk] Web/mock data suggests real provider behavior.** → Label mock records deterministically and never claim provider-reported token quality.
- **[Trade-off] A code-owned synthetic corpus cannot prove real-world answer quality.** → Treat it as a safety regression gate; richer opt-in evaluation can be proposed separately.

## Migration Plan

1. Add the desktop setting default and SQLite migration without changing the active compaction policy.
2. Add domain assessment/evaluation types and repository ports with isolated migration and privacy tests.
3. Wire best-effort recording and evidence correlation into the existing coordinator.
4. Add native commands, frontend types, and Tauri/Web adapters.
5. Add the settings panel, localization, component tests, E2E coverage, and documentation correction.
6. Run strict OpenSpec, frontend, Playwright, Rust, migration, privacy, and deterministic-regression validation before archive.

Rollback removes the UI and recording calls while leaving the additive SQLite table and setting value intact for forward compatibility; no existing session, message, or token-accounting data is rewritten.
