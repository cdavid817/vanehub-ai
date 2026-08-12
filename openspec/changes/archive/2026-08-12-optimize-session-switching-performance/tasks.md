## 1. Optimistic session switching

- [x] 1.1 Change the main-layout switch mutation to publish the selected loaded session immediately through the active-session query cache
- [x] 1.2 Add latest-request-wins completion and failure rollback without broad session-query invalidation
- [x] 1.3 Avoid switching or resetting the workspace when the selected card is already active

## 2. Regression coverage

- [x] 2.1 Add model tests for unresolved optimistic switching, persistence rollback, and out-of-order rapid switching
- [x] 2.2 Add sidebar/workspace coverage proving cached content and session-scoped state change only for a different active id
- [x] 2.3 Add Playwright coverage for rapid session switching ending on the most recent selection

## 3. Verification

- [x] 3.1 Run focused model, component, and Playwright regression tests
- [x] 3.2 Run the project frontend, Rust, coverage, contract, and build validation commands required by `AGENTS.md`
- [x] 3.3 Run `openspec validate optimize-session-switching-performance --strict` and `openspec validate --specs --strict`
