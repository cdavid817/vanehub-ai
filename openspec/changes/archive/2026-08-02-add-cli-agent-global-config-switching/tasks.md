## 1. Contracts and persistence

- [x] 1.1 Add versioned TypeScript and Rust profile payloads, validation errors, drift status, apply request/result, and operation DTOs for `claude-code`, `opencode`, and `codex-cli` without secret-bearing response fields.
- [x] 1.2 Add the SQLite migration for `cli_config_profiles` and `cli_config_applied_state`, including stable-id uniqueness, per-Agent indexes, foreign-key behavior, and migration fixture assertions.
- [x] 1.3 Implement the profile and applied-state repository with create, list, read, update, duplicate, delete, import, and atomic applied-state operations plus repository tests.
- [x] 1.4 Add scoped credential-store operations keyed by Agent/profile id, including credential replacement, missing-credential repair state, compensated deletion, and redaction tests.
- [x] 1.5 Add the required pinned Rust TOML-editing and JSON5 parsing dependencies and record the supply-chain review required by project policy.
- [x] 1.6 Add a typed, versioned, secret-free preset catalog and Agent compatibility matrix for official providers, OpenRouter, DeepSeek, Zhipu GLM, Kimi/Moonshot, SiliconFlow, Alibaba Bailian, and Volcengine Ark, with schema and catalog-audit tests.

## 2. Native configuration domain

- [x] 2.1 Create `contexts/tooling/cli_config` domain and application modules with supported-Agent dispatch, normalized payload validation, ownership manifests, and structured errors.
- [x] 2.2 Implement standard user-level path resolution and read-only status inspection for Claude Code, OpenCode, and Codex CLI, returning safe resolved-path metadata.
- [x] 2.3 Implement managed-fragment canonicalization and fingerprints, including applied, detached, drifted, malformed, and missing-file states.
- [x] 2.4 Implement import-current extraction for all three adapters so managed credentials go directly to the credential store and unmanaged live values remain untouched.
- [x] 2.5 Implement a per-Agent apply coordinator that validates credentials, serializes concurrent requests, checks pre-write drift, prebuilds documents, and persists applied state only after successful projection.
- [x] 2.6 Implement sibling temporary-file replacement, original bytes/existence snapshots, file-permission handling, and compensation reporting for single-file and multi-file failures.
- [x] 2.7 Emit unified redacted logs and page-visible operation events for profile lifecycle, import, drift, apply, rollback, and repair outcomes.

## 3. CLI-specific projection adapters

- [x] 3.1 Implement the Claude Code adapter for `settings.json`, merging only owned provider/model environment keys and preserving unrelated environment entries, hooks, permissions, plugins, and top-level settings.
- [x] 3.2 Add Claude Code adapter tests for first apply, profile switching, stale owned-key removal, malformed JSON, external drift, atomic failure, and secret redaction.
- [x] 3.3 Implement the Codex adapter for syntax-aware updates to owned `config.toml` keys and the selected `model_providers.<id>` table while preserving MCP, project, comment, and unrelated provider content.
- [x] 3.4 Implement Codex credential strategies that preserve official authentication by default, require explicit confirmation before owning `auth.json`, and restore exact prior files on partial failure.
- [x] 3.5 Add Codex adapter tests for official and third-party profiles, TOML preservation, malformed input, drift, confirmation, two-file rollback, missing credentials, and secret redaction.
- [x] 3.6 Implement the OpenCode adapter for JSON5-compatible parsing, additive `provider.<id>` upsert, credential materialization, and global default-model selection without deleting unrelated providers or settings.
- [x] 3.7 Add OpenCode adapter tests for multiple providers, default-model changes, JSON5 input, malformed input, drift, atomic failure, and secret redaction.

## 4. Desktop and Web service adapters

- [x] 4.1 Add profile lifecycle, inspection, import, validation, drift resolution, and apply commands under `commands/tooling/cli_config`, map native errors to safe DTOs, and register every command in the Tauri invoke handler.
- [x] 4.2 Extend `AgentService` and shared frontend types with the global CLI configuration contract while keeping it independent from runtime selection and CLI launch-parameter APIs.
- [x] 4.3 Implement all new methods in `tauri-agent-client.ts` with exact command/payload mappings and update frontend/native contract tests.
- [x] 4.4 Implement deterministic preset-to-profile creation, profile storage, credential-presence simulation, drift resolution, and simulated apply results in `web-agent-client.ts` without filesystem claims or secret persistence.
- [x] 4.5 Add Tauri command and Web adapter tests for supported-id rejection, profile isolation, lifecycle parity, missing credentials, drift conflicts, concurrent apply serialization, and unchanged workflow/session state.

