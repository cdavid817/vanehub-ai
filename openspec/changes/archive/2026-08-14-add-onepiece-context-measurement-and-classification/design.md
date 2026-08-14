## Context

See `proposal.md` for motivation. The current OnePiece API runtime builds provider-native JSON turns in `api_process_adapter.rs`, checks only their recursive character count against a fixed 60,000-character threshold, and may rebuild the turn list after summarization. Anthropic and OpenAI-compatible adapters already normalize streamed usage into `ReportedUsageTotals`, while token accounting preserves measurement quality and provider overlap semantics.

The analysis must observe the request that is actually sent, including system instructions and tool schemas that the current compaction counter omits. It must also avoid putting policy into provider adapters, avoid persisting prompt content, and remain non-mutating until a later change explicitly promotes the shadow decision.

## Goals / Non-Goals

**Goals:**

- Introduce provider-neutral context snapshot, grouping, classification, capacity, and shadow-decision domain models.
- Analyze the exact prepared provider request at both initial and tool-continuation boundaries.
- Reuse normalized provider usage as a correlation-safe measurement anchor without weakening token-accounting semantics.
- Keep tool interactions protocol-safe by treating a complete API round as the smallest policy unit.
- Produce deterministic, bounded diagnostics suitable for comparing the new decision with the current trigger.
- Isolate the new modules so later optimizer, automatic-policy, and evidence-projection changes can build on them.

**Non-Goals:**

- Changing the active character-count trigger or summary format.
- Removing, summarizing, or otherwise optimizing any context component.
- Adding automatic-compaction settings, session controls, cooldown, or failure circuit breaking.
- Persisting full context snapshots or exposing them through frontend service APIs.
- Adding a tokenizer or provider token-count endpoint dependency.
- Applying OnePiece context policy to managed CLI agents, which continue to own their internal context.

## Decisions

### 1. Analyze one prepared request rather than independently reconstructing context

The runtime will build a provider request body once and pass that same immutable body through a wire-specific projection function before sending it. The projection converts provider-native JSON into a provider-neutral sequence of `ContextComponent` and `ContextRound` values. The network path sends the unchanged prepared body.

The common projection vocabulary includes:

- component identity scoped to the request;
- semantic class;
- retention class;
- estimated Tokens and characters;
- content fingerprint;
- API-round identity and protocol-completeness state.

`WireFormat` gains a pure context-projection function beside its existing request-body and response-translation functions. Anthropic and OpenAI-compatible modules understand only their wire shape; they do not decide retention, thresholds, or compaction.

**Why:** analyzing the final body includes system instructions, tool schemas, and provider-specific wrapping while preventing drift between a separately reconstructed logical model and the actual request.

**Alternative considered:** analyze `turns`, `system`, and `tool_catalog` separately in `api_process_adapter.rs`. Rejected because OpenAI-compatible system-message insertion and wire-specific tool wrapping would be estimated differently from the payload sent to the provider.

### 2. Keep classification policy in a pure domain module

New agent-runtime domain types will define stable semantic classes such as `system-instruction`, `tool-schema`, `user-intent`, `assistant-response`, `tool-request`, `tool-result`, `attachment`, `memory`, and `unknown`. A separate retention enum defines `protected`, `verbatim`, `summarizable`, `microcompactable`, `reinjectable`, and `discardable`.

The classifier consumes the neutral projection and applies ordered, deterministic rules:

1. System, role, safety, unresolved protocol, and unknown control content is protected.
2. The current user request, user corrections, and a bounded recent-round tail remain verbatim.
3. Completed older conversational rounds are summarizable.
4. Older large or repeated tool results are microcompactable, but their containing round remains atomic.
5. State that has an authoritative current source may be reinjectable.
6. Only explicitly recognized transient content may be discardable; unknown content never defaults to discardable.

Classification produces metadata only. No content is cloned into logs or persisted records.

**Why:** separating semantic meaning from retention policy avoids making message role synonymous with disposal safety and lets later policies change retention without changing wire parsing.

**Alternative considered:** classify directly inside each provider adapter. Rejected because policy would diverge between wire formats and become difficult to test as one OnePiece behavior.

### 3. Group at assistant response/API-round boundaries and fail closed

