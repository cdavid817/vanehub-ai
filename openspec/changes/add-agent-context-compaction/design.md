## Context

`RuntimeAgentApiAdapter::execute()` (`src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`) fetches history once via `ConversationHistoryPort::recent_messages(session_id, HISTORY_LIMIT = 50)`, converts it to wire-native turns via `(wire_format.history_to_turns)(&recent)`, then (per Phase 2 / `add-agent-tool-execution`) runs a bounded tool-use round-trip loop that extends that same in-memory `Vec<Value>` of turns via `(wire_format.build_reply_turns)(...)` each iteration. Neither the initial fetch nor the loop has any awareness of how large the accumulated turns are.

The codebase already has a precedent for measuring conversation size without real provider token counts: `AgentRuntimeApplicationService::complete_claimed` computes `MessageTokenUsage` from `response.chars().count()` and explicitly labels it `source: "character-count"` — an approximation, not a real token count. Neither `anthropic_provider.rs` nor `openai_compatible_provider.rs` currently parses any usage field from the streaming response at all (confirmed by re-reading both `translate_sse_data` implementations this session).

The existing `GenerationProcessEvent::RichBlock(Value)` variant already has a full path from adapter → `GenerationEventHandler::rich_block()` → `sessions.append_rich_block()` → `AgentEvent::MessageRichBlock` → frontend `RichBlocks` rendering, including a `"card"` kind with `title`/`bodyMarkdown`/`tone`/`fields` already used for non-conversational, system-style messaging (Phase 1's Web mock uses it for a "Web preview summary" card).

## Goals / Non-Goals

**Goals:**
- Detect when a generation's accumulated turns are getting large using a character-count heuristic, consistent with the existing `source: "character-count"` convention — no real provider token counting is added in this phase.
- When triggered, summarize the older portion of the turns into one synthetic turn via a single extra model call, keeping the most recent turns verbatim.
- Apply this uniformly to turns from `ConversationHistoryPort` (across a session's history) and turns accumulated within one generation's Phase 2 tool-use loop — one mechanism, one trigger check, reused at both points `execute()` builds a request.
- Make compaction visible: insert a distinct notice into the chat transcript when it happens, reusing the existing `RichBlock` "card" mechanism rather than inventing a new event/persistence/rendering path.
- Preserve Web/mock parity with a deterministic simulated compaction event.

**Non-Goals:**
- Real provider token counting (parsing `usage` fields from streaming responses, or calling a token-counting endpoint) — the character-count heuristic is the explicit choice for this phase, matching existing precedent. Revisit only if character-count proves too inaccurate in practice.
- Cross-session memory (a distinct, later concern — this only compacts within what a single generation would otherwise send; nothing persists across separate generations beyond what already persists today).
- User-configurable compaction thresholds or an opt-out UI control — a fixed, conservative default this phase; revisit if needed later.
- Any change to CLI-based agents — they manage their own context/compaction internally (e.g. Claude Code's own auto-compact when run interactively); VaneHub only streams their stdout and does not drive it.

## Decisions

### 1. Character-count trigger, checked at both points `execute()` builds a request

A module-level constant (e.g. `COMPACTION_TRIGGER_CHARACTERS: usize`, a conservative default sized well under typical model context windows, tunable at implementation time) is compared against the summed character length of all turns' `content` text. Checked in exactly two places: once right after the initial `history_to_turns` conversion (before the round-trip loop's first request), and once after each `turns.extend(build_reply_turns(...))` inside the loop (since tool output, especially file reads, can push the total over the threshold mid-generation).

**Why:** matches the codebase's own existing approximation convention (Decision rationale shared with `complete_claimed`'s `source: "character-count"`) rather than introducing real token counting, which neither wire-format module currently supports and which would require provider-specific parsing this phase deliberately defers (see Non-Goals).

### 2. Compaction keeps the most recent turns verbatim, summarizes everything older into one synthetic turn

A small constant (e.g. `COMPACTION_KEEP_RECENT_TURNS: usize`) of the most recent turns stay untouched; everything before them is sent to the model in one extra, non-streamed-to-the-user summarization call asking for a concise recap preserving key facts and decisions, and the result replaces them as a single `{role: "user", content: "<summary>"}` turn prepended before the kept ones. No new wire-format code is needed for the *replacement* turn itself — it's the same plain `{role, content}` shape `history_to_turns` already produces; only the *summarization call* is new.

**Why:** simplest structure that keeps recent, most-relevant context byte-for-byte accurate while bounding the rest. Keeping a small fixed window verbatim (rather than summarizing down to zero) avoids losing the immediate thread of conversation the model needs to respond coherently right now.

**Alternative considered:** compact incrementally / keep a running summary updated turn-by-turn — rejected as unnecessary complexity for this phase; recomputing a fresh summary of "everything older than the recent window" each time it's triggered is simpler to reason about and test, at the cost of occasionally re-summarizing content that was already summarized before (acceptable; triggers are infrequent by construction).

### 3. The summarization call reuses `WireFormat`'s existing request/response machinery, not a new HTTP path

A new small function builds a summarization request using the *same* `wire_format.build_request_body`/`endpoint`/`apply_auth` as regular generation (appending one instruction turn asking for a summary to the turns being compacted), sends it, and reads the SSE stream with the *same* `wire_format.translate_sse_data`/`ToolCallAccumulator` machinery already built — except it accumulates the full text response instead of forwarding `Token` events to the sink, and does not declare tools (no `tool_catalog()` in this request). This is a blocking, internal call from `execute()`'s perspective, not something the chat UI streams live.

**Why:** avoids building a second HTTP/SSE code path. The only genuinely new piece is "accumulate text instead of streaming it" and "don't declare tools" — both small, local variations on machinery Phase 1-3 already built and unit-tested.

### 4. Visible notice reuses the existing `RichBlock` "card" mechanism — no new event type

When compaction happens, `execute()` calls `sink.handle(GenerationProcessEvent::RichBlock(...))` with a `"card"` block (`tone: "info"`, a short title/body noting that earlier turns were condensed) *before* continuing the generation — the exact same mechanism Phase 1's Web mock already uses for non-conversational system messaging, with the exact same existing persistence (`sessions.append_rich_block`) and frontend rendering (`RichBlocks` component) already in place.

**Why:** a dedicated new `GenerationProcessEvent` variant plus new persistence plus new frontend rendering would duplicate a path that already exists and already does exactly this job (attach a distinct, non-message block to the transcript). This is the same "reuse over new vocabulary" principle applied in Phase 1 (`ProviderOutputEvent`/`GenerationProcessEvent` reuse) and Phase 2 (reusing `ToolUseBlock`/`ToolLifecyclePhase` rather than inventing parallel types).

**Alternative considered:** a new `GenerationProcessEvent::ContextCompacted` variant with its own persistence — rejected; `RichBlock`'s existing shape (`title`, `bodyMarkdown`, `tone`) already carries everything a compaction notice needs.

### 5. Applies uniformly to CLI-history-derived and tool-loop-accumulated turns

The compaction check operates on `execute()`'s local `turns: Vec<Value>`, regardless of whether those turns originated from `ConversationHistoryPort` or were appended by the Phase 2 tool-use loop. There is exactly one compaction function, called at both points turns grow.

**Why:** both are the same underlying problem ("the turns list going to the model is getting large") and treating them as two separate mechanisms would duplicate the trigger/summarize/notify logic for no behavioral benefit.

## Risks / Trade-offs

- **[Risk]** Character count is a rough proxy for token count (varies by language, whitespace, encoding) — could trigger compaction later or earlier than an ideal token-aware system would. → **Mitigation:** the trigger threshold is conservative by construction (well under typical context windows) and this matches an already-accepted approximation elsewhere in the codebase; revisit only if this proves inaccurate in practice.
- **[Risk]** The extra summarization call adds latency and cost when triggered. → **Mitigation:** triggers are infrequent by construction (only long/tool-heavy sessions); the visible notice (Decision 4) makes this transparent to the user rather than a silent slowdown.
- **[Risk]** Summarization is lossy — the model could omit something the user later needs. → **Mitigation:** this is the explicit, user-confirmed trade-off of choosing LLM-summarization over a lossless-but-unbounded alternative; the visible notice at least tells the user it happened, so they know to re-state something important if needed.
- **[Trade-off]** No user-configurable threshold this phase (Non-Goals) — acceptable; revisit if a fixed default proves wrong for real usage patterns.

## Migration Plan

Purely additive: a new constant-driven check inside `execute()`, a new small summarization helper function, reuse of the existing `RichBlock` event/persistence/rendering path. No schema changes (the compaction notice persists through the same `append_rich_block` mechanism existing rich blocks already use). Sessions that never cross the trigger threshold are completely unaffected — same behavior as today.

## Open Questions

- Exact values for `COMPACTION_TRIGGER_CHARACTERS` and `COMPACTION_KEEP_RECENT_TURNS` — pick conservative defaults at implementation time (informed by typical model context windows and this codebase's existing `bounded_count` precedent for capping counts), not architectural decisions to resolve here.
- Exact summarization instruction wording — an implementation detail for tasks.md, not a design-level decision.
