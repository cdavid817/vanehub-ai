## Context

See `proposal.md` for motivation and the delta specifications for behavior. VaneHub already has correlated execution runs, metadata privacy, native API tool events, managed and interactive CLI lifecycle events, Plan verification evidence, message persistence, Skill usage and effective revisions, delegated Utility attempts, and unified logging. Those capabilities remain authoritative for their own data.

The evidence pipeline must consume safe structured projections without copying raw runtime stores or becoming part of the execution critical path. Its output is not a Skill change: a signal states that an observable outcome occurred, and a candidate seed states that compatible evidence forms a pattern worth later review. Causality, target selection, quality judgment, and mutation belong to later changes.

## Goals / Non-Goals

**Goals:**

- Capture improvement evidence from every VaneHub-managed Agent family at the fidelity actually available.
- Preserve exact native Skill revision participation and honest CLI uncertainty.
- Sanitize all bounded free text before persistence or fingerprinting.
- Produce deterministic, reproducible signal and seed lineage without model calls.
- Keep ingestion asynchronous, bounded, and fail-open relative to Agent work.
- Provide users with evidence inspection, privacy, retention, and purge control.
- Establish stable evidence contracts for later target selection and review.

**Non-Goals:**

- Determining that a participating Skill caused an outcome.
- Choosing the Skill to modify or deciding whether evidence is good enough.
- Calling an LLM, generating instructions, creating Overlay mutations, or applying changes.
- Scraping raw unified log files, terminal scrollback, provider transcripts, or source files.
- Capturing every possible user statement as implicit correction feedback.
- Exporting or synchronizing evidence to a remote service.
- Replacing existing execution, message, permission, usage, or log retention.

## Decisions

### 1. Add a dedicated evidence context fed by structured projections

The Rust native layer adds a `skill_evolution_evidence` context with domain, application, infrastructure, and API boundaries. Runtime producers depend only on a narrow `EvidenceEnvelopeSink` port:

```text
execution / chat / plan / delegation / CLI adapters
                 │
                 ▼
       bounded in-memory priority queue
                 │
                 ▼
       evidence ingestion worker
       ├─ envelope validation
       ├─ sanitization
       ├─ attribution
       ├─ six extractors
       ├─ signal transaction
       └─ seed rebuild scheduling
```

Producers never call the evidence repository synchronously. Enqueue has a small fixed time budget and returns accepted or dropped; neither result changes the source operation. Explicit user feedback is the exception: the feedback command waits for its evidence transaction because the user must know whether feedback was saved, but it still cannot change the completed message.

The pipeline does not subscribe to or parse unified log files. Producers project registered safe event shapes directly from their authoritative application boundaries. Unified logging receives only pipeline operational diagnostics after evidence sanitization.

Alternatives considered:

- Periodically mine SQLite messages, logs, and traces. Rejected because it duplicates raw content access, creates ambiguous versioning, and weakens source-time Skill attribution.
- Run extractors inline with every Agent event. Rejected because evidence storage or sanitizer failures could delay execution.
- Reuse the observability database as the evidence store. Rejected because evidence has different retention, lineage, feedback, purge, and future governance semantics.

### 2. Use a closed versioned source-envelope registry

`EvidenceSourceEnvelope` is a tagged versioned enum. Common fields are:

```text
source_event_id
source_kind / schema_version
occurred_at
stable_agent_id?
session_id? / message_id? / run_id? / attempt_id?
canonical_workspace?
fidelity
terminal_classification?
verification_classification?
safe_counts / durations / enum fields
observed_skill_revisions[]
bounded_summary_candidate?
```

Registered variants cover explicit feedback, generation terminal, tool terminal, permission terminal, provider/process terminal, verification terminal, retry linkage, Utility terminal, Skill load/refusal, prompt-budget omission, dependency/conflict state, and CLI mount snapshot. The registry defines which fields are legal and whether a bounded summary candidate may exist.

Unsupported variants or versions are dropped with a rate-limited safe diagnostic. Producers cannot attach arbitrary JSON or raw output. Source ids come from authoritative records where possible; otherwise the producer creates a stable id before first delivery.

The ingestion repository stores a compact receipt keyed by `(source_event_id, extractor_id, extractor_version, discriminator)` rather than storing the original envelope body.

Alternatives considered:

- Accept generic log-like key/value envelopes. Rejected because arbitrary fields inevitably become a content exfiltration path.
- Store every envelope for future reprocessing. Rejected because it would replicate potentially sensitive source data and defeat bounded evidence storage.

