## 0. Change validation and baseline

- [x] 0.1 Read `AGENTS.md`, `openspec/project.md`, all delta specs in this change, the current CLI frontend service/page, and `src-tauri/src/contexts/tooling/cli/` before editing production code.
- [x] 0.2 Run `openspec validate add-source-aware-cli-environment-management --strict` and correct the proposal/spec structure before implementation.
- [x] 0.3 Record the current CLI-related frontend, Rust, contract, Web/mock, and desktop test files that will be migrated.
- [x] 0.4 Add failing regression tests proving that the current selected version is not sent to the package operation and that equality is incorrectly treated as upgradeable.
- [x] 0.5 Confirm that no current active OpenSpec change owns the same CLI environment contract; resolve overlap before implementation rather than implementing competing models.

## 1. Shared operation and contract foundation

- [x] 1.1 Add `cli` to the shared operation-kind contract without changing existing operation kind values.
- [x] 1.2 Add optional operation `phase`, `completedUnits`, `totalUnits`, and `cancellable` fields to Rust and TypeScript contracts.
- [x] 1.3 Keep `queued`, `running`, `succeeded`, `failed`, and `cancelled` lifecycle statuses unchanged.
- [x] 1.4 Update operation persistence/mapping so absent progress fields remain backward-compatible.
- [x] 1.5 Add contract drift tests for the new operation fields and CLI operation result union.
- [x] 1.6 Add Tauri and Web/mock adapter conformance tests for queued, running, succeeded, failed, and cancelled CLI operations.

## 2. CLI domain model

- [ ] 2.1 Split the CLI domain into focused files that keep each production Rust file within the project size and dependency rules.
- [x] 2.2 Add validated value objects for tool, source, installation, action-plan, and bulk-plan ids while preserving existing wire `agentId` values.
- [x] 2.3 Replace flat distribution fields with `CliDistributionDefinition`, source capabilities, platform support, channel metadata, package references, and trust policy.
- [ ] 2.4 Replace transport-named `LifecycleEligibility::Wget` with source and transport concepts; do not model curl/wget/PowerShell as package sources.
- [x] 2.5 Add normalized installation, source confidence, PATH priority, executable status, and active-installation invariants.
- [x] 2.6 Add orthogonal discovery, authentication, readiness, compatibility, update, freshness, conflict, and overall-state values.
- [ ] 2.7 Add source-specific version catalogs and a single Rust version comparison path; remove lifecycle version comparison from React. (Rust half done: `domain/catalog.rs` binds every catalog to its tool and source, and `domain/version.rs` is the one comparison path. The React removal is task 10.8.)
- [x] 2.8 Add backend derivation of allowed actions, including the rule that current equals target produces no mutation.
- [x] 2.9 Add `CliActionPlan` invariants for exact source, target mode, revision, ten-minute expiry, snapshot fingerprint, single use, structured preview, and no fallback.
- [x] 2.10 Add `CliBulkActionPlan` and stable skipped-reason codes.
- [x] 2.11 Add pure domain tests for every status precedence, source capability, ordered/unordered version, action derivation, plan expiry, plan reuse, stale fingerprint, and bulk skip rule.

## 3. Application ports and use cases

- [x] 3.1 Replace the coarse package port with application-owned discovery, distribution, probe, environment repository, operation, clock/id, output sink, and mutation-coordinator ports.
- [x] 3.2 Implement bounded cached `list_cli_environments`.
- [x] 3.3 Implement async all-tool and targeted `refresh_cli_environments`, preserving unrelated snapshots on targeted refresh.
- [x] 3.4 Implement async `prepare_cli_action`, bounded `get_cli_action_plan`, and async `execute_cli_action`.
- [ ] 3.5 Implement async `prepare_cli_bulk_upgrade`, bounded `get_cli_bulk_action_plan`, and async `execute_cli_bulk_action`. (Bulk item outcomes are a placeholder: `run_bulk` discards the real five-state result and records the literal string "ran".)
- [x] 3.6 Implement async `run_cli_doctor`.
- [x] 3.7 Add typed application errors for unknown tool, unsupported source/action, invalid version, catalog unavailable, expired/stale/consumed plan, missing dependency, elevation, conflict, source unavailable, process, storage, and validation.
- [ ] 3.8 Implement per-tool and mutation-key coordination with at most two concurrent mutations and deterministic queuing. (The one-per-tool, one-per-key, cap-two policy exists only in `FakeCoordinator` under `#[cfg(test)]`; no production coordinator exists.)
- [ ] 3.9 Ensure cancellation releases every operation, plan, tool, and mutation-key reservation exactly once. (Only the happy path is covered. No test for duplicate cancellation, release on the error path, or plan state when an early `?` skips `finish_action_plan`.)
- [x] 3.10 Add deterministic application tests using port doubles; do not use SQLite, filesystem, network, Tauri, or live processes in these tests.

