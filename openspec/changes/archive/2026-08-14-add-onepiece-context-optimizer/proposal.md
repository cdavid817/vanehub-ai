## Why

OnePiece can now measure and classify its complete provider context, but the active compaction path still reduces all older turns through one undifferentiated summary request. The second phase needs a protocol-safe optimizer that chooses the least destructive reduction for each classified API round, preserves actionable evidence, and proves that an optimized request remains valid before it replaces the original context.

## What Changes

- Add a provider-neutral optimization plan that consumes the phase-one snapshot and selects protected, verbatim, summarizable, microcompactable, reinjectable, and explicitly discardable actions without splitting API rounds.
- Add a deterministic low-cost pass for old tool results and reinjectable state before invoking a summary model.
- Replace the current free-form summary with a bounded structured continuation summary covering user intent, decisions, constraints, files and code, errors and fixes, completed work, pending work, and the next action.
- Add a budget allocator and post-optimization verifier that enforce target occupancy, protocol completeness, protected-content preservation, and measurable reduction.
- Integrate the optimizer behind the existing 60,000-character trigger; this phase changes how an already-triggered compaction is performed, but does not add token-aware automatic triggering or suppression controls.
- Fall back to the existing summary-only compaction path whenever planning, microcompaction, summarization, reinjection, or verification cannot produce a safe candidate.
- Emit bounded unified-log evidence for selected actions, before/after estimates, fallback reasons, and invariant results without recording context content.

## Capabilities

### New Capabilities

- `agent-context-optimization`: Defines optimization planning, ordered reduction passes, structured summaries, budget enforcement, protocol-safety verification, and content-free optimization evidence.

### Modified Capabilities

- `agent-context-compaction`: Changes an already-triggered OnePiece compaction from summary-only replacement to optimizer-first compaction with a compatibility fallback, while retaining the existing active trigger.

## Impact

- Desktop/native runtime only: agent-runtime domain, application service, Anthropic and OpenAI-compatible context reconstruction, compaction execution, and unified diagnostics.
- No frontend service contract, Tauri command, Web/mock behavior, SQLite schema, or managed CLI Agent behavior changes.
- Reuses the phase-one snapshot, retention classification, exact-match capacity metadata, and usage estimates; adds no tokenizer or provider endpoint dependency.
- Preserves the existing frontend/backend isolation and keeps provider-specific wire reconstruction in infrastructure adapters.
