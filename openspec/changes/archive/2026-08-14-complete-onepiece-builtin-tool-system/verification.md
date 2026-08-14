## Implementation verification

Verified on 2026-08-14 in the `feat/builtin-tool-system` worktree after production composition and manual approval routing were completed.

### Production-composition evidence

- The native bootstrap constructs fixed handlers for Browser, Web, code execution, OCR, Artifact, delegation, and ChangeSet apply, with unavailable adapters and stable readiness reasons when a backend cannot be admitted.
- `manual_apply_forces_once_approval_and_reuses_bound_authority` proves manual ChangeSet apply enters the unified pending-approval path even when ordinary policy evaluates to Allow, rejects a wrong-session approval delivery, reuses the original input-bound witness, and executes exactly once.
- `manual_apply_denial_is_terminal_without_invoking_the_backend` proves denial reaches a terminal operation without invoking the apply backend.
- `production_handoff_adapter_preserves_owned_page_and_token_checks` proves the production Browser handoff adapter preserves session/generation/page ownership, rejects a forged ownership token, and routes begin/resume through the shared browser operation service.
- Manual delegation/apply commands are zero-decision Tauri adapters. `ManualNativeToolControl` resolves the canonical local workspace from the owning session, verifies ChangeSet-to-session ownership through persisted lineage, and dispatches through the same `NativeToolDispatcher` and Permissions approval broker used by model-originated calls.
- Delegation readiness, production CLI protocol adapters, ChangeSet preflight/apply/rollback/recovery, and fixed-registry readiness behavior remain covered by their domain and infrastructure suites in the full Rust run.

### Verification matrix

- `npm run lint:ci`: passed.
- `npm run test`: 204 files and 917 tests passed.
- `npm run build`: passed; 16 lazy frontend chunks verified.
- `npm run test:coverage`: 204 files and 917 tests passed; statements 69.01%, branches 65.57%, functions 64.90%, lines 72.78%.
- `npm run coverage:policy:test`: 5 tests passed.
- `npm run version:unit:test`: 9 tests passed.
- `npm run contracts:check`: 3 tests passed.
- `npx playwright test`: 92 tests passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 2532 library tests passed with 15 fixture-only tests ignored; 15 permission-hook tests, 24 architecture tests, and all MCP integration suites passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `openspec validate complete-onepiece-builtin-tool-system --strict`: passed.
- `openspec validate --specs --strict`: 100 main specifications passed.

### Archive readiness

All implementation tasks and required validations are complete. The change remains unarchived intentionally and is ready for the governed `openspec archive complete-onepiece-builtin-tool-system` workflow, followed by the required archive-index update command.