## 4. Discovery and environment fingerprint

- [x] 4.1 Preserve real PATH order and bounded known-location enumeration; do not recursively scan arbitrary disks.
- [x] 4.2 Canonicalize and deduplicate candidates while retaining safe diagnostics for canonicalization failure.
- [x] 4.3 Select the first runnable PATH candidate as active, or the first PATH candidate as active-but-broken when none are runnable.
- [x] 4.4 Add source confidence and stop treating a path heuristic as verified ownership.
- [x] 4.5 Correct version-bearing directory ordering, including NVM-style paths, with normalized version ordering rather than lexical ordering.
- [x] 4.6 Add bounded version probes with timeout, cancellation, per-stream output budget, and redaction.
- [x] 4.7 Compute the documented non-secret local-desktop environment fingerprint.
- [ ] 4.8 Add discovery tests for duplicate symlinks, PATH shadowing, broken first entries, permission failure, timeout, Windows shims, NVM ordering, and source confidence. (Pure-logic half done in domain and adapter unit tests; the PATH-shaped cases need the temporary-PATH desktop fixtures from task 12.10.)

## 4b. Duplicate-installation conflict contract

- [x] 4b.1 Split `active_installation_id` into `path_selected_installation_id` and `recommended_installation_id`; PATH order alone decides the first, probe results decide the second.
- [x] 4b.2 Add `severity`, `installation_ids`, `blocks_mutation`, `blocks_launch`, and a stable `reason_code` to every conflict.
- [x] 4b.3 Add the nine conflict kinds: duplicate launcher alias, PATH shadowing, broken PATH precedence, multiple installation sources, version divergence, ambiguous source ownership, environment PATH divergence, architecture mismatch, and stale launcher target.
- [x] 4b.4 Group platform launcher aliases into one logical installation so a single npm global install is not reported as several competing ones.
- [x] 4b.5 Withhold mutating actions when a conflict reports `blocks_mutation`.
- [x] 4b.6 Add Windows discovery tests for PATHEXT, case insensitivity, junction/symlink targets, user vs machine PATH, NVM symlinks, npm plus WinGet coexistence, and a broken first shim.
- [x] 4b.7 Add macOS discovery tests for `/opt/homebrew/bin` vs `/usr/local/bin`, arm64 vs x86_64, Finder vs login-shell PATH, and a changed Homebrew Cellar symlink target.
- [x] 4b.8 Add Linux discovery tests for system, local, user, and version-manager locations, update-alternatives, missing executable bit, noexec mounts, and desktop vs login-shell PATH.
- [x] 4b.9 Record whether the Agent Runtime launches CLIs by resolved absolute path or by bare command name; add a contract test if it already resolves, otherwise record a follow-up without widening this change.

## 5. Source adapters

