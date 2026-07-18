## Why

VaneHub AI chat currently renders streamed text plus two specialized structures: thinking and tool use. Coding-agent replies increasingly contain richer structured output such as cards, diffs, checklists, media, files, and embeddable widgets, so the chat surface needs a durable Rich Block contract instead of adding one-off message fields for every new shape.

## What Changes

- Add first-version Rich Block support to chat messages for desktop and Web runtimes.
- Persist Rich Blocks with messages in the desktop SQLite layer while preserving the existing `content`, `thinkingContent`, and `toolUse` fields.
- Extend chat stream events with a `rich_block` event that appends one structured block to the active assistant message.
- Render a first set of supported Rich Block kinds in the React chat UI: `card`, `diff`, `checklist`, `media_gallery`, `file`, `audio`, and `html_widget`.
- Keep `interactive` in the protocol shape as a planned extension point, but do not implement full click-to-send or state persistence in the first version.
- Add safe fallback rendering for unknown or invalid block kinds.
- Keep labels, statuses, fallback text, and controls localized in Simplified Chinese and English.
- Preserve current service-boundary rules: React components continue to use `AgentService`, and Tauri-specific calls stay inside the Tauri adapter.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-experience`: Chat messages and stream events support durable Rich Blocks, and the message list renders supported structured block kinds with localized UI.

## Impact

- Frontend types: `src/types/chat.ts` gains Rich Block union types and a `richBlocks` message field.
- Frontend event handling: `src/services/chat-events.ts`, `src/services/web-agent-client.ts`, and `src/services/tauri-agent-client.ts` keep Tauri/Web adapter parity.
- Frontend UI: `src/components/chat/` gains a Rich Block dispatcher and block renderers that use Tailwind tokens and current visual styles.
- i18n: `src/i18n/locales/zh-CN.json` and `src/i18n/locales/en.json` gain synchronized chat Rich Block keys.
- Desktop runtime: `src-tauri/src/lib.rs` adds message persistence for `rich_blocks` and normalizes provider output into `rich_block` events.
- Database: SQLite messages schema receives an additive `rich_blocks TEXT` column migration.
- Tests: TypeScript unit/component tests and Rust tests cover event application, Web mock parity, parser behavior, and persistence.