### 3. Implement exactly six pure extractor families

Each extractor is a deterministic pure component returning zero or more `SignalDraft`s:

1. `explicit-feedback`: helpful, unhelpful, corrected, and feedback replacement.
2. `execution-failure`: provider, process, tool, permission, timeout, limit, sandbox, and terminal classifications.
3. `verification-outcome`: test, build, lint, type, security, specification, acceptance, and Plan verification.
4. `retry-recovery`: links failed and later attempts sharing a sanitized task fingerprint; distinguishes repeated failure from recovery.
5. `delegation-outcome`: Utility start and terminal status, effective revision, limits, tool and approval counts.
6. `skill-lifecycle-anomaly`: repeated load refusal, dependency unavailability, Overlay conflict, prompt-budget omission, or use without correlated success after fixed thresholds.

Extractor versions are constants included in signals and receipts. A new version may reprocess only source references still available through bounded authoritative queries; it creates supersession lineage and does not silently overwrite prior signals.

Cancellation is neutral unless another structured fact makes it negative. Denied permission is classified as a permission outcome, not automatically as a Skill defect. Successful outcomes produce positive evidence useful for recovery and regression comparison.

Alternatives considered:

- Use language models to interpret every failure. Rejected because this layer must be deterministic, cheap, private, and reproducible.
- Infer correction from phrases in ordinary user messages. Rejected because natural conversation is too ambiguous; users get an explicit feedback action.
- Treat every non-zero process exit as Skill failure. Rejected because environment, dependency, permission, and user cancellation classifications must remain distinct.

### 4. Separate participation attribution from causality

The attribution engine consumes only `observed_skill_revisions` captured by the source runtime:

```text
Verified
  exact eager prompt inclusion
  successful load_skill revision
  started Utility delegation revision

Correlated
  effective CLI mount snapshot captured for the same run,
  without proof which mounted Skill affected output

Weak
  configured binding or stale association without an active snapshot

Unattributed
  no observable Skill participation
```

One event may have multiple verified associations because several Skills participated in one native generation. “Verified” means participation is verified, not that the Skill caused success or failure. Each association stores rationale, association kind, observation time, and revision hash.

Targeting eligibility is projected conservatively:

- verified: later automatic target consideration is permitted but not decided;
- correlated: later human review may consider the hints;
- weak/unattributed: no target identity is emitted in a seed.

CLI adapters capture canonical mount snapshots when the process starts. An interactive terminal whose internal behavior is opaque cannot upgrade correlation to verification. Hook or provider-specific structured events may improve fidelity only when they name a specific active Skill revision under a validated adapter contract.

Alternatives considered:

- Attribute every CLI outcome to all configured bindings. Rejected because configuration does not prove active use.
- Pick the only verified Skill immediately. Rejected because target selection still needs category, evidence, and later review even with one participant.
- Drop unattributed evidence. Rejected because generic runtime patterns and user feedback remain useful for memory or human investigation.

### 5. Sanitize before every derivative operation

The evidence sanitizer is independent from but compatible with unified-log redaction. It runs before free text reaches signal construction, hashing, diagnostics, persistence, seed grouping, queries, or future export. Its twelve ordered classes are:

1. private-key blocks;
2. API, access, refresh, and session tokens;
3. authorization headers and cookies;
4. password, secret, and credential assignments;
5. credential-bearing URLs;
6. credential-bearing connection strings;
7. secret environment-variable values;
8. user-home and profile paths;
9. email addresses;
10. phone numbers;
11. IP addresses and internal hostnames;
12. cloud, tenant, subscription, account, and project identifiers.

Rules operate on strict bounded input and return sanitized text, class counts, sanitizer version, and whether input was rejected. Private-key blocks and over-limit correction notes are rejected or omitted entirely rather than partially retained.

Markers use keyed HMAC tokens with an installation-local secret stored behind the native credential/settings boundary:

```text
<redacted:email:7F3A>
```

Only a short token is persisted. It supports local duplicate correlation but is neither reversible nor comparable across installations. HMAC input is normalized within the class, and raw values are not logged. Task and dedupe fingerprints are computed after sanitization with a separate installation-scoped key and workspace scope.

Alternatives considered:

- Reuse unsalted SHA hashes. Rejected because common emails, paths, and tokens can be dictionary attacked and compared across devices.
- Sanitize only when data is displayed. Rejected because sensitive values would already exist in SQLite, hashes, and diagnostics.
- Store encrypted raw evidence for future models. Rejected because raw retention increases breach scope and is unnecessary for this deterministic phase.