- [ ] 5.1 Add a source registry assembled in bootstrap; the application layer must not select concrete adapters.
- [x] 5.2 Implement the npm source adapter with source-native catalog lookup, exact target propagation, install/upgrade/downgrade/reinstall/uninstall capability checks, explicit arguments, cancellation, and bounded output.
- [x] 5.3 Add npm fixture tests proving the selected target version reaches the exact process arguments and that package names come only from the backend catalog.
- [x] 5.4 Implement the Windows-only WinGet source adapter with WinGet-native version lookup, exact target arguments when supported, install, upgrade, uninstall, and dynamic repair preflight.
- [x] 5.5 Keep WinGet downgrade and reinstall disabled until a separate verified capability is added.
- [x] 5.6 Add WinGet fixture tests for exact ids, exact target versions, localized/unparseable output, missing WinGet, unsupported repair, elevation reporting, and source errors.
- [ ] 5.7 Implement the audited vendor installer adapter with platform-specific templates, HTTPS allowlist, bounded download, redirect policy, optional checksum/signature verification, temporary-file execution, cleanup, and no fallback. (`CliInstallerDownloader` has no production implementation, so bounded download, redirect policy, and checksum verification exist only in the test double.)
- [x] 5.8 On Windows, reject Bash-only vendor definitions unless a future explicitly approved definition and preflight support it.
- [ ] 5.9 Remove pipe-to-shell and `irm | iex` execution paths.
- [ ] 5.10 Implement detect-only source summaries and guidance for Homebrew, Bun, Volta, desktop, system, manual, and unknown sources.
- [ ] 5.11 Add source matrix tests proving no adapter borrows another source's catalog or capabilities. (No source matrix test exists; each adapter separately asserts its own catalog stamp, which does not prove the cross-adapter property.)
- [x] 5.12 Add a regression test proving vendor failure does not start npm.

## 6. Provider probes and readiness

- [x] 6.1 Move version, Doctor, and authentication probe definitions into the backend tool registry.
- [x] 6.2 Implement bounded Claude Code Doctor probing without persisting raw credential-like output.
- [x] 6.3 Implement bounded Codex login-status probing.
- [x] 6.4 Implement bounded OpenCode auth-list probing that returns only normalized authentication summary.
- [x] 6.5 Return explicit `unknown` Doctor/authentication state for Gemini CLI and Antigravity CLI until a safe documented non-interactive probe is implemented.
- [x] 6.6 Derive readiness from executable, authentication, compatibility, dependency, and Doctor results in the backend.
- [x] 6.7 Add parser fixtures for success, authentication required, expired/invalid state, unsupported command, timeout, malformed output, and secret redaction.

## 7. SQLite migration and repositories

- [ ] 7.1 Add versioned, additive migrations for `cli_environment_snapshots`, `cli_version_catalogs`, and `cli_action_plans` using the repository's next migration numbers.
- [ ] 7.2 Add indexes for plan expiry/state and any bounded snapshot/catalog query paths.
- [ ] 7.3 Store versioned JSON documents through explicit fallible row-to-domain mapping.
- [ ] 7.4 Implement atomic snapshot writes and atomic bulk-plan plus item-plan creation.
- [ ] 7.5 Implement atomic `draft -> executing` plan consumption before external execution admission.
- [ ] 7.6 Implement plan terminal-state persistence and bounded expired-plan maintenance.
- [ ] 7.7 Map a legacy `cli_tool_status` row to a stale new snapshot only when no authoritative snapshot exists.
- [ ] 7.8 Preserve the legacy table and unrelated data; stop writing legacy rows after cutover.
- [ ] 7.9 Add migration tests from an empty database, a representative old database, malformed legacy JSON, malformed new JSON, and interrupted plan states.
- [ ] 7.10 Add failure-injection tests proving atomic writes roll back fully.

## 8. Lifecycle execution, verification, and logging

- [ ] 8.1 Add structured CLI phases to refresh, planning, execution, verification, Doctor, and bulk operations.
- [ ] 8.2 Propagate operation cancellation into the process gateway and source adapters.
- [ ] 8.3 Enforce output budgets of 16 KiB per version-probe stream, 128 KiB total for Doctor/auth probes, and 1 MiB retained output per lifecycle operation.
- [ ] 8.4 Insert exactly one truncation marker while continuing safe child-process draining.
- [ ] 8.5 Redact output before operation storage, frontend delivery, and unified-log persistence.
- [ ] 8.6 Perform best-effort post-mutation detection after success, failure, timeout, or cancellation when safe.
- [ ] 8.7 Implement `verified`, `applied-unverified`, `changed-but-failed`, `no-change-failed`, and `cancelled` result semantics.
- [ ] 8.8 Never restore the pre-operation snapshot as a claimed rollback after an external effect may have occurred.
- [ ] 8.9 When post-detection fails, preserve last-known data only as stale and attach mutation/verification warnings.
- [ ] 8.10 Persist redacted operation context including operation id, Agent id, source, action, safe version, phase, exit/timeout/cancel state, elapsed time, and outcome.
- [ ] 8.11 Add tests for command success plus verification failure, command failure plus detected change, cancellation during download, cancellation during process execution, and log truncation/redaction.

