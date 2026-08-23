## 0. Change preflight and baseline reconciliation

- [x] 0.1 Read `AGENTS.md`, `openspec/project.md`, `openspec/specs/cli-parameter-management/spec.md`, this change's `proposal.md`, `design.md`, delta spec, and all currently active changes that modify `cli-parameter-management`.
- [x] 0.2 Run `openspec validate upgrade-cli-parameter-management --strict` before implementation and correct only this change's artifacts if the validator identifies delta-format or baseline-name issues.
- [x] 0.3 Search active and archived changes for concurrent edits to CLI lifecycle, Agent Policies, provider invocation builders, settings navigation, generated contracts, or SQLite CLI settings; record concrete conflicts in the implementation notes before editing code.
- [ ] 0.4 Capture the current behavior with focused tests before refactoring: current profile listing, custom-text control behavior, chat preview, persistence reload, policy-owned field rejection, and provider argv placement.
- [x] 0.5 Confirm the current names and ownership of the existing CLI lifecycle read model, unified logging API, database migration registry, Tauri command registration, and Web/mock storage namespace; adapt the paths in this plan to repository reality without creating a parallel subsystem.
- [x] 0.6 Create an implementation evidence section at the end of this file or in the repository's established change-evidence location; record commands, results, platform, and relevant artifact paths as tasks are completed.

## 1. Characterize and lock down current regressions

- [ ] 1.1 Add a component test proving that selecting Custom with no text does not immediately submit an empty value and that save remains disabled until the value is valid.
- [ ] 1.2 Add a component test proving that transient custom input for `claude-code:model` cannot appear in `codex-cli:model` or another CLI that reuses the `model` parameter id.
- [ ] 1.3 Add a test proving that the existing page's preview differs between `chat` and `interactive` scopes for at least one scope-specific parameter.
- [ ] 1.4 Add a test proving that a preview token containing whitespace remains one argv token and is not asserted through a joined shell-like string.
- [ ] 1.5 Add a test proving that a backend structured field error is mapped to the correct parameter without parsing an English message.
- [x] 1.6 Add a native regression test for the existing Codex `model_reasoning_effort` provider-specific renderer before replacing parameter-id branching.
- [x] 1.7 Add native tests proving that policy-owned and runtime-reserved parameters cannot be persisted or emitted by the user profile path.

## 2. Establish the native CLI-parameter subdomain

- [ ] 2.1 Replace the monolithic `src-tauri/src/contexts/tooling/cli_parameters.rs` with `src-tauri/src/contexts/tooling/cli_parameters/` using the repository's bounded-context conventions.
- [x] 2.2 Create focused domain modules for agent ids, parameter ids, selection envelopes, definition/control types, render strategies, compatibility, dependency rules, diagnostics, profiles/revisions, validation, and deterministic rendering.
- [x] 2.3 Create application modules for list profiles, preview draft, save profile, reset profile, and resolve launch segments.
- [x] 2.4 Create infrastructure modules for canonical registry loading, SQLite repository/migration, cached CLI lifecycle compatibility access, and Web-contract generation support where native ownership requires it.
- [x] 2.5 Expose only stable DTOs and use cases through `src-tauri/src/contexts/tooling/api.rs`; do not let `agent_runtime`, Tauri commands, or provider builders import CLI-parameter infrastructure or persistence types.
- [x] 2.6 Keep `tooling` as the owning bounded context; do not add a new top-level context or duplicate CLI lifecycle state.
- [x] 2.7 Keep each new Rust module focused and testable; avoid replacing one monolithic file with another monolithic `mod.rs`.

## 3. Implement explicit selection semantics

- [x] 3.1 Add a versioned selection envelope with explicit `inherit` and typed `value` variants; support the required scalar and bounded-list value kinds without `serde_json::Value` in the domain model.
- [x] 3.2 Remove global magic handling where string `default`, boolean false, or an empty list means inheritance.
- [x] 3.3 Ensure a provider value literally named `default` can be represented and rendered when the registry declares it as a real value.
- [x] 3.4 Add normalization and validation for text length, trimming policy, allowed pattern, disallowed control characters, disallowed bidirectional formatting characters, item count, duplicate handling, and path-list normalization.
- [x] 3.5 Add domain tests for every selection kind, serialization round trip, inherited state, explicit false, literal `default`, malformed envelope, and type mismatch.
- [x] 3.6 Ensure transport DTO conversion rejects unknown variants and returns a structured error rather than falling back to a default.

## 4. Build the canonical capability registry

- [x] 4.1 Add one native-owned, versioned canonical registry source under the CLI-parameter subdomain, using the repository's existing JSON/Serde facilities and no new general-purpose configuration parser.
- [x] 4.2 Define registry metadata for ownership, category, maturity, control kind, localized keys, inherited/default semantics, launch scopes, risk, render strategy, argument slot, constraints, compatibility, dependencies, conflicts, option source, and audit record.
- [x] 4.3 Implement declarative render strategies for boolean flags, positive/negative tri-state flags, flag-value pairs, repeated flag-value pairs, joined values, provider configuration key/value tokens, and bounded environment mappings only if an audited parameter actually needs them.
- [x] 4.4 Implement provider argument slots sufficient for global options, subcommand options, fresh-chat options, resume options, and interactive options without exposing runtime-owned prompt/session/output-protocol tokens.
- [x] 4.5 Add registry bootstrap validation for duplicate agent/parameter ids, duplicate or unsafe flag mappings, invalid defaults, unsupported control-render combinations, missing localization keys, invalid constraints, contradictory compatibility ranges, dependency cycles, conflicts with reserved arguments, and policy-owned entries in the editable view.
- [x] 4.6 Fail development/test startup and contract generation on an invalid registry; production profile loading must return a safe structured catalog error rather than panic.
- [x] 4.7 Add deterministic catalog versioning based on an explicit schema/catalog version, not a random build identifier.
- [x] 4.8 Add domain and snapshot tests covering registry parse, validation failures, stable ordering, and deterministic rendering.

## 5. Audit and implement provider catalog v2

- [ ] 5.1 Re-audit every retained parameter against the official provider reference and record source id, review date `2026-08-22` or the actual implementation review date, reviewed version/document state, and verification status in the canonical registry.
- [x] 5.2 Claude Code: retain custom model values; implement audited effort values with model-dependent guidance; add ordered fallback models; replace independent Chrome booleans with inherited/enable/disable tri-state; add setting sources; version-gate accessibility and newly documented effort values; scope `bare` to verified scripted/chat use.
- [x] 5.3 Claude Code: keep prompt, system prompt, tool allow/deny lists, additional-directory authority, permission mode, session identity, output format, and dangerous bypass controls outside the editable registry.
- [x] 5.4 Codex CLI: implement model as bounded custom text; replace the hard-coded global reasoning list with the accepted current configuration-reference baseline; render reasoning through declarative `--config` metadata; preserve unsupported legacy values only as repair diagnostics.
- [x] 5.5 Codex CLI: keep profile as bounded custom text; add local provider only with its declared dependency; keep approval and sandbox settings outside the editable registry; defer unstable feature toggles unless separately audited.
- [x] 5.6 Gemini CLI: implement verified model/debug/accessibility entries; add ordered/repeatable extensions with exclusive `none`; add include directories with deduplication and maximum five; return non-destructive warnings for missing directories.
- [x] 5.7 Gemini CLI: keep prompt, resume/session operations, output format, approval, allowed-tool, sandbox, and bypass behavior runtime-owned or policy-owned.
- [x] 5.8 OpenCode: implement model with `provider/model` guidance and optional cached suggestions; implement variant as provider/model-dependent custom or dynamic values rather than one global enum; correct thinking copy to mean showing thinking blocks; exclude automatic approval.
- [x] 5.9 Antigravity CLI: retain only independently verified model, effort, and agent mappings; do not infer any flag from Gemini CLI; omit any candidate lacking a reliable current reference and surface its audit status only in developer evidence.
- [x] 5.10 Add provider-specific catalog tests that assert exact flag spelling, value grammar, scope, placement, compatibility, reserved exclusions, policy exclusions, and representative argv tokens.

## 6. Add compatibility, dependency, and diagnostic evaluation

