## Context

See `proposal.md` for motivation. Phases one through three already provide classified context snapshots, a token-aware optimizer with compatibility fallback, and generation-scoped trigger/suppression state. Successful compaction currently emits a generic card, while the shared application settings model already persists personalization values through symmetric Tauri and Web adapters. The rich-card message contract already supports visible fields and metadata and is persisted with conversation messages.

## Goals / Non-Goals

**Goals:**

- Capture one immutable, content-free evidence object for every successful automatic compaction.
- Reuse existing message persistence and rendering contracts.
- Capture the persisted user preference once per generation and combine it with existing request suppression.
- Keep desktop and Web/mock settings and evidence behavior contract-compatible.

**Non-Goals:**

- A manual “compact now” command, provider-native prompt caching, compression history dashboard, or evidence export API.
- Displaying the generated summary or any source context in evidence.
- Changing compaction thresholds, cooldown budgets, circuit limits, or optimizer policy.

## Decisions

### Reuse the rich-card protocol

The runtime will emit the existing `card` block with stable evidence fields and metadata. This preserves transcript storage, chronological rendering, accessibility behavior, and adapter compatibility without a new message schema. A dedicated rich-block kind was rejected because the current card contract already expresses the bounded key/value projection and a new kind would duplicate renderer and persistence work.

### Build evidence from immutable before/after snapshots

The automatic-compaction coordinator will retain the already prepared pre-compaction snapshot, prepare a post-compaction snapshot after either success path, and derive non-negative deltas from those two snapshots. Token values remain optional and are emitted only when the measurement source supports them. This avoids estimates being presented as authoritative evidence.

The typed success outcome will identify optimizer versus compatibility path so the coordinator can emit exactly one evidence card. Central emission is preferred over path-local cards because it prevents contract drift and duplicate events.

### Treat evidence as an allowlisted DTO

Only counts, measurement-quality labels, trigger/path labels, and policy versions cross into rich-card fields and metadata. The DTO cannot accept arbitrary source text. Tests will use secret-like prompt and tool content and assert absence from serialized evidence.

### Persist the control through shared application settings

The frontend `AppSettings` contract gains `automaticContextCompactionEnabled`, defaulting to `true`. Existing normalization makes upgrades additive. The OnePiece parameter panel uses `useSettings().saveSetting`; Tauri invocation remains in the Tauri settings adapter, and Web storage remains in the Web adapter.

The native personalization/settings gateway will include this preference in the generation-start snapshot. The generation coordinator derives the effective mode as suppressed when either the request override or persisted preference disables compaction. Capturing once per generation preserves deterministic behavior if the setting changes mid-stream.

### Keep Web/mock behavior deterministic

Web/mock checks the same persisted setting before its existing simulated compaction branch. When enabled, it emits the same field/meta keys using exact character counts and unavailable token values. It does not claim provider token accounting.

## Risks / Trade-offs

- [Post-compaction token measurement can be unavailable] → Preserve optional token fields and explicit quality labels; character evidence remains exact.
- [A successful compaction followed by evidence sink failure could affect generation flow] → Follow existing event-sink failure semantics and test that no duplicate compaction attempt is introduced.
- [Adding an application setting touches several DTO mappers] → Use a single canonical key, default it to enabled at every normalization boundary, and add round-trip tests for both adapters.
- [Evidence labels could become tightly coupled to English runtime strings] → Keep metadata enum-like and localize the user-facing card labels in runtime-compatible presentation strings where the current protocol permits.

## Migration Plan

1. Add the optional-on-read/default-enabled setting across frontend and native setting models.
2. Add the UI control and adapter round-trip tests.
3. Add the native generation snapshot field and effective suppression behavior.
4. Centralize successful compaction evidence emission and align Web/mock output.
5. Deploy additively; rollback removes the UI/field use while old persisted unknown keys remain harmless.