The grouping algorithm starts a new round when a new assistant provider response begins. Tool requests emitted by that response and their following results remain in the same round. Multiple tool calls in one assistant response remain one round. The projection records tool-call identifiers only as request-scoped hashes.

Missing or duplicate tool pairs mark a round `protocol-incomplete`. Such a round is protected from summarizable, microcompactable, and discardable policy results. Grouping does not repair malformed history in this phase; existing request normalization remains responsible for any supported repair.

**Why:** a message-count or human-turn split can separate tool protocol pairs, particularly in single-prompt agentic runs. The API round is the smallest boundary that remains safe across both supported wire formats.

**Alternative considered:** keep the current fixed number of recent JSON turns. Retained for active compaction compatibility only; it is not precise enough for future optimization.

### 4. Use a request fingerprint and generation-local usage anchor

Every prepared request receives a content fingerprint derived from canonical structural metadata and cryptographic hashes of content, never raw content. After the provider response completes, a valid normalized input measurement can finalize that request's snapshot as `reported` and becomes a `ContextUsageAnchor` containing:

- request fingerprint;
- normalized input occupancy;
- provider/model identity;
- invocation sequence;
- component fingerprints and local estimates needed to recognize an append-only successor.

Before the next request in the same generation:

- an identical fingerprint can use the matching reported measurement;
- an append-only successor uses the reported anchor plus the estimated new-component delta and is labeled `reported-plus-estimated-delta`;
- a changed system prompt, tool schema, model, provider, reordered prefix, removed component, or uncorrelated observation invalidates the anchor and forces full local estimation.

The initial request of a generation normally has no generation-local anchor and is estimated. Persisted session accounting is not treated as a context anchor because its observation identifies an invocation but does not preserve enough safe structural state to prove that a future request is the same prefix.

**Why:** provider usage is authoritative for the request it measured, not automatically for a later request. Restricting anchor reuse to a provable same-generation relationship avoids false precision.

**Alternative considered:** use the latest assistant message's input usage as the next request size. Rejected because the new assistant message, tool output, system changes, and tool-catalog changes are absent from that measurement.

### 5. Use deterministic estimation without a new tokenizer dependency

The estimator will walk the projected request and calculate:

- recursive Unicode character and serialized-byte counts;
- conservative text estimates that distinguish ASCII-dominant and non-ASCII content;
- explicit structural overhead for roles, content-block envelopes, tool schemas, and provider wrapping;
- bounded estimates for supported non-text blocks.

The estimator version is included in every snapshot. If a block cannot receive a Token estimate, the snapshot degrades to `characters-only` rather than inventing Tokens. Tests use fixed fixtures and assert deterministic results and monotonicity, not equality with a provider tokenizer.

**Why:** the phase needs relative occupancy and comparison evidence, while adding multiple provider tokenizers would create dependencies and still fail for many custom OpenAI-compatible models.

**Alternative considered:** call a provider token-count endpoint. Rejected because availability and semantics vary by provider, it adds latency and cost at every request boundary, and it cannot remain provider-neutral.

### 6. Resolve model capacity from a versioned exact-match catalog

A separate embedded model-context catalog will key verified metadata by stable source provider id and exact model id. Entries contain context-window Tokens, maximum output Tokens when verified, metadata revision, and a non-secret source identifier. The runtime will not infer capacity from display names or fuzzy model-name matching.

`ApiProviderConfig` will carry the stable source provider identity when a preset or derived profile supplies it. Custom profiles without a stable source identity, discovered model ids absent from the catalog, and aliases without an exact entry remain unknown.

Catalog validation tests enforce positive bounds, unique keys, deterministic ordering, and a metadata revision. Updating capacity data is a normal reviewed catalog change.

**Why:** arbitrary OpenAI-compatible endpoints can reuse model names with different capacities. An exact, versioned catalog makes uncertainty explicit and prevents unsafe assumptions.

**Alternative considered:** assign one default context window per wire protocol. Rejected because wire format does not determine model capacity.

### 7. Define a versioned shadow policy with no runtime authority

`onepiece-context-shadow-v1` calculates a hypothetical threshold only when capacity and a Token measurement are available:

```text
summary reserve = min(verified maximum output, 20,000)
safety buffer   = min(13,000, 10% of total context window)
shadow threshold = total context window - summary reserve - safety buffer
```