- [x] 6.1 Reuse the existing CLI lifecycle/detection read model for active executable path, normalized version, platform, installation state, and conflict state; do not create a second detector.
- [x] 6.2 Define behavior for missing, unknown, malformed, prerelease, too-old, and too-new versions and cover each branch with tests.
- [x] 6.3 Evaluate compatibility without spawning a provider process during page load, field edit, preview, save, or runtime resolution.
- [x] 6.4 Implement registry-declared `requires`, `conflictsWith`, and bounded implication rules with cycle-safe deterministic evaluation.
- [x] 6.5 Return structured diagnostics with stable code, severity, agent id, optional parameter id, message key, safe details, remediation action, and blocking/non-blocking classification.
- [x] 6.6 Add diagnostics for missing executable, multiple-installation conflict, unknown version, unsupported parameter/value, missing dependency, conflict, malformed legacy row, unknown legacy id, missing directory, revision conflict, and catalog-version conflict.
- [x] 6.7 Route persisted native diagnostics through the unified logging service with redaction; do not write feature-local logs or raw prompts, credentials, tokens, session ids, or secret-bearing environment values.

## 7. Migrate persistence and add optimistic concurrency

- [x] 7.1 Add the required SQLite migration using the repository's existing migration mechanism; add profile metadata/revision/catalog/selection-schema fields or tables without destructive deletion of existing rows.
- [x] 7.2 Preserve the current `enabled` column only as a compatibility field if removal would make migration unsafe; stop using it as the semantic source of inherited versus explicit selection state.
- [x] 7.3 Implement a repository transaction that validates the complete candidate profile, compares `expectedRevision` and accepted catalog version, replaces profile selections atomically, increments revision once, and returns the committed profile.
- [x] 7.4 Implement reset with the same revision guard and one atomic revision increment.
- [x] 7.5 Implement idempotent legacy conversion for valid strings, booleans, lists, and historical `default` values only where the old catalog makes the intended meaning unambiguous.
- [x] 7.6 Quarantine malformed, unknown, removed, and incompatible legacy rows from rendering while preserving enough safe metadata for repair diagnostics.
- [x] 7.7 Rewrite a legacy profile into the current selection schema on its first successful save or reset; do not require a destructive eager migration at application startup.
- [x] 7.8 Apply the same versioned envelope, revision, migration, and conflict behavior to the Web/mock storage adapter under its existing namespace.
- [x] 7.9 Add SQLite and Web/mock tests for fresh schema, legacy schema, repeated migration, partial profile, malformed JSON, unsupported value, stale revision, stale catalog, transaction rollback, reset, and restart/reload restoration.

## 8. Implement application use cases and Tauri DTO boundary

- [x] 8.1 Define frontend-facing DTOs for profile summary, executable status, parameter definition, explicit selection, compatibility, diagnostic, saved previews, draft-preview input/output, save input/output, and reset input/output.
- [x] 8.2 Keep DTOs free of infrastructure types and convert all domain errors to stable structured command errors at the Tauri boundary.
- [x] 8.3 Extend `src/services/agent-service.ts` with list, preview, save, and reset contracts; preserve the existing service-only dependency rule for React.
- [x] 8.4 Add or update focused Tauri command files; register commands through the existing bootstrap path and keep command functions as thin DTO adapters.
- [x] 8.5 Implement `previewCliParameterProfile` as a read-only use case accepting agent id, scope, catalog version, and complete draft selections; prove it does not mutate persistence or revision.
- [x] 8.6 Implement latest-safe response identifiers or request correlation fields needed for the frontend to discard stale preview responses without making the domain depend on UI timing.
- [ ] 8.7 Ensure list/save/reset continue to provide practical compatibility for existing frontend call sites during the refactor, then remove obsolete DTOs once both adapters and all consumers migrate.
- [x] 8.8 Add application tests for success, structured field error, dependency conflict, unsupported selection, stale revision, stale catalog, missing CLI, and non-mutating preview.

## 9. Publish runtime resolution and migrate provider builders

- [x] 9.1 Add a narrow tooling API such as `resolve_cli_launch_segments(agent_id, scope, message_overrides, execution_context)` that returns validated user-profile segments and safe diagnostics without exposing persistence.
- [x] 9.2 Preserve precedence: message-level ordinary override, explicit compatible VaneHub profile value, inherited provider behavior; keep all policy-governed values on the policy path.
- [x] 9.3 Migrate every managed provider invocation builder to consume resolved segments from the tooling API and place them in the provider grammar's declared slots.
- [x] 9.4 Remove provider-specific CLI-parameter reads, duplicate validation, duplicate `default` interpretation, and parameter-id renderer branches from `agent_runtime`.
- [x] 9.5 Migrate Agent Terminal launch construction to request only `interactive` segments through the tooling API.
- [x] 9.6 Ensure fresh-chat and resume builders preserve prompt transport, structured output, session id, stdin marker, and runtime-owned ordering exactly.
- [x] 9.7 On launch, omit known-incompatible saved selections and associate their structured diagnostic with the operation; do not fail an otherwise valid launch solely because a quarantined ordinary override exists unless provider grammar requires failure.
- [x] 9.8 Add table-driven runtime tests for all five providers across interactive, fresh chat, and resume where supported, including custom values, whitespace tokens, inherited state, policy projection, incompatible selections, and reserved arguments.

## 10. Generate the TypeScript catalog contract and maintain adapter parity

- [x] 10.1 Add a deterministic repository script that generates the frontend registry contract from the canonical native registry into a clearly marked generated file.
- [x] 10.2 Add generated-file headers and repository documentation stating that the artifact must not be hand-edited.
- [x] 10.3 Add the generator verification to `npm run contracts:check` or the repository's established contract-check composition so CI fails on drift.
- [ ] 10.4 Delete the hand-maintained frontend provider catalog and duplicate renderer after the generated contract and service preview paths replace all consumers.
- [x] 10.5 Update `src/services/tauri-agent-client.ts` for the new DTOs and commands.
- [x] 10.6 Update `src/services/web-agent-client.ts` to use the generated contract and browser storage while clearly remaining a non-launching mock adapter.
- [x] 10.7 Add TypeScript contract tests for deterministic generation, Rust/TypeScript registry parity, Web preview parity on representative fixtures, and reserved/policy exclusions.

## 11. Build the isolated frontend draft engine

- [ ] 11.1 Create a focused CLI-parameter page directory and split production TS/TSX files so each remains at or below the repository's 300-line rule without adding an exemption.
- [ ] 11.2 Implement `useCliParameterDrafts` or an equivalent hook keyed by both agent id and parameter id, tracking baseline revision, baseline catalog version, baseline selections, draft selections, transient custom inputs, dirty ids, blocking diagnostics, and server-conflict state.
- [ ] 11.3 On refetch, replace an unmodified draft; retain a dirty draft when revision is unchanged; mark conflict and disable save when a newer revision arrives.
- [ ] 11.4 Make every field fully controlled; selecting Custom changes editor mode only; clearing custom input produces local validation without inserting an invalid transport value.
- [ ] 11.5 Preserve all per-CLI drafts when switching active CLI and expose dirty/error/warning counts for inactive rail items.
- [ ] 11.6 Integrate the shared unsaved-change guard for any dirty profile.
- [ ] 11.7 Debounce draft preview, attach request identity, apply latest-request-wins, preserve the previous valid preview during refresh/failure, and cancel or ignore obsolete responses on agent/scope change.
- [ ] 11.8 Map structured backend diagnostics by stable code and parameter id; delete English-message regular expressions.
- [ ] 11.9 Add hook tests for isolation, refetch merge, revision conflict, catalog conflict, custom input, scope switch, stale preview, discard, reset, and save success/failure.

## 12. Redesign Settings → CLI Parameters UI

- [ ] 12.1 Replace the current mixed Agent list with an external-CLI-only branded rail for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI; add a concise link to Agent Configuration for OnePiece.
- [ ] 12.2 Show cached active version, executable path, missing/unrunnable/conflict status, dirty count, warning count, and error count in the rail/header; link operational problems to CLI Management.
- [ ] 12.3 Add explicit Chat and Interactive scope controls with accessible names and persistent selection while switching CLIs.
- [ ] 12.4 Add search plus All, Modified, Warnings, Unsupported, and Advanced filters; search localized label/description/option text and literal flag/stable id.
- [ ] 12.5 Group fields into registry categories such as Model & reasoning, Experience & accessibility, Context & extensions, Runtime, and Diagnostics; omit empty groups.
- [ ] 12.6 For each field, show label, literal flag or render summary, localized description, Inherit state, compatibility/maturity badges, VaneHub source statement, inline validation, and dependency/conflict guidance.
- [ ] 12.7 Implement accessible controls for enum, custom text, tri-state, boolean, multi-enum, ordered string list, and path list; make reorder/remove/directory actions keyboard operable.
- [ ] 12.8 Add a policy notice that approval, automatic approval, permissions, sandboxing, and dangerous bypass behavior are configured in Agent Policies; provide navigation without duplicating controls.
- [ ] 12.9 Render preview as individual tokens grouped by segment where useful; add Copy argv JSON and optional display-only escaped copy; never present a joined string as an executable command.
- [ ] 12.10 Keep the preview sticky on supported wide layouts and place it after controls at narrow widths without horizontal page overflow.
- [ ] 12.11 Add Restore Inherited Values, Discard Draft, and Save Profile actions in the sticky action area; do not add Save All unless a future spec defines cross-profile atomicity.
- [ ] 12.12 Display legacy, compatibility, lifecycle, and preview diagnostics with repair/navigation actions and non-color-only severity labels.
- [ ] 12.13 Preserve focus during filtering where possible; use polite live regions for refresh/preview status and an assertive alert for stale-revision conflict.
- [ ] 12.14 Use shared semantic tokens and compact settings primitives for both `futuristic` and `minimal`; do not introduce inline styles, CSS modules, a second UI library, or nested decorative card stacks.