### 6. Default to metadata-only, with bounded explicit feedback

Most signals use template summaries derived from enums and counts, for example “content search failed with sandbox classification in 3 attempts.” They do not include raw error text. If the existing observability redacted-summary mode is explicitly enabled, registered envelope variants may provide a bounded summary candidate that still passes evidence sanitization.

Explicit correction feedback is user-authorized free text with a 1,000-character hard limit and a privacy warning. Helpful and unhelpful feedback need no note. The sanitizer processes the note before transaction commit. The UI may keep unsaved raw input in component state after failure, but native evidence tables and unified logs never receive it.

Evidence source references are opaque internal ids pointing to authoritative runtime records. Query APIs do not dereference those records into content; authorized existing timeline/message views remain separate.

Alternatives considered:

- Copy bounded stderr snippets by default. Rejected because bounded does not mean non-sensitive and source diagnostics already have their own controls.
- Require correction text for all negative feedback. Rejected because it discourages explicit feedback and creates unnecessary content storage.

### 7. Persist normalized signals and immutable lineage in SQLite

The evidence context adds additive tables:

```text
evolution_signal_receipts
evolution_signals
evolution_signal_skill_associations
evolution_signal_source_links
evolution_candidate_seeds
evolution_candidate_seed_signals
evolution_feedback_current
evolution_feedback_events
evolution_pipeline_state
```

Signals are immutable except for retention deletion and an explicit supersession link. Feedback replacement creates a feedback event, updates one current state, supersedes the prior active signal, and schedules seed rebuild. Seeds are versioned deterministic projections over retained active signals; their contributing links are immutable for that version.

SQLite uniqueness constraints enforce receipt idempotency and one current feedback state per message. Signal insertion, associations, receipts, feedback transition, and seed-dirty marker share one transaction. Seed rebuild uses a state revision witness so concurrent ingestion cannot publish lineage from a stale signal set.

No table contains raw prompt, transcript, hidden reasoning, command, tool result, terminal output, file body, credential, or absolute path. Safe summaries are bounded to 1,000 characters; source and association counts are capped.

Alternatives considered:

- Store evidence as JSONL next to Skills. Rejected because multi-dimensional queries, lineage, replacement, retention, and transactional purge need indexed relations.
- Mutate one aggregate row per Skill. Rejected because later review must inspect immutable contributing evidence and versions.

### 8. Build task fingerprints after sanitization

The fingerprint builder combines canonical workspace hash, registered source kind, coarse operation class, safe verifier or tool category, sanitized bounded user feedback when explicitly supplied, and stable task identity already provided by a Plan or delegation. It does not include raw task text.

Normalization lowercases stable enums, normalizes safe whitespace in authorized summaries, sorts set-valued ids, and versions the algorithm. A keyed installation-local HMAC produces the fingerprint. Similar activity in different workspaces does not group by default.

Retry and recovery sources with authoritative retry or predecessor ids use those links first. Fingerprints are a fallback correlation mechanism, not proof that two operations are identical.

Alternatives considered:

- Hash raw user prompts. Rejected because hashes can leak low-entropy prompts and privacy rules require sanitization first.
- Use semantic embeddings. Rejected because they require model execution, store content-derived vectors, and are not deterministic enough for this layer.

### 9. Construct candidate seeds with deterministic grouping rules

The seed builder runs after committed signals and periodically for dirty grouping keys. A key includes:

- workspace scope;
- signal category family;
- task fingerprint version and value;
- compatible Skill association cohort;
- evidence-strength class;
- rolling 14-day pattern window.

Default readiness rules are:

- one explicit corrected feedback signal with verified association may form a single-source seed;
- otherwise at least two non-duplicate signals from distinct runs are required;
- positive recovery and preceding negative evidence can form one mixed-polarity seed;
- neutral or weak isolated anomalies remain signals only;
- version cohorts do not merge silently across incompatible effective Skill revisions.

The builder emits no prose proposal. Its bounded template summary states category, counts, polarity, time span, source distribution, attribution distribution, and recovery presence. It exposes verified target hints only when present in contributing signals and human-only hints for correlated cohorts. It never chooses one final target.

Rerunning the same builder version over the same active signals produces the same grouping hash and no duplicate seed. Signal supersession, purge, or expiry marks affected groups dirty and rebuilds or removes their seeds transactionally.

Alternatives considered:

