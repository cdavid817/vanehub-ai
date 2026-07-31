## Why

The native API-based agent's history fetch (`ConversationHistoryPort::recent_messages(session_id, HISTORY_LIMIT = 50)`) is a flat message-count cap with no awareness of actual context size — and Phase 2's tool-use loop can now produce large turns (file-read tool output especially) within a single generation on top of that. A long-running or tool-heavy session will eventually build a request that either exceeds the model's real context window (a hard failure) or silently sends far more history than the conversation needs. This change adds compaction so long conversations keep working.

## What Changes

- Affects the **desktop runtime primarily** (real provider calls); the Web/mock runtime gets a deterministic simulated compaction event so the frontend contract stays exercised without a real provider call.
- Add a character-count-based compaction trigger, consistent with the existing `MessageTokenUsage`/`source: "character-count"` convention already used for usage accounting (`AgentRuntimeApplicationService`'s `complete_claimed`) rather than introducing real token counting or a provider token-counting endpoint call.
- When the accumulated turns for a generation (initial history plus, within the Phase 2 tool-use loop, any turns appended so far) cross the trigger threshold, call the model once with a summarization-only request to condense the older portion into a single synthetic turn, then continue the generation using the summary in place of what it replaced.
- Insert a visible, distinctly-rendered notice into the chat transcript when compaction happens, so the user knows earlier turns were condensed rather than silently missing.
- Apply the same compaction mechanism uniformly whether the turns being compacted came from `ConversationHistoryPort` (across a session's prior user turns) or were accumulated in-memory within one generation's Phase 2 tool-use loop (many large tool results in one turn) — one mechanism operating on "the current turns list," not two separate ones.
- Add Web/mock parity: `web-agent-client.ts` simulates a deterministic compaction notice for long mock sessions.

## Capabilities

### New Capabilities
- `agent-context-compaction`: character-count-based compaction triggering, LLM-summarization of older turns into a synthetic turn, and the visible in-transcript compaction notice, for the native API-based agent's generation turns list.

### Modified Capabilities
- None. `api-agent-runtime` and `agent-tool-execution` (introduced by the still-unarchived `add-custom-agent-registration` and `add-agent-tool-execution` changes) are extended in behavior — the turns list a generation sends can now be a compacted version of the full history — but since neither has an archived baseline yet to diff against, this change describes the compaction behavior entirely within the new `agent-context-compaction` capability rather than as a delta against not-yet-merged specs.

## Impact

- **`agent_runtime` (Rust, primary)**: `RuntimeAgentApiAdapter::execute()` gains a compaction check before building each request (both at the initial history fetch and around the Phase 2 tool-use loop's turn-accumulation); a new summarization call path reusing the existing wire-format request/response machinery; the visible compaction notice reuses the existing `GenerationProcessEvent::RichBlock` variant — no new event type — following the same `RichBlock` → `sessions gateway` persistence pattern already used for tool-use/rich-blocks (see design.md Decision 4).
- **Frontend**: no new rendering case needed — the compaction notice reuses the existing `RichBlock` "card" mechanism, already rendered distinctly from a normal assistant message (see design.md Decision 4). `web-agent-client.ts` gains a deterministic simulated compaction event (a `rich_block` card) for sufficiently long mock sessions.
- **Unaffected**: `agent-terminal-runtime`, `cli-parameter-management`, `prompt-hook-management`, the existing CLI process adapter and its 4 managed CLIs, `skill-management`, cross-session memory (a distinct, later concern — this change only compacts within what a generation already sends, not across sessions).
- No breaking changes: purely additive: sessions/generations that never cross the trigger threshold behave exactly as they do today.