## 13. Localization and documentation

- [ ] 13.1 Add every new visible string to all registered locale files, not only `zh-CN` and `en`; retain literal flags, paths, versions, and stable ids where translation would be incorrect.
- [ ] 13.2 Add localized labels and detailed descriptions for every exposed registry parameter, option, category, maturity state, compatibility state, diagnostic code, filter, preview action, and repair action.
- [ ] 13.3 Add a contract test that every registry localization key resolves in every registered locale.
- [ ] 13.4 Update the user guide to explain scope, Inherit versus explicit value, precedence, safe token preview, policy ownership, version compatibility, legacy repair, Web/mock limitations, and when changes affect processes.
- [ ] 13.5 Generate or update the documented provider parameter matrix from the canonical registry where practical; do not maintain another manual parameter table that can drift.
- [ ] 13.6 Document the registry update/audit workflow for future CLI releases, including official-source review, compatibility changes, generation, smoke fixtures, and contract checks.
- [ ] 13.7 Update any screenshots or help anchors affected by the settings-page information architecture.

## 14. Automated verification

- [x] 14.1 Run focused Rust domain/application/repository/runtime tests throughout implementation and keep each task's evidence.
- [ ] 14.2 Run focused Vitest component/hook/service/adapter tests throughout implementation.
- [x] 14.3 Run `npm run contracts:check` after registry generation and prove a deliberately stale generated artifact is detected by the test, then restore the generated output.
- [x] 14.4 Run `npm run architecture:check` and fix any cross-context, direct-invoke, service-boundary, generated-contract, or file-size violation without adding exemptions.
- [ ] 14.5 Run `npx playwright test` because settings behavior and accessibility change; retain failure artifacts if blocked by the environment.
- [x] 14.6 Run `npm run desktop:unit:test` and `npm run test:desktop:desktop-settings-persistence` using fixed CLI fixtures and no real model calls.
- [ ] 14.7 Run `npm run test:desktop:desktop-cli-terminal` to verify interactive profile projection and provider token placement through a real desktop test client.
- [ ] 14.8 Run the full desktop suite with `npm run test:desktop` when the local platform supports it; report other platforms as `NOT RUN` rather than extrapolating.
- [ ] 14.9 Add or update Windows, macOS, and Linux CI fixture coverage for path-list normalization, executable status, argv tokens, and persistence; report each platform as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`.
- [ ] 14.10 Run accessibility checks for keyboard operation, focus behavior, labels, live regions, contrast under both themes, and narrow-width reflow.
- [ ] 14.11 Run legacy migration fixtures against a copy of representative existing SQLite rows; verify no destructive loss and one-time rewrite on successful save/reset.
- [x] 14.12 Verify that no test, log, preview, snapshot, or persisted diagnostic contains prompts, credentials, API tokens, session ids, or unredacted secret-bearing environment values.

## 15. Final repository gates and change completion

- [x] 15.1 Run `npm run lint:ci`.
- [x] 15.2 Run `npm run test`.
- [x] 15.3 Run `npm run test:coverage` and resolve coverage-policy failures in changed areas.
- [x] 15.4 Run `npm run coverage:policy:test`.
- [x] 15.5 Run `npm run version:unit:test` if registry/catalog version behavior touches version utilities or contracts.
- [x] 15.6 Run `npm run build`.
- [x] 15.7 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 15.8 Run `cargo check --workspace`.
- [x] 15.9 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 15.10 Run `npm run native:panic:check`.
- [x] 15.11 Run `cargo test --workspace`.
- [x] 15.12 Run `openspec validate --specs --strict`.
- [x] 15.13 Run `openspec validate upgrade-cli-parameter-management --strict`.
- [x] 15.14 Inspect `git diff --check`, generated artifacts, migration files, localization coverage, production TS/TSX line counts, command registration, and both service adapters.
- [x] 15.15 Update this task list only for tasks actually verified; leave blocked platform-specific work unchecked and document why.
- [x] 15.16 Produce a final implementation report listing architecture changes, migrations, provider audit decisions, UI changes, tests, command results, known limitations, and per-platform status using `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`.
- [ ] 15.17 Do not archive the change until every required task is complete and `openspec validate upgrade-cli-parameter-management --strict` passes; do not commit or push unless the user explicitly requests it.

## Implementation evidence

Platform: Windows 11 (`win32`), worktree `.claude/worktrees/cli-args`, branch `worktree-cli-args`, based on
`main` at `ee3eaf3f`. Node dependencies installed with `npm ci`.

### Landed in this pass

The native CLI-parameter subdomain is complete and tested **but not yet reachable from a Tauri
command, bootstrap, provider builder, or the settings page**. It is deliberately additive: the
legacy `cli_parameters.rs` monolith still serves every production caller, so the running product is
unchanged. The four new module declarations carry a scoped `#[allow(dead_code)]` (plus
`unused_imports` on the two re-export surfaces) that task 2.1 removes when the monolith is deleted.

| Area | Artifacts |
| --- | --- |
| Canonical registry | `src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json` |
| Domain | `cli_parameters/domain/{selection,definition,rendering,validation,compatibility,dependency,diagnostic,profile,catalog,catalog_validation,error,testing}.rs` |
| Application | `cli_parameters/application/{models,ports,service,resolution,support,error,fakes,tests}.rs` |
| Infrastructure | `cli_parameters/infrastructure/{catalog_loader,sqlite_profile_repository,lifecycle_snapshot_adapter,runtime_adapters}.rs` + two test modules |
| Published API | `cli_parameters/api.rs` (written; not yet re-exported from `tooling/api.rs`) |
| Persistence | migration **81 `cli-parameter-profiles`**, table `cli_parameter_profiles` |
| Generated contract | `scripts/generate-cli-parameter-catalog.mjs` → `src/generated/cli-parameter-catalog.json` |
| Frontend types | `src/types/cli-parameter.ts`, `src/types/cli-parameter-profile.ts` (not yet consumed) |

### Command results

All commands were run on Windows 11 in this worktree.