The policy returns a decision and reason code such as `below-threshold`, `at-or-above-threshold`, `insufficient-capacity-metadata`, or `characters-only-measurement`. The existing recursive character count is evaluated separately. Both outcomes are passed to diagnostics, while only the existing character decision controls `maybe_compact_accounted`.

The policy and estimator versions are constants in the snapshot so later evidence can distinguish results produced by different rules.

**Why:** a concrete shadow policy is necessary to gather disagreement evidence. Keeping it non-authoritative allows thresholds to be revised before the automatic-policy phase.

**Alternative considered:** log measurements without a decision. Rejected because it would not reveal whether the proposed model-aware trigger materially differs from the existing trigger.

### 8. Keep snapshots ephemeral and diagnostics content-free

The complete snapshot lives only for the generation. Unified logging receives a bounded projection containing correlation ids, request sequence, provider/model safe ids, measurement quality, counts, aggregate estimated Tokens/characters, capacity values, class totals, policy versions, active/shadow decisions, disagreement reason, and component hashes only when needed for diagnosis.

No new SQLite table, frontend command, frontend service method, or Web/mock simulation is added. Logs use the existing native logging service and redaction path. Individual component lists are capped; overflow is represented by counts.

**Why:** phase 4 will define durable evidence and user-facing projection. Persisting an interim internal shape now would create a migration burden and risk retaining sensitive content.

**Alternative considered:** persist every context component for later inspection. Rejected because it duplicates transcript content, increases privacy risk, and conflicts with token-accounting's safe persistence boundary.

### 9. Integrate through a small application service, not the process adapter

The implementation will add focused modules rather than expanding `api_process_adapter.rs` with classification and measurement logic:

- domain: snapshot, component, round, classification, capacity, decision, and version types;
- application: `ContextAnalysisService` orchestrating projection, estimation, anchor reconciliation, classification, and shadow comparison;
- infrastructure: wire projections, embedded model-context catalog, deterministic estimator, and unified-log projection;
- adapter integration: request preparation calls the service and threads a generation-local anchor across loop iterations.

The service returns the original request body together with analysis metadata; it has no method capable of editing the body.

**Why:** the current adapter already coordinates networking, tools, memory extraction, accounting, and compaction. A dedicated service makes the non-mutation guarantee reviewable and gives later phases a stable seam.

## Risks / Trade-offs

- **[Risk] Local Token estimates differ significantly from some provider tokenizers.** → Preserve measurement quality, use conservative multilingual fixtures, compare against reported anchors, and keep the decision in shadow mode.
- **[Risk] Request projection and grouping accidentally omit provider-native content.** → Analyze the final prepared body, maintain recorded fixtures for both wire formats, and verify projected aggregate characters against a recursive whole-body walk.
- **[Risk] A reported anchor is applied to a structurally changed request.** → Require provider/model equality, prefix fingerprints, append-only component ordering, and same-generation invocation sequencing; otherwise fall back to full estimation.
- **[Risk] Model context metadata becomes stale.** → Use an exact-match versioned catalog with reviewed source metadata and treat unmatched models as unknown.
- **[Risk] Classification exposes sensitive content through diagnostics.** → Persist only bounded aggregates, reason codes, stable correlations, and hashes through unified logging.
- **[Trade-off] Initial requests are usually estimated even when prior usage exists.** → Accept lower quality until a future durable safe-anchor design exists; do not trade correctness for apparent precision.
- **[Trade-off] Web/mock gains no equivalent analysis in this phase.** → No frontend contract or user-visible behavior changes, so parity is deferred to the evidence-projection phase.

## Migration Plan

1. Add domain types, estimator fixtures, wire projection fixtures, and model-catalog validation without invoking the analyzer.
2. Integrate snapshot construction before provider sends, but retain only test-visible results.
3. Enable bounded unified-log shadow diagnostics while leaving `maybe_compact_accounted` unchanged.
4. Compare active and shadow outcomes in automated fixtures and manual long/tool-heavy sessions.
5. Roll back by removing the analyzer invocation; the existing compaction path and persisted data require no migration because snapshots are ephemeral.

## Open Questions

- Exact verified model-capacity entries will be populated from official provider metadata during implementation. Missing or ambiguous models safely remain unknown and do not change the architecture or task breakdown.