## 9. Tauri commands and bootstrap

- [ ] 9.1 Add command DTOs separate from domain and SQLite row types.
- [ ] 9.2 Add one command file for list, refresh, prepare/get/execute single action, prepare/get/execute bulk action, and Doctor.
- [ ] 9.3 Make every variable-duration command return a stable operation id before process or network work completes.
- [ ] 9.4 Keep bounded cached reads and persisted-plan reads direct.
- [ ] 9.5 Add one command-safe error mapper with stable categories and optional diagnostic id.
- [ ] 9.6 Register commands centrally and assemble concrete repositories/adapters only in bootstrap.
- [ ] 9.7 Add serialized DTO and command-safe error tests.
- [ ] 9.8 Migrate all internal callers, then delete `list_cli_tools`, `refresh_cli_detections`, `install_cli_version`, `upgrade_all_cli_versions`, their obsolete DTOs, and obsolete background helpers before completing the change.
- [ ] 9.9 Update `src-tauri/ARCHITECTURE.md` with the source adapter, action-plan, external-effect sequencing, and partial-completion decisions.

## 10. Frontend service and runtime adapters

- [ ] 10.1 Add `src/types/cli-environment.ts` with the normalized snapshot, source, catalog, action, plan, bulk plan, diagnostic, and operation-result contracts.
- [ ] 10.2 Remove obsolete flat CLI status types from the broad Agent type file after all callers migrate.
- [ ] 10.3 Replace `CliToolService` with list, refresh, prepare/get/execute single action, prepare/get/execute bulk action, and Doctor methods.
- [ ] 10.4 Update `AgentService` composition without allowing components to call runtime-specific clients directly.
- [ ] 10.5 Implement every method in the Tauri adapter with typed error mapping.
- [ ] 10.6 Implement deterministic Web/mock operation transitions, cancellation, plans, snapshots, and bulk outcomes without native paths or side effects.
- [ ] 10.7 Add runtime adapter conformance tests for every new method and result variant.
- [ ] 10.8 Remove frontend semantic-version and lifecycle-action derivation utilities.
- [ ] 10.9 Ensure the selected source, channel, and target version are passed to `prepareCliAction`; execution must pass only plan id and expected revision.
- [ ] 10.10 Preserve cached data during background refresh and invalidate only affected query keys after terminal operations.

## 11. CLI Management UI

- [ ] 11.1 Rename/refactor the current Providers page into a focused CLI Management feature module and update the settings page registry.
- [ ] 11.2 Add the compact summary bar for ready, needs login, updates, conflicts, and broken counts.
- [ ] 11.3 Add search, status, source, and needs-action filters while preserving mounted filter and scroll state.
- [ ] 11.4 Render backend-derived overall state plus orthogonal executable, authentication, compatibility, source, and freshness badges.
- [ ] 11.5 Show active version, source, safe shortened path, update source/channel, and one backend-derived primary action per row/card.
- [ ] 11.6 Keep unrelated cards interactive while one operation runs; show queue/running state only on affected tools.
- [ ] 11.7 Add the details drawer with Overview, Installations, Diagnostics, and Operations tabs.
- [ ] 11.8 Add installation rows for path, version, source, confidence, PATH priority, executable state, and active/shadowed state.
- [ ] 11.9 Add the action-plan review dialog with source, version transition, structured command preview, network/elevation, preconditions, warnings, expiry, and no-fallback statement.
- [ ] 11.10 Prevent opening/executing a plan when target equals current.
- [ ] 11.11 Add the bulk-upgrade plan dialog with eligible and skipped sections and per-item execution outcome.
- [ ] 11.12 Add visible stale, refreshing, `applied-unverified`, and `changed-but-failed` states without clearing cached content.
- [ ] 11.13 Add cancel and safe-log-copy actions through the existing operation service.
- [ ] 11.14 Use shared semantic tokens/primitives, compact desktop density, no nested card-in-card styling, and both registered visual styles.
- [ ] 11.15 Add keyboard navigation, focus management, `aria-expanded`, `aria-controls`, accessible tooltips, non-color status cues, and restrained live-region announcements.
- [ ] 11.16 Add every new visible string to every locale registered in `src/i18n/supported-locales.ts`; keep date formatting locale-aware.

