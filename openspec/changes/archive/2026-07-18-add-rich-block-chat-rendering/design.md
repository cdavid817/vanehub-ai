## Context

VaneHub AI already has a chat surface with streamed assistant text, collapsible thinking content, and tool-use blocks. The current message model stores these as separate fields:

- `content` for visible text
- `thinkingContent` / `thinking_content` for reasoning output
- `toolUse` / `tool_use` for tool events

Clowder AI uses a richer message attachment model where messages can carry `extra.rich.blocks`. Its base protocol supports `card`, `diff`, `checklist`, `media_gallery`, `audio`, `interactive`, `html_widget`, and `file`, while additional specialized cards are layered on top of `card` metadata. VaneHub AI should adopt the useful protocol shape without importing Clowder-specific product behaviors or its store architecture.

The change affects both desktop and Web runtimes. React must continue to depend on `AgentService`; Tauri `invoke()` calls remain inside `src/services/tauri-agent-client.ts`; SQLite and provider CLI parsing stay in `src-tauri/`.

## Goals / Non-Goals

**Goals:**

- Add a durable `richBlocks` field to `ChatMessage`.
- Add a `rich_block` chat stream event that appends a structured block to a message.
- Persist Rich Blocks in desktop SQLite and return them through `list_messages`.
- Keep Web/mock behavior contract-compatible with desktop behavior.
- Render first-version block kinds consistently in both `futuristic` and `minimal` visual styles.
- Localize frontend-owned labels, fallback text, action titles, and status text in Simplified Chinese and English.
- Preserve existing `ThinkingBlock` and `ToolUseBlock` behavior for backward compatibility.
- Document the short-term implementation and future optimization path for later expansion.

**Non-Goals:**

- Do not replace the whole message model with Clowder's `extra.rich` envelope in the first version.
- Do not implement Clowder-specific business cards such as proposal, handoff, community issue, or callback-auth cards.
- Do not add a new frontend state library or UI component library.
- Do not implement full `interactive` block click-to-send behavior or persisted option state in the first version.
- Do not add a general upload/media hosting backend beyond rendering supported URLs already provided by the runtime.

## Decisions

### Decision 1: Add `richBlocks` beside existing message fields

Use an additive field:

```ts
interface ChatMessage {
  content: string;
  thinkingContent?: string;
  toolUse?: ToolUseBlock[];
  richBlocks?: RichBlock[];
}
```

Rationale: this keeps existing messages, tests, persistence, and UI behavior stable. A full migration to `extra.rich` would touch every message consumer and create unnecessary risk.

Alternative considered: adopt Clowder's `extra.rich.blocks` shape directly. This was rejected for the first version because VaneHub does not currently have Clowder's cross-thread, connector, or specialized card metadata model.

### Decision 2: Support the Clowder-compatible base block kinds

Define a VaneHub `RichBlock` union compatible with the stable base fields:

- `card`: `title`, optional `bodyMarkdown`, `tone`, `fields`, `meta`
- `diff`: `filePath`, `diff`, optional `languageHint`
- `checklist`: optional `title`, `items`
- `media_gallery`: optional `title`, `items`
- `file`: `url`, `fileName`, optional `mimeType`, `fileSize`
- `audio`: `url`, optional `text`, `title`, `durationSec`, `mimeType`
- `html_widget`: `html`, optional `title`, `height`
- `interactive`: typed but rendered as a disabled/preview block in the first version

Rationale: these are general coding-agent output primitives and do not bake in Clowder product concepts.

Alternative considered: only add `card`, `diff`, and `checklist`. This would be simpler but would immediately block common output such as screenshots, generated files, audio summaries, and small HTML visualizations.

### Decision 3: Persist blocks as JSON text in SQLite

Add an additive `rich_blocks TEXT` column to `messages`. Store the serialized `RichBlock[]` and deserialize it when loading messages. Invalid persisted JSON should degrade to no blocks and record diagnostics through existing native logging where appropriate.

Rationale: this matches the current `tool_use TEXT` persistence approach and avoids creating multiple relational tables before block querying or mutation is needed.

Alternative considered: normalize blocks into a `message_rich_blocks` table. That is better for future per-block mutation, analytics, and indexing, but it is unnecessary for first-version append-only rendering.

### Decision 4: Add a stream event instead of encoding blocks in token text only

Extend `ChatStreamEvent` with:

```ts
{ type: "rich_block"; sessionId: string; messageId: string; block: RichBlock }
```

Desktop provider parsers should also detect Clowder-style structured lines such as JSON payloads shaped like `{ "type": "rich_block", "block": ... }`. Optional `cc_rich` fenced extraction may be added as a fallback when provider output includes inline blocks in assistant text.

Rationale: explicit events preserve ordering, persistence, and UI updates without forcing the renderer to parse ordinary markdown text.

Alternative considered: parse all rich blocks from final content only. This would be easier but would not stream blocks into the UI and would complicate persistence during cancellation or failure.

### Decision 5: Build a local Rich Block dispatcher under `src/components/chat`

Create a `RichBlocks` dispatcher and one renderer per supported block kind. Renderers should use Tailwind semantic tokens and existing visual density:

- 8px or smaller radii
- no nested decorative card-in-card layout
- no hard-coded one-off palettes
- compact operational text sizing
- stable dimensions for media, iframe, and action surfaces

Use `react-markdown` for card body markdown and plain code/pre rendering for diffs. Use lucide icons where useful.

Rationale: a local dispatcher fits VaneHub's current component structure and avoids importing Clowder UI assumptions.

Alternative considered: copy Clowder's `packages/web/src/components/rich` tree. This would pull in product-specific styles, store calls, trusted-source gates, and app-specific actions that do not exist in VaneHub.

### Decision 6: Treat `html_widget` as sandboxed display only

Render `html_widget` in an iframe with `srcDoc`, a bounded height, and restrictive sandbox permissions. The first version should not grant same-origin, popups, downloads, or top navigation.

Rationale: provider output is untrusted content. A sandbox preserves visual utility while limiting risk.

Alternative considered: render raw HTML into the React tree. This is unsafe and inconsistent with the chat threat model.

### Decision 7: Defer interactive behavior

Include `interactive` in the type union for forward compatibility, but first-version UI should render it as a disabled/preview block or a read-only options list. Full behavior needs a separate contract for user selection, message sending, service calls, and persisted per-block state.

Rationale: interactive blocks are not just rendering. They change the chat input and persistence model.

Alternative considered: implement Clowder-style `updateRichBlock` and direct action callbacks now. This would require a new service method, native command, Web mock behavior, and security rules before the base rendering contract is proven.

## Short-Term Implementation

1. Extend frontend chat types:
   - Add `RichBlockKind` and `RichBlock` union types in `src/types/chat.ts`.
   - Add `richBlocks?: RichBlock[]` to `ChatMessage`.
   - Add a `rich_block` variant to `ChatStreamEvent`.

2. Extend event application:
   - Update `applyChatEvent` to append or replace blocks by stable `id`.
   - Keep `thinking` and `tool_use` behavior unchanged.

3. Extend Web/mock adapter:
   - Persist `richBlocks` in the in-memory message map.
   - Emit representative mock `card`, `checklist`, and `diff` or `media_gallery` blocks for predictable tests.

4. Extend Tauri adapter and Rust model:
   - Update Rust `ChatMessage` and `ChatStreamEvent`.
   - Add a migration for `messages.rich_blocks`.
   - Add load/serialize helpers for `Vec<RichBlock>`.
   - Add an append helper similar to `append_assistant_tool_use`.

5. Extend provider parsing:
   - Add a `ParsedAgentEvent::RichBlock`.
   - Detect explicit JSON lines shaped like `{"type":"rich_block","block":{...}}`.
   - Optionally detect fenced `cc_rich` blocks in buffered content as a fallback; keep this scoped and tested.

6. Add React renderers:
   - `RichBlocks.tsx` dispatcher.
   - `CardBlock`, `DiffBlock`, `ChecklistBlock`, `MediaGalleryBlock`, `FileBlock`, `AudioBlock`, `HtmlWidgetBlock`, and a read-only `InteractiveBlock`.
   - Integrate `<RichBlocks blocks={message.richBlocks ?? []} />` in `MessageItem`.

7. Add i18n:
   - Add synchronized keys under `chat.richBlock.*` in both locale files.
   - Localize unknown block fallback, file size labels, checklist progress, audio labels, widget title fallback, and disabled interactive text.

8. Verify:
   - Run focused Vitest tests for event application and component rendering.
   - Run Rust tests for parser and persistence.
   - Run the required project checks before implementation is considered complete.

## Future Optimization Points

- Move from `messages.rich_blocks TEXT` to a normalized `message_rich_blocks` table when block search, per-block updates, or analytics become necessary.
- Implement full `interactive` selection behavior through `AgentService`, including persisted `disabled` and `selectedIds` state.
- Add trusted-provenance gates for specialized card renderers if future blocks trigger privileged actions.
- Add media publication support for local generated images, returning stable runtime-served URLs instead of requiring external URLs.
- Add block-level validation with a shared schema used by TypeScript and Rust to reduce drift.
- Preserve and render block ordering relative to text chunks if provider streams interleaved text/block/text sequences become important.
- Add artifact extraction/indexing so files, diffs, and media blocks can appear in session workspace tabs.
- Add visual regression coverage for representative Rich Blocks in both visual styles and narrow/desktop widths.
- Add provider-specific rich output mappers for Codex, Claude, Gemini, and OpenCode as their structured output formats mature.

## Risks / Trade-offs

- Rich Block payloads can be malformed or malicious -> validate shape, bound rendered sizes, and show localized fallback UI for unsupported blocks.
- `html_widget` can execute untrusted code -> render only in a restrictive iframe sandbox with bounded height.
- JSON-in-text fallback parsing can accidentally remove user-visible content -> prefer explicit `rich_block` events and keep fallback extraction narrow.
- Additive SQLite JSON storage is less queryable -> acceptable for first-version append-only rendering; document normalized storage as a later optimization.
- Media URLs may not resolve in desktop runtime -> render broken-media fallback and defer upload/publication service to a later change.
- Specialized Clowder cards may appear as ordinary cards -> acceptable for first version; preserve `meta` for future routing without privileged behavior.