| Command | Result |
| --- | --- |
| `npx openspec validate upgrade-cli-parameter-management --strict` | PASSED (after the minimal delta rebase below) |
| `npx openspec validate --specs --strict` | PASSED — 136 items |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo check --workspace` | PASSED |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED |
| `cargo test --workspace` | PASSED — 3648 tests. One unrelated flake, see below |
| `cargo test --workspace --lib cli_parameters` | PASSED — 104 new tests (56 domain, 24 infrastructure, 24 application) |
| `npm run native:panic:check` | PASSED |
| `npm run lint:ci` | PASSED |
| `npx tsc --noEmit` | PASSED |
| `npm run test` | PASSED — 296 files, 1350 tests |
| `npm run contracts:check` | PASSED — generator drift check + 3 conformance tests |
| `npm run architecture:check` | PASSED — 42 native fitness tests |
| `npm run build` | PASSED — 16 lazy chunks, main static closure 132.9 KiB gzip |
| `npm run version:unit:test` | PASSED |
| `npm run coverage:policy:test` | PASSED |
| `npm run test:coverage` | PASSED — statements 71.37%, branches 67.67%, functions 67.28%, lines 75.10% |
| `npm run desktop:unit:test` | PASSED |
| `git diff --check` | PASSED |
| generator determinism | `node scripts/generate-cli-parameter-catalog.mjs` twice → `--check` reports no diff |
| generator drift detection | catalogVersion mutated to `0.0.0-stale` → `--check` exits 1; artifact restored, `--check` exits 0 |

Not run: `npx playwright test`, `npm run test:desktop*`. This pass changes no UI and no Tauri
command, so neither suite exercises anything new; both must run once sections 11-12 land.
Per-platform desktop status is therefore `NOT RUN` for Windows, macOS, and Linux alike.

`contexts::desktop::infrastructure::tauri_desktop_lifecycle::tests::graceful_shutdown_distinguishes_success_failure_and_timeout`
failed once in the loaded full-suite run and passed twice in isolation with no code change. It
asserts a 1 ms timeout and is unrelated to this change.

### Adding migration 81 broke a fifth version assertion

`EXPECTED_MIGRATIONS`, the `apply_transactional_migration` call, both
`assert_eq!(migration_count, N)` in `platform/database/mod.rs`, and
`assert_eq!(migration_state, (N-1, N))` in `platform/database/migrations/tests.rs` are the four
usually cited. `src-tauri/src/migration_fixture_tests.rs::expected_versions()` — `(1..=N).collect()`
plus a per-migration doc comment — is a fifth, and it is invisible to a grep for the count/state
assertions. It surfaced only in `cargo test --workspace` as five failing fixture tests.

### Delta rebase applied before implementation (task 0.2)

`openspec validate --strict` rejected five MODIFIED requirements for dropping scenario names the
baseline still carries. Each new scenario was renamed back to its baseline name with its updated
content, and one scenario was added rather than renamed:

- `Deterministic configuration precedence` → `Message value overrides persisted default`, `No message override`
- `Safe effective argument preview` → `Display preview after save`
- `Custom-text parameter control kind` → `Validation accepts arbitrary non-empty values`, `Validation rejects control characters`, `Validation rejects empty values`
- `Argument preview renders custom model values directly` → `Known model value in argument preview (unchanged)`, plus a new `Default model value omitted from preview (unchanged)` covering the legacy `default` sentinel migrating to inherited state
- `Antigravity CLI parameter catalog` → `Load the Antigravity parameter catalog`, `The permission bypass flag is absent from the catalog`, `Preview reflects saved selections`

No acceptance goal was reduced.

### Concurrent-change conflicts recorded (task 0.3)

- **`verify-antigravity-cli-live-runtime`** (active, unimplemented) plans to add real `agy` model
  slugs to `contexts/tooling/cli_parameters.rs` *and* `src/services/cli-parameter-catalog.ts`. This
  change deletes both files. Once the registry is canonical, that work becomes an edit to
  `catalog.v2.json` plus `npm run contracts:generate`. Its Antigravity audit is still pending a live
  capture, which is why every Antigravity entry here records `verification: repository-verified`
  rather than `verified`.
- **`improve-workspace-ui-ergonomics`** (active) owns settings-navigation legibility and requires the
  CLI parameter entry to stay immediately after CLI management. This change does not touch settings
  navigation registration, so the two are compatible.
- **Migration number contention.** `upgrade-session-workspace-evidence-console` is also numbering
  from 81. Whichever lands second must renumber; `EXPECTED_MIGRATIONS` plus
  `assert_migration_history_is_dense` will surface the collision as a name divergence rather than
  silently skipping a migration.

### Deviations from `design.md`, with reasons

- **Generated artifact is JSON, not `.ts`.** The projection is ~1.9k lines. ESLint enforces
  `max-lines: 300` on every `.ts`/`.tsx`, and `eslint.config.js` forbids adding entries to its
  technical-debt budget list. A `.json` artifact carries the same data, is imported the same way
  (`resolveJsonModule` is enabled), and matches the repository's existing cross-language contract
  precedent under `src/contracts/fixtures/`. JSON has no comments, so the generated marker is a
  reserved `$generated` key that the parity test asserts on.
- **`contracts:check` is now a composition.** It runs the generator's `--check` mode before the
  existing conformance test, and `contracts:generate` was added alongside it.
- **`audit` is `skip_serializing`.** Review provenance stays native-only, so neither the generated
  contract nor any future command response can carry audit prose.
- **A version-comparator port instead of a direct call.** `tooling::cli::api::compare_versions` is
  the repository's one version ordering. Importing it from `cli_parameters/domain` would be an
  inward-violating layer jump, so the domain declares `CliVersionComparator` and the infrastructure
  adapter delegates to that single implementation. No second comparator exists.

### Not done, and why

- **Sections 1, 9, 11, 12, 13 and most of 8, 10, 14, 15 are unstarted or partial.** The remaining
  work is one coupled unit: switching the Tauri command DTOs forces the settings page to be
  rewritten in the same pass, and switching persistence to the v2 envelope forces `agent_runtime`
  and `sessions` off the legacy reader at the same time — a legacy reader would reject v2 rows and
  silently stop applying saved parameters to launches. Landing any half of that pairing on its own
  produces a non-functional product, so this pass stops at the additive boundary instead.
- **Task 5.1 (the re-audit itself) is unchecked.** The registry contents implement `design.md`'s
  recorded 2026-08-22 audit and every entry carries a source id, URL, review date, note, and
  verification status. The official provider references were not independently re-fetched in this
  environment, so the audit provenance is the design document's, not a fresh verification.
- **Task 0.4 / section 1 characterization tests are unchecked.** A frontend regression suite was
  drafted against the target page contract, then removed: it can only pass once sections 11-12 land,
  and leaving a red `npm run test` would block every other gate for the next pass.

## Implementation evidence — round 2: native runtime read cutover

Every real managed-CLI launch now resolves its user-profile argv through the published Tooling
runtime API, dual-reading legacy and v2 rows. The settings page, its three Tauri commands, and the
Web/mock writer are deliberately untouched, so no v2 write path is reachable from the UI.

### Call sites migrated

| Call site | Before | After |
| --- | --- | --- |
| `agent_runtime/infrastructure/cli_profile.rs` | `CliParametersApi::{load_selections, normalize_selections, preview_args}` | `tooling::api::CliParameterRuntimeApi::resolve_cli_launch_segments` |
| `providers/invocation.rs` (all five builders) | one flat `managed_args` slice | `ProviderLaunchSegments { global, invocation }` |
| `providers/compatibility.rs` | `request.managed_args` | `request.global_args` + `request.invocation_args` |
| `infrastructure/process_adapter.rs` (fresh chat, resume) | `cli_profile.managed_args` | `cli_profile.{global_args, invocation_args}` |
| `infrastructure/terminal_process.rs` (Agent Terminal) | `cli_profile.managed_args` | `cli_profile.{global_args, invocation_args}` |
| `sessions/infrastructure/chat_profile.rs` | `CliParametersApi::load_selections` | `CliParameterRuntimeApi::resolved_selections` |
| `bootstrap/runtime.rs` | one legacy api for commands, sessions, and runtime | legacy api for commands only; new runtime api for sessions and `agent_runtime` |

Deleted with the cutover: `preview_args`, the free `normalize_selections`, and
`CliParametersApi::{load_selections, normalize_selections, preview_args}` — the legacy launch
renderer has no production caller left. Also deleted: the hand-written `--ephemeral` reshuffle and
`force_gemini_standard_approval_flag`; both were workarounds for semantics the registry now
declares.

### New runtime data flow

```text
send_message / open_terminal
  └─ RuntimeAgentCliProfileAdapter
       ├─ PermissionsApi.find_principal → resolve_effective_execution_policy → launch template
       ├─ providers::policy_override_selections(agent, template)   [governed ids only]
       ├─ providers::message_override_selections(agent, chat config) [ordinary ids only]
       └─ tooling::api::CliParameterRuntimeApi::resolve_cli_launch_segments
            ├─ SQLite repository load  (legacy rows and v2 envelopes)
            ├─ definition-aware legacy migration → quarantine diagnostics
            ├─ CLI lifecycle snapshot → compatibility
            ├─ precedence: message override > explicit profile value > inherited
            ├─ dependency/conflict evaluation
            └─ declarative render → { global tokens, invocation tokens, diagnostics,
                                      profile revision, catalog version }
  └─ provider builder places the two segments in its own grammar
