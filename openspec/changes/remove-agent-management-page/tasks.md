## 1. Remove the Agent Management surface

- [x] 1.1 Remove the registered-Agent runtime/lifecycle section from Agent Configuration and delete its composition module.
- [x] 1.2 Delete settings components used only for API Agent registration, registered-Agent cards, edit/delete, runtime control, tool trust, and memory management.
- [x] 1.3 Keep Agent Configuration limited to OnePiece and CLI provider panels, including supported configuration-tab preselection.

## 2. Remove management copy and test expectations

- [x] 2.1 Remove localization keys used only by the deleted management UI while preserving Agent configuration and shared session copy.
- [x] 2.2 Update component and Playwright tests to remove API Agent registration/management flows and assert that Agent Configuration has no registered-Agent or runtime controls.
- [x] 2.3 Remove obsolete localization guardrails and search active code plus non-archived OpenSpec artifacts for stale consolidation assumptions.

## 3. Verification

- [x] 3.1 Run focused Agent Configuration, settings registry, localization, and OnePiece E2E-compatible frontend tests.
- [x] 3.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 3.3 Run `cargo test`, `cargo check`, and `cargo clippy` for `src-tauri/Cargo.toml` to confirm retained runtime integrations still pass.
- [x] 3.4 Run `openspec validate remove-agent-management-page --strict` and `openspec validate --specs --strict`.
