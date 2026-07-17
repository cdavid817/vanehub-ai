## 1. Contracts and first-version catalog

- [x] 1.1 Add strict TypeScript CLI parameter definition, selection, profile, save, and reset contract types without `any` or runtime-specific fields.
- [x] 1.2 Add failing catalog contract tests for the four stable agent ids, unique parameter ids, control/value compatibility, defaults, launch scopes, risk metadata, reserved-flag exclusions, and safe preview tokens.
- [x] 1.3 Implement the curated native catalog and Web/mock fixture for Claude Code, Codex CLI, Gemini CLI, and OpenCode, verifying each delivered CLI has documented enum and presence-style controls.
- [x] 1.4 Verify current provider help/official references for every first-version flag and value; remove unsupported fresh/resume combinations and record any catalog narrowing in `design.md` implementation notes.

## 2. Native persistence and commands

- [x] 2.1 Add failing Rust migration/repository tests for creating `cli_parameter_settings`, upgrading an existing database, round-tripping typed values, isolating profiles by stable agent id, and preserving all existing tables.
- [x] 2.2 Implement the additive SQLite migration and CLI parameter repository with atomic save, load normalization, and per-CLI reset behavior.
- [x] 2.3 Add failing native validation tests for unknown agent ids, unknown parameter ids, wrong JSON value kinds, unsupported enum values, control characters, reserved conflicts, and atomic rollback.
- [x] 2.4 Implement bounded list, save, and reset Tauri commands in a CLI-parameter command module and register them without adding direct database access outside the native layer.
- [x] 2.5 Route load/save/reset warnings and errors through unified logging with stable agent/parameter context and redaction tests.

## 3. Frontend service and runtime adapters

- [x] 3.1 Add failing `AgentService` contract tests for listing four profiles, atomic saves, normalized responses, safe previews, and reset-to-default semantics.
- [x] 3.2 Extend `AgentService`, the runtime client, and the Tauri adapter with list/save/reset methods, keeping all `invoke()` calls inside `tauri-agent-client.ts`.
- [x] 3.3 Implement Web/mock localStorage persistence with the same normalization, validation-visible errors, reset behavior, and preview contract without claiming local launch support.
- [x] 3.4 Extend frontend contract-conformance tests to detect native/Web drift in stable ids, parameter ids, control kinds, defaults, value shapes, and profile mutation behavior.

## 4. Provider-specific argument composition

- [x] 4.1 Add failing Rust tests for each provider's fresh chat, resumed chat, and interactive argv shapes with no saved overrides, proving existing required arguments remain unchanged.
- [x] 4.2 Refactor the CLI invocation seam to accept typed effective selections while preserving provider subcommands, structured output, session/resume ids, prompt delivery, stdin markers, and distinct argv tokens.
- [x] 4.3 Implement Claude Code mappings for the delivered model, effort, and permission-mode catalog values with bypass-permission arguments excluded.
- [x] 4.4 Implement Codex mappings for the delivered model, sandbox, approval, ephemeral, and strict-config values, including safe TOML serialization and correct pre-/post-subcommand placement.
- [x] 4.5 Implement Gemini mappings for the delivered model, approval, and sandbox values while preserving the documented effective YOLO default unless the user explicitly saves another supported value.
- [x] 4.6 Implement OpenCode mappings for the delivered Agent, thinking, and automatic-approval values while preserving `run`, session, format, and prompt placement; keep dynamic model discovery deferred as recorded in `design.md`.
- [x] 4.7 Add security regression tests proving catalog values never use shell interpolation, cannot override reserved arguments, and cannot introduce secrets or prompts into the preview.

## 5. Launch scope and chat precedence

- [x] 5.1 Add failing tests showing interactive launch reads only `interactive` selections immediately before spawn and does not affect availability detection.
- [x] 5.2 Update interactive launch routing to load and apply the active CLI profile through the provider builder.
- [x] 5.3 Add failing provider-mapping tests for supported per-message model, reasoning, and permission values, including concise behavior for unsupported mappings.
- [x] 5.4 Connect native `ChatConfig` to provider argument construction and enforce `per-message > persisted profile > provider default` precedence without mutating the stored profile.
- [x] 5.5 Add next-process tests showing a save during streaming does not alter the running child and is read by the following fresh or resume spawn.

## 6. Settings page interaction

- [x] 6.1 Add failing settings registry/navigation tests for the `cli-parameters` page immediately after CLI Management and before the remaining settings pages.
- [x] 6.2 Add the CLI Parameter Management page using shared page parts, semantic Tailwind tokens, lucide icons, and keep every component file below the 300-line limit.
- [x] 6.3 Implement fixed-order CLI selection, translated parameter/value descriptions, enum dropdowns, boolean switches, catalog-driven multi-select support, risk notices, and search filtering.
- [x] 6.4 Implement mounted per-CLI draft state, dirty indicators, explicit Save, confirmation-based Restore Defaults, loading/error/success states, and safe tokenized previews through `AgentService` only.
- [x] 6.5 Add component/service and Playwright tests for draft preservation, profile switching, control rendering, validation errors, save/reset mutations, preview updates, and separation from CLI package operations.

## 7. Internationalization, themes, and accessibility

- [x] 7.1 Add semantically aligned `zh-CN` and `en` resources for navigation, page copy, every delivered parameter/value explanation, warnings, actions, status, validation, and previews.
- [x] 7.2 Extend i18n parity and visible-text guardrail tests to cover the new page and ensure literal exceptions are limited to provider names, flags, values, and stable ids.
- [x] 7.3 Verify keyboard navigation, labels, focus states, disabled states, switch semantics, dropdown semantics, confirmation behavior, and screen-reader names in component and Playwright tests.
- [x] 7.4 Add or update Playwright coverage for desktop and narrow layouts in both `futuristic` and `minimal` themes and both supported locales, checking clipping, contrast, overflow, and fixed settings navigation.

## 8. Focused and full verification

- [x] 8.1 Run focused frontend service, adapter, settings page, i18n, and contract tests and fix all failures.
- [x] 8.2 Run focused Rust catalog, migration, validation, logging, argument-builder, chat precedence, interactive launch, and next-process tests and fix all failures.
- [x] 8.3 Run `npm run test` and `npm run build`; verify strict TypeScript through the build and record the repository's missing `npm run lint` script in `design.md`.
- [x] 8.4 Run changed-file `rustfmt --check`, full `cargo test`, `cargo check`, and `cargo clippy`; record unrelated repository-wide rustfmt and clippy baseline findings in `design.md`.
- [x] 8.5 Run Playwright E2E/visual checks in Web/mock mode and proportional native repository/provider tests for save, restart restore, interactive launch, fresh chat, and resume chat; leave real authenticated CLI spawning to a release smoke test.
- [x] 8.6 Run `openspec validate --specs --strict` and `openspec validate add-cli-parameter-management --strict`, then reconcile implementation, specs, and task checkboxes before verification/archive.
