## 1. Native domain and persistence

- [x] 1.1 Add the local/semantic mode to the Rust domain model, command DTOs, validation, and focused domain/DTO tests.
- [x] 1.2 Add the centralized SQLite migration and repository mapping so existing rows remain semantic and new workspaces default to local.
- [x] 1.3 Make configuration transitions generation-safe while preserving parsed manifests, chunks, symbols, FTS rows, and existing vectors.

## 2. Native indexing and retrieval behavior

- [x] 2.1 Make reconciliation and status calculation complete local workspaces as ready with zero pending/estimated external requests.
- [x] 2.2 Prevent local workspaces from preparing, claiming, or storing Embedding work and retain semantic confirmation behavior.
- [x] 2.3 Make `search_code` skip query Embedding in local mode without reporting keyword-only degradation, with workspace-scoping tests.

## 3. Frontend contracts and settings

- [x] 3.1 Extend TypeScript types, normalization, AgentService, Tauri adapter, and Web/mock adapter with mode parity and deterministic transitions.
- [x] 3.2 Add an accessible local/semantic selector and conditional Embedding guidance to the workspace configuration/status UI.
- [x] 3.3 Add semantically aligned translations for every registered locale and update component/contract tests.

## 4. End-to-end verification

- [x] 4.1 Extend Playwright coverage for local ready behavior and semantic confirmation without external calls in Web mode.
- [x] 4.2 Run frontend lint, tests, build, and Playwright verification.
- [x] 4.3 Run Rust fmt, Clippy, tests, check, change validation, and strict main-spec validation; record results.

## Completed baseline validation

- `npm run lint:ci`, `npm run test` (685 tests), and `npm run build` passed.
- Code-index Playwright coverage passed both standalone and within the full suite. The full suite reached 81/83 on the final constrained-machine run; the two unrelated timeout cases were outside this change, and the Agent profile case passed when rerun alone.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, strict Clippy, all 1,791 Rust library tests, integration tests, and `cargo check` passed using an isolated target directory because the normal target was held by a running desktop process.
- `openspec validate add-local-code-index-mode --strict` and `openspec validate --specs --strict` passed with all 92 main specs valid.
- Automatic-policy follow-up validation passed: `npm run lint:ci`, all 689 Vitest tests, production build, coverage and policy checks, version tests, and contract checks.
- Full Playwright passed 84/84 on isolated port 5190; Rust fmt, strict Clippy, 1,783 library tests plus integration suites, and `cargo check` passed with an isolated target directory.
- The updated change and all 92 main specs passed strict OpenSpec validation.

## 5. Automatic indexing policy and persistence

- [x] 5.1 Add the disabled/local/semantic automatic policy to the retrieval domain, SQLite configuration, Tauri DTOs, TypeScript contracts, and both frontend adapters.
- [x] 5.2 Preserve existing workspace configuration as explicit state while applying the automatic policy only to newly discovered projects.
- [x] 5.3 Add migration and repository tests for the safe disabled default, existing-workspace preservation, and canonical-root reuse.

## 6. Session-driven workspace discovery

- [x] 6.1 Add native post-session-creation orchestration for local OnePiece project folders without coupling the sessions domain directly to retrieval infrastructure.
- [x] 6.2 Make automatic registration and reconciliation asynchronous, idempotent by canonical root, generation-safe, and non-blocking for session success.
- [x] 6.3 Exclude remote sessions, scope Git worktrees to their actual folder, and route failures through unified logging and workspace status.
- [x] 6.4 Mirror session-driven discovery and deterministic transitions in the Web/mock adapter without filesystem or network access.

## 7. Status contracts and OnePiece UI

- [x] 7.1 Extend status contracts to distinguish local index readiness from semantic configuration, confirmation, Embedding, and degradation.
- [x] 7.2 Add the accessible three-state automatic policy to OnePiece settings and conditionally expose the effective Embedding source/model.
- [x] 7.3 Update the workspace dashboard for automatic origin, effective mode, local/semantic state, detailed progress, overrides, rebuild, disable, delete, and advanced manual pre-indexing.
- [x] 7.4 Add a compact current-session code-index indicator through the AgentService boundary.
- [x] 7.5 Update every registered locale and add contract, component, adapter, and status-transition tests.

## 8. Automatic indexing verification

- [x] 8.1 Add native tests for post-session discovery, duplicate-session reuse, disabled policy, existing overrides, non-blocking failure, remote exclusion, and worktree scoping.
- [x] 8.2 Add Playwright coverage for creating local and semantic OnePiece sessions, automatic workspace appearance, progress, local-ready behavior without Embedding, and semantic configuration/confirmation.
- [x] 8.3 Run all repository-mandated frontend, Rust, Playwright, contract, coverage-policy, and strict OpenSpec validation commands and append the new results above.

## 9. UI ownership refinement

- [x] 9.1 Add a dedicated OnePiece page to CLI Parameter Management and move the retrieval policy and conditional Embedding parameters there.
- [x] 9.2 Remove retrieval and workspace code-index controls from Agent Configuration.
- [x] 9.3 Add a session-scoped code-index tab to the information panel with current-workspace status and management actions.
- [x] 9.4 Update all locales, component tests, Playwright coverage, and strict validation results for the relocated UI.

UI ownership refinement validation:
- `npm run lint:ci`, `npm run test` (688 tests), `npm run build`, coverage/policy, version, and contract checks passed.
- Full Playwright passed 83/83 on isolated port 5194, including CLI Parameter Management and session-scoped local index flows.
- Rust formatting, strict Clippy, 1,783 library tests plus integration suites, and `cargo check` passed with the isolated Cargo target.

## 10. Index status ownership

- [x] 10.1 Remove global index status and rebuild controls from OnePiece CLI parameter management.
- [x] 10.2 Keep workspace index status, progress, and management exclusively in the active session information panel.
- [x] 10.3 Update component and Playwright coverage and rerun strict validation.

Index status ownership validation:
- `npm run lint:ci`, all 686 Vitest tests, and `npm run build` passed.
- The focused OnePiece Playwright suite passed 2/2 on isolated port 5195.
- The change and all main specs passed strict OpenSpec validation.

## 11. OnePiece parameter presentation

- [x] 11.1 Restyle OnePiece retrieval mode and Embedding controls to match the managed CLI parameter cards.
- [x] 11.2 Remove stale index-status wording and preserve responsive, accessible controls.
- [x] 11.3 Update component and Playwright tests and rerun strict validation.

OnePiece parameter presentation validation:
- `npm run lint:ci`, all 686 Vitest tests, and `npm run build` passed.
- OnePiece desktop flows and the 390px English minimal-theme parameter layout passed focused Playwright coverage.
- The change and all main specs passed strict OpenSpec validation.

## 12. Non-blocking workspace deletion

- [x] 12.1 Move workspace-index SQLite deletion onto the Tauri blocking thread pool.
- [x] 12.2 Close the information-panel delete confirmation immediately while retaining pending and error feedback.
- [x] 12.3 Add native and component regression coverage and rerun strict validation.

Non-blocking deletion validation:
- `npm run lint:ci`, all 687 Vitest tests, production build, coverage and policy checks, version tests, and contract checks passed.
- Rust formatting, strict Clippy, all 1,798 library tests plus integration suites, architecture enforcement, and `cargo check` passed.
- Full Playwright reached 82/83 under parallel load; the unrelated Agent configuration navigation timeout passed when rerun alone.
- The change and all 92 main specs passed strict OpenSpec validation.