- Create one seed per failure. Rejected because transient failures would overwhelm later review.
- Wait for a nightly LLM summary. Rejected because deterministic seeds are needed before target selection and lifecycle scheduling.
- Merge across all projects. Rejected because project guidance and environment failures are often not generalizable.

### 10. Bound retention, storage, and backpressure by evidence value

First-version retention is 90 days for signals, feedback evidence projections, receipts needed for active idempotency, and seeds. A daily native maintenance job expires data and rebuilds affected seeds. It does not delete source messages, traces, permission audits, logs, usage, Skills, or Overlays.

Default quotas are centralized constants:

```text
ingestion queue                  512 envelopes
signals per workspace            10,000
seeds per workspace              2,000
global evidence database budget  64 MiB
signals per seed                 100
Skill associations per signal   32
source links per signal          16
```

Priority order under queue or storage pressure is:

1. explicit corrected feedback and verified recovery/verification;
2. verified failures and delegated outcomes;
3. correlated CLI evidence;
4. helpful feedback and neutral lifecycle signals;
5. weak anomalies.

The queue uses bounded priority lanes and drop counters, not blocking eviction from producer threads. Storage maintenance deletes expired data first, then lowest-priority oldest evidence while preserving referential integrity. Every drop/expiry class is counted and visible; diagnostics are rate-limited.

Alternatives considered:

- Unlimited local evidence because data is metadata-only. Rejected because high-volume tools and long-running CLI sessions can still create unbounded records.
- Block Agent execution until evidence persists. Rejected because evolution is secondary to user work.
- Keep seeds after deleting their signals. Rejected because lineage would become unauditable.

### 11. Provide transactional scoped purge

Purge criteria compile into an evidence-only deletion plan by global scope, canonical workspace, canonical Skill id, stable source Agent id, time range, and evidence kind. The plan identifies signals, receipts, current feedback projections, feedback events, seed links, and seeds that must be rebuilt or deleted.

Purge runs in one SQLite transaction per bounded batch with a stable operation id and progress. On failure, each batch is all-or-nothing; the UI can retry. Purging correction evidence does not edit the source message. Purging by Skill association removes the affected signals rather than editing an immutable signal into a misleading unattributed form.

The operation writes only safe counts and scope hashes to unified logging. There is no undo because privacy deletion should be definitive; the confirmation copy names data that remains outside the evidence context.

Alternatives considered:

- Remove only the Skill association and retain the signal. Rejected because the user asked to purge that Skill's evidence and partial retention complicates lineage.
- Cascade into execution logs and messages. Rejected because those capabilities have independent retention and deletion contracts.

### 12. Add explicit feedback through the existing chat boundary

Completed assistant messages gain one current feedback state: helpful, unhelpful, or corrected. The frontend sends message id, expected feedback revision, selected state, and optional correction note through `agent-service.ts`. The native command validates session access and message terminal state, sanitizes the note, commits the feedback transition and signal, and returns the saved sanitized projection and revision.

Changing feedback uses compare-and-swap. A stale response keeps the UI input and asks for reload. The message content and status never change. Web/mock mode simulates the same revision, sanitization result, failure, and replacement transitions without native storage.

The controls remain compact on the message and use a dialog only for optional correction. Privacy copy warns users not to paste secrets even though the sanitizer remains mandatory.

Alternatives considered:

- Infer negative feedback when the user sends another message. Rejected because follow-up conversation is not reliably a correction.
- Store feedback as a message edit. Rejected because it would corrupt conversation history and mix user evidence with assistant output.

### 13. Extend execution projection for native and CLI sources honestly

Native API prompt assembly records exact eager revisions; `load_skill` records exact loaded revisions; Utility delegation records its exact revision. These become verified participation associations on the exact generation or attempt.

Managed CLI launch captures an effective Skill mount manifest hash and bounded canonical revision list at process start. Outcomes from that same process can be correlated to the list, but not to one causal Skill. Interactive CLI terminals capture the same snapshot if VaneHub owns the mount boundary; internal TUI steps remain opaque. A configured binding with no captured mount is weak only.

Plan verification envelopes carry PlanRun, SubTask attempt, verifier class, outcome, and exact native or correlated CLI associations inherited from the attempt. Retry lineage uses durable attempt relationships rather than matching text when available.

Adapters do not scrape CLI session content for evidence. Future CLI-specific structured Skill-use hooks can register a new envelope version and improve attribution after separate validation.

Alternatives considered:

- Exclude CLI evidence entirely. Rejected because correlated verification and failure patterns are useful when uncertainty is explicit.
- Label active mounts as verified usage. Rejected because a mounted instruction may never have influenced the CLI result.

### 14. Expose evidence through typed read-only service contracts

Rust queries return collection status, funnel counts, distributions, sanitized signals, seed lineage, retention/quota state, and purge previews/results with cursor pagination. Scope checks use canonical workspace and Skill identity. Unattributed evidence has a separate generic pool and never gains a fabricated Skill id.

`agent-service.ts` owns shared TypeScript models. `tauri-agent-client.ts` is the only frontend native invocation and event boundary. `web-agent-client.ts` simulates healthy, empty, correlated CLI, weak, degraded, quota, feedback, lineage, retention, and purge states.

The per-Skill Evolution area is explicitly evidence-only. It presents:

- collection and degradation status;
- runtime events → signals → grouped patterns → seeds funnel;
- extractor, attribution, source Agent, category, polarity, and severity distributions;
- sanitized signal and seed lineage inspection;
- sanitizer version, twelve classes, retention, quota, dropped and expired counts;
- scoped purge.

It does not show approve, apply, Overlay diff, target selection, or automatic evolution controls. Those arrive with later capabilities. React components use service models only, Tailwind, localization, existing accessibility patterns, and focused modules below 300 lines.

Alternatives considered:

- Hide evidence until Curator exists. Rejected because users need to verify privacy and attribution before any mutation system is introduced.
- Show raw observability timelines inside Skill settings. Rejected because evidence queries are intentionally sanitized, scoped, and lineage-focused.

## Risks / Trade-offs

- [Deterministic extractors miss nuanced lessons] → Preserve safe lineage and add later LLM review only after privacy, attribution, and quality gates exist.
- [Verified participation is mistaken for causality] → Label it explicitly as participation, preserve all participating revisions, and defer target choice.
- [CLI evidence is noisy] → Keep correlated, weak, and unattributed classes separate and prohibit weak evidence from target hints.
- [Redaction damages useful summaries] → Prefer structured enum/count templates and explicit bounded correction text; display sanitizer classes so users understand omissions.
- [Pattern rules over-redact source-code-like identifiers] → Apply class-aware detectors to bounded registered fields, retain safe category metadata, and version the sanitizer for regression tests.
- [Evidence queue drops high-value events] → Use priority lanes, reserve capacity for explicit feedback and verified recovery, and expose drop counts.
- [Seed proliferation creates later review load] → Require independent-run thresholds, deduplicate deterministically, cap seeds, and keep isolated events below readiness.
- [Changing feedback leaves inconsistent seeds] → Use CAS, supersession, dirty-group rebuild, and one transaction for feedback and signal state.
- [Purge appears to delete all related user data] → Name the evidence-only boundary clearly and enumerate source records that remain.
- [A pipeline bug affects user work] → Keep runtime ingestion asynchronous and fail-open, with only explicit feedback reporting its own save failure.

## Migration Plan

1. Complete the effective Skill runtime prerequisite and verify revision observations and usage events are stable.
2. Add evidence domain enums, source-envelope registry, sanitizer, attribution, six pure extractors, and deterministic tests without enabling producers.
3. Add SQLite schema, repositories, idempotency receipts, feedback transitions, seed builder, retention, quotas, purge, and recovery tests.
4. Add the bounded priority queue and ingestion worker behind a disabled feature flag; verify producer failures and queue pressure never affect source execution.
5. Project native API, Skill load, Utility delegation, Plan verification, managed CLI, and interactive CLI facts in increasing fidelity order.
6. Enable metadata-only ingestion, compare expected counts against observability fixtures, and audit that prohibited content never reaches evidence tables or logs.
7. Add explicit chat feedback, CAS replacement, service contracts, Tauri adapter, and Web/mock behavior.
8. Add evidence queries, per-Skill Evolution UI, privacy/retention details, scoped purge, localization, accessibility, and E2E tests.
9. Run privacy corpus, attribution, deduplication, seed reproducibility, quota, retention, purge, frontend, Rust, contracts, docs, and strict OpenSpec validation before enabling collection by default.

Rollback disables source projection and stops the evidence worker before removing UI entry points. Existing evidence tables remain additive and can be purged by the user or ignored by older binaries; rollback does not merge evidence into logs, messages, Skills, usage, or Overlays. Any queued envelopes are dropped safely. Re-enabling validates schema and sanitizer versions, runs retention, and resumes with idempotency receipts so already processed source events do not duplicate.