## 5. Agents page experience

- [x] 5.1 Decompose `agents-page.tsx` into focused card, runtime, and global-configuration components/hooks while keeping every modified React file below 300 lines.
- [x] 5.2 Add a supported-Agent global configuration panel showing profiles, applied/drift state, credential presence, resolved desktop paths, and explicit Web simulation state.
- [x] 5.3 Add a searchable compatible-preset chooser with official/common-provider grouping, preset version/deprecation metadata, a custom-provider entry, and an editable draft preview that never applies immediately.
- [x] 5.4 Add agent-specific create/edit/import/duplicate/delete forms with field-level validation and credential replace/remove controls that never repopulate secret values.
- [x] 5.5 Add apply preview and confirmation flows for drift resolution, affected paths, Codex `auth.json` ownership, observable progress, rollback outcome, and restart guidance.
- [x] 5.6 Keep runtime Agent/mode/Session actions visually and behaviorally separate, and add invariant tests proving a global apply never calls runtime Agent selection or changes active workflow state.
- [x] 5.7 Add Chinese and English translations, accessible labels, keyboard/focus behavior, empty/loading/error states, and narrow-layout coverage for the new Agents page controls.
- [x] 5.8 Add component and Playwright coverage for preset filtering and profile creation plus Claude Code, OpenCode, and Codex profile lifecycle, apply success, drift conflict, credential repair, rollback error, and Web simulation.

## 6. Verification and documentation

- [x] 6.1 Document the bundled preset list and upgrade policy, supported fields, exact global target files, additive versus exclusive semantics, credential exposure boundaries, drift choices, backup/rollback behavior, and restart requirements.
- [x] 6.2 Run `npm run lint`, `npm run test`, and `npm run build` and resolve all frontend failures.
- [x] 6.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` and resolve all native failures.
- [x] 6.4 Run the frontend/native contract checks and targeted Playwright Agents-page scenarios on the Web adapter.
- [x] 6.5 Run `openspec validate add-cli-agent-global-config-switching --strict` and `openspec validate --specs --strict`, then record implementation verification results in this change before archival.

## Implementation verification (2026-08-02)

- `npm run lint`: passed.
- `npm run test`: passed, 101 files and 345 tests.
- `npm run build`: passed, including TypeScript and lazy-chunk checks.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed, 985 unit tests plus 9 architecture tests; 3 fixture-only tests remained ignored by design.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml`: passed.
- Native/frontend command contract test: passed as part of the Rust suite.
- `npx playwright test tests/e2e/agent-global-config.spec.ts --reporter=line`: passed, 2 scenarios covering all three Agents and narrow-layout filtering.
- `openspec validate add-cli-agent-global-config-switching --strict`: passed.
- `openspec validate --specs --strict`: passed, 80 specifications.

## Blocker remediation verification (2026-08-02)

- Added regression coverage rejecting secret-like Claude advanced environment keys, Codex advanced TOML keys, and OpenCode header names before SQLite persistence.
- Added applied ownership snapshots plus migration 34 and verified that editing an applied profile cannot leave stale Claude-owned keys during the next switch.
- Added structured failed apply results with redacted errors, restoration status, affected paths, operation ids, and unified success/failure log metadata.
- Added credential compensation when drift import updates the credential store but profile persistence fails.
- Targeted native CLI configuration suite: passed, 24 tests.
- `npm run lint`: passed.
- `npm run test`: passed, 101 files and 345 tests.
- `npm run build`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed, 989 unit tests plus 9 architecture tests; 3 fixture-only tests ignored by design.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed.
- `npx playwright test tests/e2e/agent-global-config.spec.ts --reporter=line`: passed, 2 scenarios.
- `openspec validate add-cli-agent-global-config-switching --strict`: passed.
- `openspec validate --specs --strict`: passed, 80 specifications.

## 7. Dedicated Agent configuration experience

