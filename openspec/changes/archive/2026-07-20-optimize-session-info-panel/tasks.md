## 1. Session Usage Contract

- [x] 1.1 Add shared TypeScript types for a compact session usage summary with reported token totals, estimated character totals, coverage counts, response count, and generated timestamp.
- [x] 1.2 Extend `AgentService` with a session-scoped usage method and update contract conformance coverage for the new shape.
- [x] 1.3 Add the matching Tauri frontend adapter method without exposing `invoke()` to React components.
- [x] 1.4 Add the matching Web/mock adapter method with deterministic per-session reported and estimated usage behavior.

## 2. Native Usage Summary

- [x] 2.1 Add an application/repository query that aggregates `usage_records` by one `session_id` only.
- [x] 2.2 Add a Tauri command and DTO mapper for session usage summary responses.
- [x] 2.3 Register the new command and include command contract tests or mapper tests for zero, reported, estimated, and mixed usage cases.
- [x] 2.4 Ensure unknown or deleted sessions cannot leak usage from other sessions and return a bounded service error or isolated zero result per existing policy.

## 3. Information Panel UI

- [x] 3.1 Replace the information panel tab model with Basic Info, Token Usage, and Skill while preserving keep-alive rendering and collapse state.
- [x] 3.2 Update Basic Info to show active CLI identity, lifecycle state, project/worktree context, and `getSessionChatConfig(sessionId).modelId` with a localized empty state.
- [x] 3.3 Add Token Usage content that displays reported token totals as primary and estimated response/character context separately when reported tokens are absent.
- [x] 3.4 Add Skill content that queries global and active-workspace Skills, groups available Skills for the selected CLI separately from project Skills, and de-emphasizes disabled project Skills.
- [x] 3.5 Preserve compact 300px panel behavior with shared `ucd-*` classes, semantic tokens, stable tab dimensions, truncation, loading states, empty states, and accessible labels.

## 4. Localization and Theme Parity

- [x] 4.1 Add synchronized zh-CN and en translation keys for new tabs, labels, loading states, empty states, token fields, and Skill group headings.
- [x] 4.2 Add or update tests that guard i18n key parity for the new workspace labels.
- [x] 4.3 Verify the implementation does not branch on `futuristic` or `minimal` theme ids and uses shared design tokens.

## 5. Verification

- [x] 5.1 Add focused frontend tests for tab labels, Basic Info model display, token fallback behavior, and Skill grouping.
- [x] 5.2 Run `npm run test`.
- [x] 5.3 Run `npm run build`.
- [x] 5.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.6 Run `openspec validate optimize-session-info-panel --strict` and `openspec validate --specs --strict`.
