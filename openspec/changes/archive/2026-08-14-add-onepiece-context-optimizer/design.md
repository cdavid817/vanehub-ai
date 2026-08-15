## Context

See `proposal.md` for motivation. Phase one added a provider-neutral snapshot, API-round grouping, retention classification, deterministic estimates, exact-match capacity, usage anchors, and shadow evidence. The active compaction implementation still slices native `turns`, summarizes everything before the six-turn tail, and replaces it with one synthetic turn.

The optimizer must work for Anthropic Messages and OpenAI-compatible Chat Completions without moving policy into wire adapters. It must preserve `maybe_compact_accounted` as the only active trigger, keep summary calls tool-free, use unified logging, and remain best-effort so an optimizer defect cannot fail a generation. The Web/mock runtime and managed CLI Agents remain outside this native execution path.

Claude Code provides useful reference patterns: microcompact old tool results before full summarization, group by API round, prohibit tools in the compact call, use a structured continuation prompt, rebuild current authoritative attachments after compaction, measure pre/post occupancy, and reject or retry unusable summaries. This design adapts those patterns to OnePiece's existing ports and two wire formats rather than copying provider-specific implementation details.

## Goals / Non-Goals

**Goals:**

- Produce a deterministic, reviewable optimization plan from phase-one snapshot metadata.
- Reclaim low-value context before spending a provider call on summarization.
- Preserve complete tool protocol, protected controls, current intent, corrections, and recent continuity.
- Generate a structured continuation summary with explicit evidence-preservation sections.
- Reconstruct both wire formats from one verified provider-neutral candidate.
- Verify measurable reduction and fail back to the existing summary-only implementation.

**Non-Goals:**

- Promoting the token-aware shadow decision to the active trigger.
- Adding automatic-compaction suppression, cooldown, failure circuit breakers, or user settings.
- Exposing snapshots, plans, or evidence in the UI.
- Persisting raw snapshots or optimization candidates in SQLite.
- Calling provider-native cache-edit APIs; the first optimizer remains portable across compatible endpoints.
- Optimizing contexts owned internally by managed CLI Agents.

## Decisions

### 1. Separate plan, execute, and verify phases

The application layer will expose an optimizer service with three explicit products: an immutable `ContextOptimizationPlan`, an in-memory candidate, and a `ContextOptimizationVerification`. The plan references request-scoped component and round identities plus safe fingerprints; it does not own raw content. Infrastructure executes actions against the already prepared wire body, while the domain verifier compares the candidate projection with the original snapshot.

The original body and turns remain untouched until verification succeeds. A rejected candidate is dropped and the current summary-only function receives the original turns.

**Alternative considered:** mutate `turns` while selecting actions. Rejected because partial failure could leave no trustworthy fallback input and makes invariant testing difficult.

### 2. Optimize in ascending semantic cost

`onepiece-context-optimizer-v1` uses this stable order:

1. Remove only explicitly recognized discardable transient components.
2. Remove stale copies of reinjectable state when an authoritative replacement is available.
3. Replace eligible old tool results with bounded protocol-valid placeholders.
4. Summarize the oldest eligible complete API rounds in one contiguous prefix.
5. Reinject bounded authoritative state.

The planner stops when projected occupancy reaches the target. It never chooses a protected, verbatim, unknown, or protocol-incomplete unit. Summary selection is a contiguous old prefix so temporal ordering remains intelligible and request reconstruction is deterministic.

**Alternative considered:** rank every component by estimated Tokens and greedily remove the largest. Rejected because it can split protocol, destroy chronology, and optimize size at the expense of task continuity.

### 3. Keep tool microcompaction portable and deterministic

The first implementation replaces selected tool result content locally with a bounded marker containing no raw output, while retaining the original tool reference, success/failure state, source fingerprint, and round membership. Wire adapters translate this neutral replacement into their native result shape.

Eligibility requires a complete older round and either large or duplicate output. The containing round remains in place; only the result payload changes. Recent and incomplete results are never modified.

**Alternative considered:** Anthropic cache edits modeled after Claude Code. Deferred because arbitrary Anthropic-compatible and OpenAI-compatible endpoints do not share cache-edit semantics, and phase two needs one portable correctness model.

### 4. Use a strict structured continuation summary

