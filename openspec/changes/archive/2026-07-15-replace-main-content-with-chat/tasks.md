## 1. Main Content Chat UI

- [x] 1.1 Replace the flowchart canvas in `src/main-layout/main-layout.tsx` with a chat transcript area.
- [x] 1.2 Render existing `workspace.chatMessages` in the transcript with user and agent message styling.
- [x] 1.3 Ensure the transcript uses internal scrolling and the composer remains fixed and non-shrinking.
- [x] 1.4 Remove unused flowchart-only imports, types, helpers, and rendering code from `main-layout.tsx`.

## 2. Verification

- [x] 2.1 Run `openspec validate "replace-main-content-with-chat" --strict`.
- [x] 2.2 Run `npm run build`.
- [x] 2.3 Run `npm run test`.