## 12. Frontend, Web, and desktop tests

- [ ] 12.1 Replace SSR/string-only CLI page tests with interaction tests using the shared service doubles.
- [ ] 12.2 Test selected old/new/current versions and assert exact `prepareCliAction` inputs.
- [ ] 12.3 Test that equal target renders current state and creates no mutation operation.
- [ ] 12.4 Test source-specific catalogs and detect-only manual installations.
- [ ] 12.5 Test per-tool busy state, queued operations, cancellation, stale display, and partial-completion warnings.
- [ ] 12.6 Test action-plan and bulk-plan review, expiry, stale-plan rejection, skipped reasons, and retry through a new plan.
- [ ] 12.7 Test drawer tabs, path truncation/copy, accessibility attributes, focus restoration, and keyboard use.
- [ ] 12.8 Keep i18n parity and hard-coded visible text guardrails passing.
- [ ] 12.9 Add Playwright coverage for filtering, details, plan review, bulk preview, operation progress, and Web/mock cancellation.
- [ ] 12.10 Add deterministic fake CLI and fake package-manager fixtures under the existing desktop fixture hierarchy for Windows, macOS, and Linux.
- [ ] 12.11 Add native desktop coverage for real Tauri IPC, temporary PATH discovery, duplicate/broken candidates, plan creation, fake mutation, verification, cancellation, and SQLite persistence across restart.
- [ ] 12.12 Ensure automated tests never invoke a real global npm, WinGet, vendor URL, provider login, credential store, or model API.
- [ ] 12.13 Report desktop results separately as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` for Windows, macOS, and Linux.

## 13. Documentation and cleanup

- [ ] 13.1 Update the English and Chinese user guides to describe source-aware management, action review, supported source capabilities, detect-only sources, Doctor, bulk preview, and partial-completion states.
- [ ] 13.2 Remove or correct statements that VaneHub only uses npm when the implementation supports another disclosed source.
- [ ] 13.3 Document that VaneHub never captures provider credentials and that login remains provider-owned.
- [ ] 13.4 Document vendor installer trust, no silent fallback, and platform limitations.
- [ ] 13.5 Remove obsolete CLI UI utilities, types, commands, source eligibility code, tests, and dead imports.
- [ ] 13.6 Keep every new production TS/TSX file at or below 300 physical lines and avoid adding lint exemptions.
- [ ] 13.7 Confirm no feature-local log file, direct React `invoke()`, raw shell interpolation, `irm | iex`, or silent source fallback remains.

## 14. Validation and completion evidence

- [ ] 14.1 Run `openspec validate add-source-aware-cli-environment-management --strict`.
- [ ] 14.2 Run `npm run lint:ci`.
- [ ] 14.3 Run `npm run test`.
- [ ] 14.4 Run `npm run test:coverage`.
- [ ] 14.5 Run `npm run coverage:policy:test`.
- [ ] 14.6 Run `npm run version:unit:test`.
- [ ] 14.7 Run `npm run contracts:check`.
- [ ] 14.8 Run `npm run architecture:check`.
- [ ] 14.9 Run `npm run build`.
- [ ] 14.10 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [ ] 14.11 Run `cargo check --workspace`.
- [ ] 14.12 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] 14.13 Run `npm run native:panic:check`.
- [ ] 14.14 Run `cargo test --workspace`.
- [ ] 14.15 Run `npx playwright test`.
- [ ] 14.16 Run `npm run desktop:unit:test`.
- [ ] 14.17 Run the relevant `npm run test:desktop:<layer>` suites and then `npm run test:desktop` on each available native platform.
- [ ] 14.18 Run `openspec validate --specs --strict`.
- [ ] 14.19 Record command results, platform-specific desktop status, and any genuine platform block in the implementation notes or PR description.
- [ ] 14.20 Mark this change complete only when all internal callers use the new contract, old CLI lifecycle APIs are removed, documentation is aligned, and no unchecked task is being hidden by a compatibility layer.
