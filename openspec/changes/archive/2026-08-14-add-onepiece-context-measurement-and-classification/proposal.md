## Why

OnePiece currently decides when to compact from a fixed character-count threshold and treats the provider request as an undifferentiated list of JSON turns. This makes the decision insensitive to model context windows, provider-reported usage, tool-call boundaries, and the different retention value of instructions, user intent, tool results, and transient output.

Before introducing a more aggressive optimizer or user-controlled automatic compaction, OnePiece needs a provider-neutral context snapshot that measures occupancy with explicit quality, groups protocol-safe API rounds, and classifies content by semantic and retention policy. Building this foundation in shadow mode lets the project compare it with the existing trigger without changing live compaction behavior prematurely.

## What Changes

- Add a native OnePiece context-analysis pipeline that builds a bounded snapshot before each provider request, including the initial history request and tool-continuation requests.
- Measure context using the best available provider-reported input usage plus an estimated delta for content added since that observation, with explicit `reported`, `reported-plus-estimated-delta`, `estimated`, or `characters-only` quality.
- Group messages into protocol-safe API rounds so a tool call and its result are never classified as independently removable units.
- Classify context into stable semantic categories and retention classes such as protected, verbatim, summarizable, microcompactable, reinjectable, and discardable.
- Resolve the active model's context-window metadata and compute occupied, reserved, and remaining capacity when the model is known, while representing unknown capacity without fabricating a percentage.
- Run the new decision calculation in shadow mode alongside the existing character-count compaction trigger and emit only bounded, redacted comparison diagnostics; the active trigger and compaction output remain unchanged in this phase.
- Keep the analysis domain provider-neutral so later changes can add the optimizer, automatic-compaction policy, and evidence UI without rewriting provider adapters.

## Capabilities

### New Capabilities

- `agent-context-measurement`: Provider-neutral OnePiece context snapshots, measurement-quality provenance, protocol-safe API-round grouping, semantic content classification, retention classification, model-window capacity calculation, and shadow decision diagnostics.

### Modified Capabilities

- `agent-context-compaction`: Clarify that the existing character-count trigger remains authoritative while the new token-aware decision operates in non-mutating shadow mode during this phase.

## Impact

- **Desktop runtime:** Primary impact. The Rust `agent_runtime` request path gains context-analysis domain models and application services invoked before initial and tool-continuation provider requests.
- **Provider adapters:** Existing reported-usage normalization is consumed through provider-neutral models; Anthropic and OpenAI-compatible wire adapters do not gain context-management policy.
- **Token accounting:** Existing invocation observations remain the source of reported usage. Context snapshots reference safe normalized counters and quality rather than copying prompts or raw provider payloads.
- **Unified logging:** Shadow comparisons use the existing logging service with bounded reason codes, counts, ratios, stable correlation identifiers, and no message content, credentials, raw protocol frames, or feature-local log files.
- **Frontend and Web runtime:** No user-facing UI or service-contract change in this phase. Web/mock compaction behavior remains unchanged; parity for user-visible evidence belongs to the later evidence-projection phase.
- **Compatibility:** No breaking changes. The existing 60,000-character trigger, six-turn retention behavior, summary generation, and visible compaction notice remain authoritative.
