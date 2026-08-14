## 1. Runtime

- [x] 1.1 Add the `ask_user_question` tool definition with a closed schema bounding one question and two to four options.
- [x] 1.2 Add an `awaiting_input` variant to `ToolLifecyclePhase` and handle it at every match site.
- [x] 1.3 Extend the tool-resolution decision with an answered variant carrying the answer text, and treat it as resolved wherever decisions are consumed.
- [x] 1.4 Publish the question, block the tool call on the existing approval channel, and return the answer as the tool result.
- [x] 1.5 Derive interactivity from the execution profile; exclude the tool from non-interactive catalogs and refuse it in the executor regardless of what the catalog offered.
- [x] 1.6 Validate bounds before publishing, so a rejected call neither renders nor blocks.
- [x] 1.7 Classify the tool as a no-approval operation in the permission mapping.

## 2. Boundary and UI

- [x] 2.1 Add a Tauri command and `AgentRuntimeApi` method that resolve a pending question by session and call id.
- [x] 2.2 Add the matching `AgentService` contract method with Tauri and Web/mock implementations.
- [x] 2.3 Add `awaiting_input` to the shared `ToolUseBlock` status union and handle it in the tool block renderer.
- [x] 2.4 Add a question card that renders the model's options plus a free-text field and submits the answer through the service boundary.
- [x] 2.5 Add the card's strings to every shipped locale.

## 3. Tests

- [x] 3.1 Bound-validation tests for question count, option count, empty text, and oversized text, each asserting nothing was published.
- [x] 3.2 Executor tests for the answered path, the cancelled path, and an answer for an unknown or already-resolved question.
- [x] 3.3 Non-interactive refusal tests covering Loop, scheduled-task, Plan attempt, and delegated Utility profiles.
- [x] 3.4 Catalog and permission tests covering interactive, plan-mode, and non-interactive catalogs.
- [x] 3.5 Component tests for the question card, including the free-text answer path.
- [x] 3.6 Web/mock adapter parity tests.
- [x] 3.7 A Playwright case covering the presented-question-to-answer round trip.

## 4. Validation

- [x] 4.1 `npm run lint:ci`
- [x] 4.2 `npm run test`
- [x] 4.3 `npm run build`
- [x] 4.4 `npx playwright test`
- [x] 4.5 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 4.6 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 4.7 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.8 `openspec validate add-agent-user-question --strict`