- [x] 7.1 Add a lazy-loaded Agent configuration settings page and navigation entry, keep a management entry on the Agents page, and preserve an originating supported Agent id without invoking runtime Agent selection.
- [x] 7.2 Build Claude Code, OpenCode, and Codex tabs with responsive applied/drift or Web-simulation summaries, resolved paths, and loading/empty/error states.
- [x] 7.3 Move the searchable, categorized compatible common-provider catalog and custom-provider entry to the dedicated page; quick-create SHALL open an editable draft and never apply immediately.
- [x] 7.4 Refactor profile create/edit into an accessible application dialog and replace browser prompts/confirmations for apply, delete, and destructive import choices with focused application dialogs.
- [x] 7.5 Add the selected Agent's profile collection and lifecycle actions with credential presence, operation progress, field errors, rollback outcome, restart guidance, and Chinese/English copy.
- [x] 7.6 Add component coverage for page navigation, Agent-tab isolation, provider search/category filtering, dialog lifecycle/focus, runtime-selection invariants, operation refresh, and narrow-layout behavior.
- [x] 7.7 Update targeted Playwright coverage and rerun frontend, native, contract, formatting, lint, build, and strict OpenSpec validation required by the project.

## Dedicated configuration UI verification (2026-08-02)

- `npm run lint`: passed.
- `npm run test`: passed, 103 files and 350 tests.
- `npm run build`: passed, including the dedicated lazy Agent-configuration chunk and chunk checks.
- Targeted Agent configuration component suites: passed, 4 files and 15 tests.
- `npx playwright test tests/e2e/agent-global-config.spec.ts --reporter=line`: passed, 2 scenarios covering navigation, all three Agent tabs, confirmation dialogs, and a 390px viewport.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed in an isolated target, 989 unit tests plus 9 architecture tests; 3 fixture-only tests ignored by design.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed in the isolated target.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed in the isolated target.
- Native/frontend command contract checks: passed as part of the Rust suite.
- `openspec validate add-cli-agent-global-config-switching --strict`: passed.
- `openspec validate --specs --strict`: passed, 80 specifications.
- `git diff --check`: passed; the remaining messages are Windows LF-to-CRLF conversion warnings for existing working-copy files.

## 8. CC Switch-inspired profile-first UI refinement

- [x] 8.1 Recompose the dedicated page around a compact Agent segmented switcher and focused add/import/refresh/search toolbar, removing the permanent side-by-side preset catalog without changing runtime selection or service contracts.
- [x] 8.2 Replace the large status summary with a lightweight applied/drift/Web-simulation strip that keeps resolved paths and last-apply context accessible in wide and narrow layouts.
- [x] 8.3 Redesign saved profiles as a single-column provider-card list with deterministic provider identity, endpoint/model metadata, credential and validation state, persistent applied-profile emphasis, and responsive lifecycle actions.
- [x] 8.4 Move compatible preset search/category selection into the large create dialog above the Agent-specific form, add a sticky action footer, and keep edit mode form-oriented without credential repopulation.
- [x] 8.5 Update Chinese/English copy and component tests for profile search, applied-state visibility, create-versus-edit behavior, dialog focus/keyboard handling, unchanged runtime selection, and narrow-layout affordances.
- [x] 8.6 Update targeted Playwright coverage for the profile-first flow and rerun lint, frontend tests/build, native contract checks, Rust formatting/check/test/clippy, and strict OpenSpec validation.

## CC Switch-inspired UI refinement verification (2026-08-02)

- `npm run lint`: passed.
- `npm run test`: passed, 103 files and 351 tests.
- `npm run build`: passed, including TypeScript and frontend chunk checks.
- Targeted Agent configuration component suites: passed, 2 files and 7 tests.
- `npx playwright test tests/e2e/agent-global-config.spec.ts`: passed, 2 scenarios covering the profile-first create/apply flow for all three CLIs and a 390px create dialog.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed in an isolated target, 989 unit tests plus 9 architecture tests; 3 fixture-only tests ignored by design.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed in the isolated target.
- `openspec validate add-cli-agent-global-config-switching --strict`: passed.
- `openspec validate --specs --strict`: passed, 80 specifications.
- `git diff --check`: passed; output contains only Windows LF-to-CRLF conversion warnings for existing working-copy files.

## 9. Read-only automatic local configuration discovery