```

Policy never travels the profile path: `resolve_cli_launch_segments` rejects a user-editable id
supplied as a policy override, and the repository never returns a policy-governed row to the
ordinary path. `contexts/tooling` contains zero `contexts::permissions` references, enforced by
`tooling_never_depends_on_permissions`.

### argv placement per provider

`G` is a token the registry declared `global`; `I` one it declared `invocation`.

| Provider | Fresh chat | Resume chat | Interactive |
| --- | --- | --- | --- |
| claude-code | `G I -p --output-format stream-json --include-partial-messages --verbose` | `… --resume <id>` | `G I --session-id <uuid>` / `G I --resume <id>` |
| codex-cli | `G exec I --json -` | `G exec resume <id> I --json -` | `G I` / `G I resume <id>` |
| gemini-cli | `G I -o stream-json` | `G I --resume <id> -o stream-json` | `G I --session-id <uuid>` / `G I --resume <id>` |
| opencode | `G run I --format json <prompt>` | `G run I --session <id> --format json <prompt>` | `G I` / `G I --session <id>` |
| antigravity-cli | `G I -p <prompt> --output-format stream-json` | `G I --conversation <id> -p …` | `G I` / `G I --conversation <id>` |

The registry declares every opencode parameter in the `invocation` slot (its options follow `run`)
and exactly one codex parameter — `ephemeral` — there (the `exec` grammar owns it). That is
asserted directly by `opencode_claims_no_global_slot_because_its_options_follow_the_run_subcommand`.
`fixtures/invocations.json` keeps its original `expectedArgs` — `git diff ee3eaf3f` on that file
shows only the input field being split from `managedArgs` into `globalArgs` + `invocationArgs`; the
golden output is untouched on all five fixtures.

**Correction to the round-2 wording.** That round described this as "fixing the fixture", which read
as if a golden had been adjusted to match the implementation. It had not. What changed was which
*input slot* opencode's tokens are fed through: they were first placed in `global`, the test failed
because `global` precedes `run`, and they were moved to `invocation` to match what the registry
declares. Round 3 replaces that hand-written evidence with a recomputed one — see
`baseline_argv_equivalence_tests.rs`.

### Legacy / v2 dual-read coverage

`contexts/agent_runtime/infrastructure/cli_profile_tests.rs` writes rows exactly as the still-legacy
settings command writes them, then resolves a real launch:

| Case | Test |
| --- | --- |
| legacy rows only reach the next launch (Invariant II) | `a_legacy_row_written_by_the_old_settings_page_reaches_the_next_launch` |
| v2 envelopes reach the next launch (Invariant III) | `a_v2_row_is_read_by_the_next_launch` |
| malformed / unknown / unsupported rows emit nothing, do not fail the launch, and are not rewritten (Invariant VII) | `malformed_unknown_and_unsupported_rows_produce_no_token_and_do_not_fail_the_launch` |
| a policy-governed legacy row never reaches argv through the user path | `a_policy_governed_legacy_row_never_reaches_argv_through_the_user_profile_path` |
| precedence: message > profile, policy over both | `a_message_override_beats_the_saved_profile_and_policy_beats_both` |
| version-gated value omitted when the active version is too old | `a_version_gated_value_is_omitted_when_the_active_version_is_too_old` |
| whitespace-bearing value stays one argv token | `a_whitespace_bearing_value_stays_one_argv_token` |
| missing include directory is dropped, not fatal | `a_missing_include_directory_is_dropped_instead_of_failing_the_launch` |
| every agent × every policy template, chat and terminal | `every_managed_cli_projects_every_policy_for_chat_and_terminal` |

Legacy `"default"` is no longer mapped by a global string rule. `convert_legacy` is
definition-aware: it means inheritance only where the v2 registry does **not** also declare
`default` as a real provider value, and is quarantined where it does (`gemini-cli.approvalMode`).
A stored `false` on a definition whose renderer can now emit a negative flag is likewise ambiguous
and quarantined rather than guessed.

### Migration number

Main is still at 80. Migration 81 is claimed by three unmerged lanes at once:

| Branch | Migration | In main? |
| --- | --- | --- |
| `worktree-ocr` (also `origin/worktree-ocr`) | `(81, "local-media-profiles")` | no |
| `worktree-skill-plugin-mcp` | `(81, "extension-platform-feature-gates")`, `(82, "extension-platform-gate-degradations")` | no |
| this worktree | `(81, "cli-parameter-profiles")` | no |

This lane keeps 81. `expected_versions()` requires a *contiguous* range, so pre-allocating 83 would
open a gap the moment another lane does not land. Whoever merges second renumbers and updates all
five assertion sites: `EXPECTED_MIGRATIONS`, the `apply_transactional_migration` call, both
`assert_eq!(migration_count, N)` in `platform/database/mod.rs`, `assert_eq!(migration_state, …)` in
`migrations/tests.rs`, and `expected_versions()` in `migration_fixture_tests.rs`.

### Compatibility code still in place, and its removal condition

| Kept | Why | Removed when |
| --- | --- | --- |
| `contexts/tooling/cli_parameters.rs` monolith (catalog, `load_profile`, save, reset, `apply_schema`) | still serves `list`/`save`/`reset` and the old settings page | the settings cutover moves those commands to the v2 DTOs (task 2.1) |
| `CliParametersApi` (legacy) in bootstrap and Tauri state | same | same |
| `cli_parameters::api::CliParametersApi` (settings facade, unpublished) | ready for that cutover; deliberately not re-exported so no v2 writer is reachable | it replaces the legacy facade |
| `src/services/cli-parameter-catalog.ts` and the old settings page | untouched this round by design | frontend sections 10.4-12 |
| `#[allow(dead_code)]` on the v2 subdomain modules | the settings-facing half still has no caller | task 2.1 |
| legacy test shims `normalize_selections`/`preview_args` in the monolith's `mod tests` | keep the legacy settings-preview renderer under test after its public wrappers were deleted | with the monolith |

### Round 2 command results

Host OS: Windows 11 (`win32`). Every command was run in this worktree.