The summary prompt is versioned and requests these sections: primary request, technical constraints, decisions, files and code areas, errors and fixes, completed work, pending work, and immediate next action. It asks the model to preserve identifiers and exact user constraints when necessary, but diagnostics never record the resulting text. The summarizer receives only selected rounds plus bounded instructions and has no declared tools.

Output is accepted only when every section marker is present, content is non-empty, and output fits the configured character and estimated-Token bounds. Phase two uses one initial summary call; prompt-too-long retry and automatic failure circuit breaking belong with phase-three recovery policy unless the provider immediately rejects the summary input, in which case compatibility fallback runs.

**Alternative considered:** retain the current unconstrained summary string. Rejected because downstream continuity and completeness cannot be validated consistently.

### 5. Reinject from authoritative application ports

Reinjectable state is resolved through focused application ports rather than copied from history. Initial supported kinds are current memory/context sections and other state already available through runtime ports; each kind has a per-item and aggregate budget. Every reinjection carries only source kind, safe revision/fingerprint, and size in plan/evidence metadata.

If the authoritative source is unavailable, the planner preserves the historical component when possible. If it was already removed, verification fails and compatibility fallback uses the original turns.

**Alternative considered:** trust the latest historical copy. Rejected because retention class `reinjectable` specifically means a current source is authoritative.

### 6. Verify a reprojected candidate before use

Each wire adapter projects the candidate through the same phase-one projection used for measurement. The pure verifier enforces:

- all protected and verbatim fingerprints occur in order and unchanged;
- every request/result relationship is complete and unique;
- optimization actions match the plan and do not affect unselected components;
- required reinjection fingerprints and summary boundary metadata are present;
- complete recursive character coverage still holds;
- estimated Tokens decrease when both estimates exist, otherwise characters decrease;
- the candidate meets the target when safely possible and never exceeds the original occupancy.

Verification returns bounded reason codes. It does not attempt to repair a candidate.

**Alternative considered:** trust individual transformation tests. Rejected because composed passes and two wire reconstructions can violate invariants even when each pass is locally correct.

### 7. Integrate only inside the existing trigger

`maybe_compact_accounted` remains the sole authority. Once its current character predicate is true, it invokes optimizer-first compaction. Accepted candidates replace the turns and emit the existing visible notice. Rejected candidates call the preserved summary-only implementation. Token accounting continues to record any summary provider invocation as `ContextCompaction`; zero-call microcompaction creates no model invocation.

**Alternative considered:** use phase-one shadow threshold to invoke the optimizer. Rejected because trigger promotion, suppression, and failure circuit breaking are explicitly phase three.

### 8. Emit bounded before/after evidence only

Unified logging records plan/verifier versions, action counts by kind and class, original/candidate measurement qualities, estimated occupancy, safe fingerprints, invariant flags, fallback stage/reason, and correlation ids. No raw request, summary, placeholder source content, tool data, or provider payload is logged. The full plan and candidate are generation-local and are not persisted.

## Risks / Trade-offs

- **[Risk] A transformation creates valid JSON but invalid tool protocol.** → Reproject and verify request/result identifiers and complete rounds before replacement.
- **[Risk] Structured summary passes shape checks but omits an important fact.** → Preserve protected/verbatim content, require evidence-oriented sections, retain safe source fingerprints, and fall back when required sections are absent.
- **[Risk] Local microcompaction breaks prompt-cache reuse.** → Restrict it to old eligible results and measure savings; provider-native cache edits remain a future optimization.
- **[Risk] Optimization costs more Tokens than it saves.** → Apply zero-call passes first, require measured candidate reduction, and account for every summary invocation separately.
- **[Risk] Reinjected state is stale or unavailable.** → Require authoritative source revision and preserve history or reject the candidate.
- **[Risk] Provider-specific reconstruction diverges.** → Maintain cross-wire fixtures and reproject both candidates through the common verifier.
- **[Trade-off] Compatibility fallback may make a second summary call after an optimizer summary failure.** → Preserve generation success in phase two; phase three will add cooldown and circuit-breaker policy.

## Migration Plan

1. Add pure optimizer plan, action, budget, summary-schema, and verification domain tests without runtime integration.
2. Add provider-neutral candidate execution and cross-wire reconstruction fixtures.
3. Integrate optimizer-first execution behind the existing character trigger while retaining the current summary function as fallback.
4. Add bounded diagnostics and accounting assertions for zero-call and summary paths.
5. Roll back by bypassing optimizer-first execution; no persisted data or frontend contract requires migration.