- [x] 9.1 Add secret-free TypeScript and Rust discovery candidate/result DTOs plus service methods for selected-candidate import, keeping Tauri and Web adapters interface-compatible and Web discovery explicitly unavailable.
- [x] 9.2 Reuse the native Claude Code JSON, Codex TOML, and OpenCode JSON5 adapters for read-only discovery from standard user-level paths; return one exclusive candidate for Claude Code/Codex and every compatible OpenCode provider without changing profiles, credentials, applied state, or live files.
- [x] 9.3 Implement explicit discovered-candidate import that re-reads current live configuration, supports multi-candidate OpenCode import, moves credentials directly to the credential store, skips case-insensitive suggested-name conflicts, and reports imported/skipped outcomes without overwriting existing profiles.
- [x] 9.4 Add a compact dismissible discovery prompt below the Agent configuration toolbar with automatic page-open loading, manual refresh integration, safe candidate summaries, OpenCode multi-select import, parse/unavailable states, Chinese/English copy, and responsive keyboard-accessible behavior.
- [x] 9.5 Add native adapter, command-contract, Web parity, component, and Playwright coverage for exclusive discovery, multiple OpenCode providers, missing/malformed files, credential redaction, conflict skipping, explicit import, runtime-selection invariants, and narrow layouts.
- [x] 9.6 Update the CLI global configuration documentation and rerun lint, frontend tests/build, targeted Playwright, native contract checks, Rust formatting/check/test/clippy, strict change/spec validation, and `git diff --check`; record the results before archival.

## Read-only automatic discovery verification (2026-08-02)

- `npm run lint`: passed.
- `npm run test`: passed, 103 files and 354 tests. A pre-existing Session drag interaction received a local 10-second timeout after repeatedly passing alone but exceeding the global 5-second threshold under full-suite load.
- `npm run build`: passed, including TypeScript and lazy-chunk checks.
- Targeted Agent configuration and Web adapter suites: passed, 2 files and 13 tests.
- `npx playwright test tests/e2e/agent-global-config.spec.ts --reporter=line`: passed, 2 scenarios including Web discovery-unavailable behavior, dismissal, and a 390px viewport.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed in the isolated target.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed in the isolated target, 994 unit tests plus 9 architecture tests; 3 fixture-only tests ignored by design.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed.
- Native/frontend command contract checks: passed as part of the Rust suite.
- `openspec validate add-cli-agent-global-config-switching --strict`: passed.
- `openspec validate --specs --strict`: passed, 80 specifications.
- `git diff --check`: passed; output contains only Windows LF-to-CRLF conversion warnings for existing working-copy files.

## 10. CC Switch-inspired live synchronization

- [x] 10.1 Replace page-open candidate discovery as the primary workflow with a native startup synchronization result contract; retain manual import-current only as an explicit recovery action and keep Tauri/Web service interfaces aligned.
- [x] 10.2 Implement best-effort exclusive startup bootstrap for Claude Code and Codex: import one stable `default` profile only when the Agent has no profiles, persist credentials outside SQLite, record applied state without rewriting live files, and skip all later startups.
- [x] 10.3 Implement best-effort OpenCode startup synchronization that parses every compatible live provider and idempotently creates or updates profiles by provider id, preserves database-only/missing-live profiles, and compensates credential writes when persistence fails.
- [x] 10.4 Change exclusive apply coordination to automatically backfill externally edited managed fields and credentials into the leaving profile before switching, while retaining compare-before-write race protection, atomic projection, rollback, and redacted logging.
- [x] 10.5 Simplify the dedicated Agent configuration page by removing the discovered-candidate selection prompt, surfacing compact startup synchronization status/warnings, and keeping manual import-current, refresh, lifecycle, accessibility, and runtime-selection invariants intact.
- [x] 10.6 Add native startup/bootstrap, repository, credential compensation, exclusive backfill, OpenCode upsert/non-deletion, Web parity, component, and targeted Playwright regression coverage.
- [x] 10.7 Update documentation and run npm lint/test/build, targeted Playwright, Cargo fmt/test/check/clippy, native/frontend contract checks, strict change/spec validation, and `git diff --check`; record verification results before archival.

## CC Switch-inspired live synchronization verification (2026-08-02)

- `npm run lint`: passed.
- `npm run test`: passed, 103 files and 354 tests.
- `npm run build`: passed, including TypeScript and production bundle checks; only the existing Vite large-chunk advisory remains.
- Targeted Agent configuration and Web adapter suites: passed, 3 files and 14 tests.
- `npx playwright test tests/e2e/agent-global-config.spec.ts`: passed, 2 scenarios.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast`: passed, 997 unit tests plus 9 architecture tests; 3 fixture-only tests ignored by design.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed.
- Native/frontend command contract checks: passed as part of the Rust and frontend suites.
- `openspec validate add-cli-agent-global-config-switching --strict`: passed.
- `openspec validate --specs --strict`: passed, 80 specifications.
- `git diff --check`: passed; output contains only Windows LF-to-CRLF conversion warnings for existing working-copy files.
