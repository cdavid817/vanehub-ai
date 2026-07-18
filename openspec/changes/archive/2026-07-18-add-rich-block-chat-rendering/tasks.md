## 1. Chat Contract

- [x] 1.1 Add `RichBlockKind`, `RichBlock`, and per-kind TypeScript interfaces in `src/types/chat.ts`.
- [x] 1.2 Add `richBlocks?: RichBlock[]` to `ChatMessage`.
- [x] 1.3 Add the `rich_block` variant to `ChatStreamEvent`.
- [x] 1.4 Update contract conformance coverage so `src/contracts/agent.ts` and `src/types/chat.ts` stay aligned where applicable.

## 2. Event Handling And Web Runtime

- [x] 2.1 Update `src/services/chat-events.ts` to append or replace Rich Blocks by stable block id.
- [x] 2.2 Update `src/services/web-agent-client.ts` to persist `richBlocks` in Web/mock messages.
- [x] 2.3 Add deterministic Web/mock Rich Block stream events for representative first-version block kinds.
- [x] 2.4 Add or update Vitest coverage for Rich Block event application and Web/mock persistence.

## 3. Desktop Runtime And Persistence

- [x] 3.1 Add Rust Rich Block structs/enums or validated JSON value handling in `src-tauri/src/lib.rs`.
- [x] 3.2 Add an additive SQLite migration for `messages.rich_blocks TEXT`.
- [x] 3.3 Update desktop message loading and serialization to return `richBlocks`.
- [x] 3.4 Add an `append_assistant_rich_block` helper that persists block arrays without dropping existing blocks.
- [x] 3.5 Extend `ChatStreamEvent` and `ParsedAgentEvent` with Rich Block support.
- [x] 3.6 Parse explicit provider JSON output shaped like `{"type":"rich_block","block":...}` into persisted and emitted Rich Block events.
- [x] 3.7 Add Rust tests for migration, message load/list behavior, duplicate block handling if implemented natively, and parser normalization.

## 4. Rich Block Rendering

- [x] 4.1 Add `src/components/chat/RichBlocks.tsx` dispatcher with safe fallback rendering.
- [x] 4.2 Add first-version renderers for `card`, `diff`, `checklist`, `media_gallery`, `file`, `audio`, and `html_widget`.
- [x] 4.3 Add read-only preview rendering for `interactive` blocks without sending messages or invoking native commands.
- [x] 4.4 Integrate Rich Block rendering into `MessageItem` after text, errors, thinking, and tool-use content.
- [x] 4.5 Keep renderer styling based on Tailwind semantic tokens and verify it works in both `futuristic` and `minimal` styles.
- [x] 4.6 Ensure `html_widget` renders in a bounded sandboxed iframe rather than injecting HTML into the React document.

## 5. Localization

- [x] 5.1 Add synchronized `chat.richBlock.*` keys to `src/i18n/locales/zh-CN.json` and `src/i18n/locales/en.json`.
- [x] 5.2 Localize unknown block fallback, checklist progress, file metadata labels, audio labels, widget labels, and interactive-disabled text.
- [x] 5.3 Run or update i18n parity tests so new keys stay synchronized.

## 6. Verification

- [x] 6.1 Run focused frontend tests for chat event handling and Rich Block rendering.
- [x] 6.2 Run `npm run test`.
- [x] 6.3 Run `npm run build`.
- [x] 6.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.6 Run `openspec validate add-rich-block-chat-rendering --strict`.
- [x] 6.7 Perform visual QA for representative Rich Blocks in both visual styles at desktop and narrow widths.