| Command | Result |
| --- | --- |
| `npx openspec validate upgrade-cli-parameter-management --strict` | PASSED |
| `npx openspec validate --specs --strict` | PASSED — 136 items |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo check --workspace` | PASSED |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED |
| `cargo test --workspace` | 3674 passed, 4 failed — all four are `mcp::infrastructure::relay_stdio` socket-timing flakes; re-run isolated gives 5 passed / 0 failed. Nothing in this change touches the MCP relay. |
| `npm run native:panic:check` | PASSED |
| `npm run contracts:check` | PASSED |
| `npm run architecture:check` | PASSED — 42 native fitness tests |
| `npm run lint:ci` | PASSED |
| `npx tsc --noEmit` | PASSED |
| `npm run test` | PASSED — 296 files, 1350 tests |
| `npm run test:coverage` | PASSED — statements 71.37%, branches 67.67%, functions 67.28%, lines 75.10% |
| `npm run build` | PASSED |
| `git diff --check` | PASSED |

Focused suites: `cli_parameters` 132 passed, `cli_profile_tests` 16 passed, `providers::tests`
39 passed.

`npx playwright test`, `npm run test:desktop*`: NOT RUN. This round changes no UI and no Tauri
command surface, so neither suite exercises anything new. Desktop E2E status is `NOT RUN` for
Windows, macOS, and Linux alike; a host-OS result would not be transferable to the other two in any
case.

Two architecture violations were introduced and fixed inside this round rather than suppressed:
`ARCH-NATIVE-003` (the new test file constructed `permissions`' concrete repositories) was repaired
by extracting `permissions::api::test_permissions_api_on`, which keeps that wiring in the owning
context; that repair also brought `agent_runtime/infrastructure` back under its `ARCH-NATIVE-007`
aggregate line budget without raising it. (Round 3's characterization tests took it over that
budget again; see the phase A section for the raise and its itemization.)

## Implementation evidence — round 3, phase A: native runtime cutover stabilization

Host OS: Windows 11 (`win32`). Every command below was run in this worktree.

### The round 2 verification status was wrong, and is corrected here

The round 2 table recorded `cargo test --workspace` as "3674 passed, 4 failed" and moved on. That
was a FAILED gate reported as a footnote, and task 15.11 stayed unchecked until this round. An
isolated re-run of a failing test is diagnostic evidence about *why* it failed; it is not the gate.

The complete command now exits 0. Seven targets, 3,766 tests, nothing failed:

| Target | Result |
| --- | --- |
| `vanehub-ai` lib | ok — 3,692 passed, 0 failed, 15 ignored |
| `architecture` | ok — 43 passed |
| `migration_fixture` | ok — 3 passed |
| `mcp_relay_provider_invocations` | ok — 3 passed |
| `mcp_fixture_contracts` | ok — 25 passed |
| `vanehub-permission-hook` lib and bin | ok — 0 tests |

Getting there took four full runs, and the three that failed are worth recording because they
failed for a reason that has nothing to do with this change:

| Run | Tree | Result |
| --- | --- | --- |
| 1 | this change | FAILED — 3,691 passed, 1 failed: `code_intelligence::…::initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` |
| 2 | this change | FAILED — identical, same single test |
| 3 | this change | FAILED — identical, same single test |
| control | `ee3eaf3f`, this change's diff reverse-applied | FAILED — the same `code_intelligence` test, plus `mcp::infrastructure::relay_stdio::tests::child_exit_does_not_wait_for_open_parent_input` |
| 4 | this change | ok — exit 0 |

The control run is the decisive one: the failure reproduces on the unmodified baseline, so it is a
property of this host, not of the cutover. That test spawns a fixture process, waits 2 s for an
initialize that never comes, and asserts the forced process-tree kill finishes inside its own
budget; while a second `cargo` competed for this machine — runs 1-3 and the control all recorded
`Blocking waiting for file lock on package cache` — that budget was not enough. Run 4 was taken with
the machine otherwise idle. Nothing in this change touches `code_intelligence`.

`cargo test --workspace` stops at the first failing target, so runs 1-3 never reached `architecture`,
`migration_fixture` or the two MCP targets at all. A `--no-fail-fast` run was added for that reason;
it is the only source of the per-target evidence above.

### The invocation fixture's golden data was never edited

Phase A asked for this to be printed rather than asserted. `git diff ee3eaf3f --` on
`providers/fixtures/invocations.json`, changed lines only:

```
-    "managedArgs": [   +    "globalArgs": [    +    "invocationArgs": [],
-    "managedArgs": [   +    "globalArgs": [    -      "gpt-5.5",   +      "gpt-5.5"
-    "managedArgs": [   +    "globalArgs": [    +    "invocationArgs": [],
-    "managedArgs": [   +    "globalArgs": [],  +    "invocationArgs": [
-    "managedArgs": [   +    "globalArgs": [    +    "invocationArgs": [],
```

No `expectedArgs` line appears in the diff at all. The only change is that the fixture's *input*
field `managedArgs` was split into `globalArgs` and `invocationArgs`, because the builders now take
two slots instead of one; codex's `--ephemeral` and all of opencode's tokens moved to the invocation
side. Round 2 described this as "correcting the fixture", which reads as though expectations had
been adjusted to match new behavior. They were not, and that wording is withdrawn.

Because a fixture that was never edited also cannot by itself prove compatibility, round 3 adds
`agent_runtime/infrastructure/baseline_argv_equivalence_tests.rs`. It transcribes `build_invocation`,
`build_interactive_invocation`'s resume branch, `apply_policy_template_overrides`' value table and
`force_gemini_standard_approval_flag` verbatim out of `git show ee3eaf3f`, recomputes each provider's
pre-cutover argv through the legacy renderer — `baseline_preview_args`, itself the pre-cutover
renderer kept alive under `#[cfg(test)]` — and compares it against what the live resolver and live
builders produce. Seven tests:

| Test | What it pins |
| --- | --- |
| `fresh_chat_argv_matches_the_pre_cutover_pipeline_for_every_provider` | all five providers, v1-expressible profiles |
| `resume_chat_argv_matches_the_pre_cutover_pipeline_for_every_provider` | same, resume grammar |
| `interactive_resume_argv_matches_the_pre_cutover_pipeline_for_every_provider` | Agent Terminal |
| `the_gemini_standard_difference_is_only_the_approval_mode_position` | difference 1, pinned to its exact shape |
| `claude_bare_moves_from_the_interactive_scope_to_the_chat_scope` | difference 2, likewise |
| `no_unlisted_provider_changes_its_argv` | asserts `compared == 22`, so the suite cannot go vacuous |
| `the_policy_projection_matches_the_legacy_encoding_for_every_combination` | every agent × template |

Exactly two argv differences exist, both intended by `design.md`, both named in `accepted_difference`
so a third cannot appear silently. One of them — `--bare` moving from Interactive to chat — was found
by this suite, not by review.

### Runtime legacy readers: zero, and now ratcheted

Searching `contexts/agent_runtime` and `contexts/sessions` for `load_selections`,
`normalize_selections`, `preview_args`, `managed_args`, `cli_parameter_settings` and
`tooling::cli_parameters::`, excluding `*tests.rs`: **0 matches**. The only hits anywhere in those two
contexts are a local variable named `managed_args` in `providers/tests.rs` and the deliberate
compatibility imports in the two new test files.

The legacy monolith now has exactly four production consumers, all of them the old settings page's
command path:

```
bootstrap/cli_parameters.rs                                     assemble_legacy_cli_parameters_api
commands/tooling/cli_parameters/list_cli_parameter_profiles.rs
commands/tooling/cli_parameters/save_cli_parameter_profile.rs
commands/tooling/cli_parameters/reset_cli_parameter_profile.rs
```

`architecture.rs::cli_parameter_consumers_only_reach_the_published_tooling_api` is the ratchet. It
scans production files under both contexts for eight forbidden needles — the private `domain`,
`application` and `infrastructure` modules, the settings table name, and the three deleted reader
symbols — truncating each file at `#[cfg(test)]` and skipping test sources. It was verified to fire by
temporarily adding a forbidden import, which produced `cli_profile.rs: imports the CLI-parameter
domain`; the import was then removed.

### Launch-time re-evaluation

Four tests in `cli_profile_tests.rs` establish that nothing is frozen into a session:

| Test | Established |
| --- | --- |
| `a_profile_change_reaches_the_next_launch_and_leaves_a_running_process_alone` | a profile edited after session creation reaches the next fresh/resume process; the snapshot already handed to a running process is unchanged |
| `a_policy_change_reaches_the_next_launch` | a policy template edited after session creation reaches the next launch |
| `a_cli_version_change_reevaluates_compatibility_on_the_next_launch` | a version-gated value omitted under an old CLI appears once the installed version changes, through an interior-mutable `StubInstallations` |
| `the_launch_snapshot_carries_no_policy_or_compatibility_state` | `CliProfileSnapshot` is destructured exhaustively, so adding a cacheable field to it fails to compile rather than silently reintroducing staleness |

`CliProfileSnapshot` holds only `executable`, `global_args`, `invocation_args` and `env`. Neither the
policy projection nor a compatibility verdict is representable in it.

### Diagnostics are operation-associated and carry nothing private

`ResolveCliLaunchParametersInput` gained `execution_context: CliLaunchExecutionContext`, whose only
field is `operation_id`. `AgentCliProfileGateway::load` takes it; `agent_runtime`'s service passes
`Some(&operation.id)`, and Agent Terminal passes `None` because a terminal launch has no operation.
`resolution.rs` stamps every diagnostic with that id and emits it before returning, so a caller that
discards the returned diagnostics cannot drop them. `UnifiedCliParameterDiagnostics` writes them to
the unified logging port under category `cli.parameter`, severity mapped from the diagnostic's own.

| Test | Established |
| --- | --- |
| `resolver_diagnostics_are_emitted_even_though_the_launch_caller_discards_them` | emission is the resolver's responsibility, not the caller's |
| `an_emitted_diagnostic_carries_only_stable_safe_fields` | context keys are exactly `code`, `agentId`, `parameterId`, `operationId` and safe remediation details |
| `no_diagnostic_ever_contains_a_prompt_session_id_or_environment_value` | prompts, credentials, API tokens, session ids and secret env values never reach a diagnostic |
| `no_resolved_token_carries_a_prompt_session_or_output_protocol_value` | the same values never reach argv through the user profile path |

One assertion in that group was wrong on first writing and is worth recording: it asserted argv must
not contain a token shaped like `sk-live-…`, but that string is a legal model identifier under the
registry's pattern, so rendering it is correct behavior. The assertion now checks only that
*quarantined* values never reach argv.

### API naming

Three facades exist and are now distinguishable everywhere, including at their carriers:

| Type | Reachable from | Purpose |
| --- | --- | --- |
| `CliParameterRuntimeApi` | `tooling/api.rs` | launch-time resolution: `resolve_cli_launch_segments`, `resolved_selections`, `validate_registry` |
| `CliParameterSettingsApi` | not re-exported | list/preview/save/reset for the settings cutover; deliberately unpublished so no v2 writer is command-reachable |
| `LegacyCliParametersApi` | `contexts/tooling/cli_parameters.rs` | the three old settings commands only |

Bootstrap variables are `cli_parameter_runtime_api` and `legacy_cli_parameters_api`. The carriers
were renamed this round too, since a field named `cli_parameters` reads as any of the three:
`AgentRuntimeDependencies::cli_parameter_runtime`, `assemble_sessions_api`'s `cli_parameter_runtime`,
and `SqliteSessionChatProfileAdapter::cli_parameter_runtime`.

### ARCH-NATIVE-007 was raised, in the same commit, with the itemization it requires

`npm run architecture:check` failed on the first attempt this round:
`agent_runtime/infrastructure: 60547 aggregate physical lines exceeds budget 59467`. The subtree
grows 1,476 lines against `ee3eaf3f`, of which 396 fit the existing headroom.

Production in that subtree does not grow — it falls by 328, because `invocation.rs` lost its
per-parameter-id renderer branches and `cli_profile.rs` its duplicate `default` interpretation. The
entire raise is `#[cfg(test)]`: 693 for the baseline equivalence suite, 932 for `cli_profile_tests.rs`
(most of which is `cli_profile.rs`'s own `mod tests` moved out, then extended to 23 tests), and 179
across `providers/tests.rs`, `compatibility_tests.rs` and the three JSON fixtures. The budget was
raised to the measured 60,547 with that itemization recorded inline, which is the repair the guard's
own diagnostic prescribes. No ESLint `max-lines` exemption was added and no exemption list was
touched.

### Round 3 phase A command results

| Command | Result |
| --- | --- |
| `cargo test --workspace` | PASSED — exit 0, 3,766 tests across 7 targets |
| `cargo test --workspace --no-fail-fast` | 1 failed — `mcp_relay_provider_invocations::injected_protocol_failure_reaps_descendants_and_leaves_no_raw_secret_artifact`, "stdio fixture did not report its descendant PID". Node-spawn timing under load; passes in the gate run above. Nothing in this change touches the MCP relay. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo check --workspace` | PASSED |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED |
| `npm run native:panic:check` | PASSED |
| `npm run architecture:check` | PASSED — 43 native fitness tests, 42 plus the new CLI-parameter ratchet |
| `npm run lint:ci` | PASSED |
| `npx tsc --noEmit` | PASSED |
| `npm run test` | PASSED — 296 files, 1,350 tests |
| `npm run test:coverage` | PASSED — statements 71.35%, branches 67.65%, functions 67.28%, lines 75.08% |
| `npm run coverage:policy:test` | PASSED |
| `npm run version:unit:test` | PASSED |
| `npm run contracts:check` | PASSED — 3 tests |
| `npm run build` | PASSED |
| `npm run desktop:unit:test` | PASSED — 18 tests |
| `openspec validate --specs --strict` | PASSED |
| `openspec validate upgrade-cli-parameter-management --strict` | PASSED |
| `git diff --check` | PASSED |

### Desktop, per platform

| Layer | Windows | macOS | Linux |
| --- | --- | --- | --- |
| `test:desktop:build` | PASSED | NOT RUN | NOT RUN |
| `test:desktop:settings-persistence` | PASSED — 2 spec files | NOT RUN | NOT RUN |
| `test:desktop:cli-terminal` | FAILED — pre-existing, see below | NOT RUN | NOT RUN |
| `npx playwright test` | NOT RUN — deferred to the UI phase | NOT RUN | NOT RUN |

macOS and Linux are `NOT RUN`, not inferred. A Windows result is not transferable to either.

`test:desktop:cli-terminal` fails with `The terminal never produced VANEHUB-FIXTURE-CLI READY`. The
same reversible baseline experiment used for `cargo test` was run for it: the diff was
reverse-applied, `test:desktop:build` rebuilt the client at `ee3eaf3f`, and the layer failed with the
byte-identical error before the diff was restored and the tree verified to compile. It is a
pre-existing failure on this host, not a regression, and task 14.7 stays unchecked.

Controls gathered while diagnosing it, all consistent with that conclusion: `test:desktop:smoke`
passes 32/32; `desktop:unit:test` passes 18/18; the native log shows `cli parameter registry loaded
at catalog version 2.0.0`, then the terminal opening for `opencode`, then no CLI-parameter error of
any kind; the spec's own pre-terminal `availabilityState === "available"` assertion passes; the
fixture stub prints READY unconditionally 500 ms after start; and for an opencode interactive launch
with an empty profile both the baseline and the new path resolve to empty argv.
`globalThis.__terminalOutput` stays empty for the full 30 s wait — the PTY produces nothing, which is
upstream of anything this change decides.

Note that `npm run test:desktop:desktop-cli-terminal` and
`npm run test:desktop:desktop-settings-persistence`, as tasks 14.6 and 14.7 spell them, do not exist;
the scripts are `test:desktop:cli-terminal` and `test:desktop:settings-persistence`, and an individual
layer requires `npm run test:desktop:build` first because it reads
`test-results/desktop/latest-artifact.json`.

### Migration number is a merge blocker

This lane holds migration 81 (`cli-parameter-profiles`). `origin/main` is at 80
(`retire-plan-execution`) as of this round, so 81 is contiguous *today*. It is not reserved: any other
lane that lands an 81 first makes this one collide, and the shared
`%APPDATA%\ai.vanehub.app\vanehub.sqlite` turns that collision into a startup crash rather than a
merge conflict.

**Before merging, fetch and rebase, then renumber to whatever is actually next.** 83 is deliberately
not pre-allocated. Renumbering touches five hard-coded assertions that no compiler or linter checks:
`EXPECTED_MIGRATIONS`, `assert_migration_history_is_dense`, the two counts in
`platform/database/migrations/tests.rs`, and `migration_fixture_tests.rs::expected_versions`'s
`(1..=N)`.

## Implementation evidence — round 3, phase B: command and adapter cutover

Host OS: Windows 11 (`win32`). Phase A's acceptance gate passed first — `cargo test --workspace`
exit 0 — and phase B started only after that.

### The command boundary

Four commands, all on `CliParameterSettingsApi`, all thin DTO adapters:

| Command | Input | Output |
| --- | --- | --- |
| `list_cli_parameter_profiles` | — | `Vec<CliParameterProfileDto>` |
| `preview_cli_parameter_profile` | `PreviewCliParameterProfileRequest` | `CliParameterPreviewDto` |
| `save_cli_parameter_profile` | `SaveCliParameterProfileRequest` | `CliParameterProfileDto` |
| `reset_cli_parameter_profile` | `ResetCliParameterProfileRequest` | `CliParameterProfileDto` |

`preview_cli_parameter_profile` is new and registered in `core_registry.rs`. The other three kept
their names and changed their shapes, so no frontend call site had to learn a new command.

`commands/tooling/cli_parameters/dto.rs` holds the whole boundary. The domain types it embeds
already carry the wire shape the TypeScript contract declares — `CliParameterDefinition`,
`CliParameterSelection`, `CliArgumentSegments`, `CliParameterDiagnostic`, `CliParameterSupport` all
derive serde with the right casing — so the DTOs name the boundary rather than restate it. No
application model and no infrastructure type crosses.

Two serialization decisions are pinned by tests because either could regress into a shape the
frontend contract does not admit: `updatedAt` is `null` rather than absent (the page distinguishes
"never saved" from "saved at an unknown time"), and `requestId` is absent rather than `null` (the
contract declares it optional, and `null` is not `undefined`).

### Errors are objects with codes, not sentences

`CommandError` serializes as a prose string, so these commands do not use it. They reject with
`CliParameterCommandError`:

```json
{ "code": "CLI_PARAMETER_REVISION_CONFLICT", "agentId": "gemini-cli",
  "details": { "expectedRevision": "2", "actualRevision": "5" } }
```

`code` is the stable identifier, `agentId` and `parameterId` locate it on the page, and `details`
carries only the bounded context the application error already decided to publish — a repository
failure serializes to exactly one key, its code, because its cause can carry filesystem or SQL text.

The page's four English regular expressions (`/Invalid value for CLI parameter: ([\w-]+)/i` and
friends) are gone. `asCliParameterServiceError` recognizes the structured rejection by matching
`code` against the ten known codes and returns `null` for anything else, which falls through to a
generic transport message. Both adapters reject with the same shape: the Web/mock client throws
`Object.assign(new Error(code), error)`, so `rejects.toMatchObject({ code })` works against either.

`From<CliParametersError> for CommandError` was deleted with its last caller.

### Revision and catalog CAS, now reachable

`SaveCliParameterProfileRequest` and `ResetCliParameterProfileRequest` both require
`expectedRevision` and `catalogVersion`, and both are `deny_unknown_fields`. A caller that has not
read a profile cannot construct either input, which is the point: a blind write would silently
overwrite whatever another window saved. Reset carries the same tokens as save, because reset is a
write.

`preview` carries `catalogVersion` but no revision, and touches neither stored selections nor the
revision — proven on both sides (`the preview use case does not mutate persistence`, and
`previews a draft through the Web adapter without persisting it`, which compares the revision
before and after).

The legacy writer no longer exists as a reachable path, so there is nothing left that bypasses the
revision.

### The legacy monolith is now test-only

`contexts/tooling/cli_parameters.rs` is 69 lines: the `cli_parameter_settings` table's schema, which
migration 81 still needs because the v2 reader dual-reads legacy rows, and the error type that
schema returns. Everything else — catalog, validator, renderer, persistence, and the
`LegacyCliParametersApi` facade — moved verbatim into `cli_parameters/legacy_baseline.rs`, declared
`#[cfg(test)]`. It is what `baseline_argv_equivalence_tests` recomputes the pre-cutover argv
through, and its own tests keep the legacy write path — the one the dual-read must stay compatible
with — from drifting unobserved. The facade itself was deleted: nothing constructed it any more.

That move surfaced a vestigial seam and removed it. `CliParameterClockPort`,
`SystemCliParameterClock` and `CliParameterApplicationService::now` had no caller at all — the
repository stamps `updated_at` from its own `chrono::Utc::now()` — so the port was a seam with no
joint. Removing it, plus `LoadedProfile::definitions`, `selections_from`, and the settings API's
redundant `catalog_version`, is what let the four `#[allow(dead_code)]` module attributes come off
the subdomain entirely. `cargo clippy --workspace --all-targets -- -D warnings` passes with none.

### Web/mock

`web-cli-parameter-client.ts` was rewritten against the generated catalog. Three things about it are
deliberate:

**The generated artifact is parsed, not asserted.** `cli-parameter-registry.ts` runs the JSON
through a zod schema at module load. A bare `as CliParameterCatalog` would turn a generator
regression into a runtime shape mismatch inside the adapter, and the adapter is the one place the
native tests cannot see.

**The mock enforces the conflict rules rather than skipping them.** A page that only ever runs
against a permissive adapter never exercises its conflict branch, so the mock rejects a stale
revision and a stale catalog with the same codes the command does.

**The mock is honest about what it cannot do.** It has no CLI to detect, so every installation
reports not-installed and every field's support is `not-installed`. It never claims a version.

`cli-parameter-renderer.ts` ports the six declarative render strategies and the TOML basic-string
encoder so the mock can preview without a native process. No branch in it keys on a parameter id,
for the same reason the native side has none.

Storage moved to `vanehub.cli-parameter-profiles.v2` with a one-time, read-side, non-destructive
migration from `.v1`. The conversion is definition-aware, not string-matched: `"default"` and
`false` were v1's two "not set" sentinels, but neither is a sentinel where the definition gives it a
real meaning, so a definition carrying a literal `default` option keeps the value and a `tri-state`
control keeps an explicit `false`. The v1 key is left in place, so a downgrade still finds its data.

### Frontend

`src/services/cli-service.ts`'s `CliParameterService` now declares list/preview/save/reset over the
v2 DTOs; `tauri-agent-client.ts` and `web-cli-parameter-client.ts` implement the same four methods,
so adapter parity is a compile-time property rather than a review item.

The settings page keeps its visual structure — same rail, same panels, same preview block — and
changed what it talks to:

* fields come from `profile.fields[].definition` instead of `profile.definitions`
* selections are `CliParameterSelection` envelopes, so the control offers Inherit as its own choice
  rather than a value named `default`
* the preview comes from `previewCliParameterProfile`, keyed by the draft itself, so a response for
  an older draft belongs to an older react-query key and is discarded rather than raced
* save and reset send the profile's revision and catalog version
* errors are read by code

No React component gained a Tauri import: `npm run architecture:check` passes, including
`ARCH-FE-002`.

Three production files stayed under the 300-line rule without an exemption — page 229,
control 174, view model 70 — and the two new service modules are 161 and 112.

`ARCH-FE-004` had to be raised twice, to the measured 19,727, with the reasoning recorded inline in
`scripts/architecture/frontend-rules.mjs`: 273 lines for the validated registry loader and the
renderer port, 215 for the adapter's optimistic concurrency and structured errors, 51 for the
storage migration. The old `cli-parameter-catalog.ts` (207 lines) cannot come out yet — see below —
so the budget should fall again when it does.

### Still open, and why

| Task | Why it is not checked |
| --- | --- |
| 0.4, 1.1-1.5 | These characterize the *old* page's component behavior. Sections 11-13 replace that page, and writing component tests against an interim page would pin behavior the redesign is meant to change. |
| 2.1 | The monolith file still exists, holding the legacy table's schema. It cannot go until the dual-read does. |
| 5.1 | Requires re-reading five official provider references. Not done in this pass; the registry still carries the earlier audit's dates. |
| 8.7 | The v1 DTOs in `src/types/agent.ts` still have consumers: `cli-parameter-catalog.ts` and its two tests. |
| 10.4 | `cli-parameter-catalog.ts`'s remaining consumers, by search: `src/contracts/cli-parameter-catalog-audit.test.ts` and `src/services/cli-parameter-catalog.test.ts`. Both are tests, but the audit test asserts the source-review fixture, which task 5.1 owns. Deleting the catalog before 5.1 would drop that assertion. |
| 11.x, 12.x, 13.x | Out of scope for this round by instruction. |
| 14.5, 14.7-14.11 | Playwright deferred to the UI phase; `test:desktop:cli-terminal` fails for a pre-existing host reason recorded in phase A; per-platform CI fixtures, accessibility, and the SQLite legacy-row fixture pass are not done. |

### Round 3 phase B command results

| Command | Result |
| --- | --- |
| `cargo test --workspace` | PASSED — exit 0, 3,773 tests across 7 targets (lib 3,699) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo check --workspace` | PASSED |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED — no `#[allow(dead_code)]` left on the subdomain |
| `npm run native:panic:check` | PASSED |
| `npm run architecture:check` | PASSED — 43 native fitness tests |
| `npm run lint:ci` | PASSED |
| `npx tsc --noEmit` | PASSED |
| `npm run test` | PASSED — 298 files, 1,371 tests |
| `npm run test:coverage` | PASSED — statements 71.46%, branches 67.60%, functions 67.36%, lines 75.20% |
| `npm run coverage:policy:test` | PASSED |
| `npm run version:unit:test` | PASSED |
| `npm run contracts:check` | PASSED — now 2 files, 13 tests |
| `npm run build` | PASSED |
| `npm run desktop:unit:test` | PASSED — 18 tests |
| `npm run docs:check` | PASSED |
| `openspec validate --specs --strict` | PASSED |
| `openspec validate upgrade-cli-parameter-management --strict` | PASSED |
| `git diff --check` | PASSED |

`contracts:check` gained `src/contracts/cli-parameter-contract.test.ts`, which is where task 10.7
lives: deterministic regeneration (generate twice, compare bytes), Rust/TypeScript registry parity
by id, order, ownership, slot, renderer kind and launch scope, the policy/reserved exclusion, an
argv-token check for whitespace-bearing values, scope filtering, and Web preview parity per
strategy including the TOML encoder.

### Desktop, per platform

| Layer | Windows | macOS | Linux |
| --- | --- | --- | --- |
| `test:desktop:build` | PASSED | NOT RUN | NOT RUN |
| `test:desktop:smoke` | PASSED — 32 spec files | NOT RUN | NOT RUN |
| `test:desktop:settings-persistence` | PASSED — 2 spec files | NOT RUN | NOT RUN |
| `test:desktop:cli-terminal` | FAILED — pre-existing, established in phase A | NOT RUN | NOT RUN |
| `npx playwright test` | NOT RUN — deferred to the UI phase | NOT RUN | NOT RUN |

The smoke layer earned its keep here. `tests/desktop/specs/domain-cli-tooling.e2e.mjs` still spoke
the v1 DTO, and the first run after the cutover failed with
`invalid type: boolean true, expected internally tagged enum CliParameterSelection` — the
command correctly refusing a v1 payload against a real desktop client. The spec was migrated:
`profile.definitions` became `profile.fields[].definition`, selections became envelopes, the flag
came off the renderer rather than a flat field, save and reset gained the two optimistic tokens, and
the preview command got its own read-only assertion.

Migrating it exposed a second real difference. The old spec set `opencode.variant` on its own; the
v2 registry declares `variant` as depending on `model` being set, because a variant names a variant
of the selected model. v1 had no dependency rules and rendered it anyway. The spec now asserts both
halves: setting `variant` alone is rejected, and setting it with a `provider/model` value renders
`--model … --variant high --thinking`. It also pins that a stale revision and a stale catalog
version are each rejected without mutating the stored profile.

Second run: 32 of 32 spec files pass.

`src-tauri/gen/schemas/desktop-schema.json` and `windows-schema.json` pick up the WebDriverIO
plugin's permission entries whenever `test:desktop:build` runs with the `desktop-e2e` feature. That
is a byproduct of running the desktop suite, not part of this change, so both files were restored
before the final diff.
