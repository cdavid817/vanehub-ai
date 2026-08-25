## 1. Baseline evidence before any change

- [x] 1.1 Record the pre-change pass state of the LSP suites that are this change's acceptance evidence: `cargo test --workspace` filtered to `code_intelligence`, `native_lsp_end_to_end_tests`, and `migration_fixture_tests`; the frontend LSP vitest files; `npx playwright test tests/e2e/lsp-settings.spec.ts`. A generalization is only proven by these staying green, so a pre-existing failure must be known now rather than blamed on the refactor later
  - Baseline captured at `712bbafa` + planning commit, all green:
    - `cargo test --workspace code_intelligence` — **152 passed, 0 failed**; plus `tests/architecture.rs` 2 passed (`code_intelligence_context_exposes_a_layered_public_api_boundary`, `code_intelligence_never_imports_private_retrieval_layers`)
    - `cargo test --workspace native_lsp` — **1 passed** (`native_lsp_runtime_covers_tools_reconfiguration_trust_and_desktop_shutdown`)
    - `cargo test --workspace migration_fixture` — **13 passed**
    - Frontend LSP vitest (6 files: `lsp-contract`, `web-lsp-client`, `tauri-lsp-adapter`, `lsp-adapter-conformance`, `code-intelligence-page`, `lsp-settings-localization`) — **36 passed**
    - `npx playwright test tests/e2e/lsp-settings.spec.ts` — **1 passed**. Requires unsetting `all_proxy`/`ALL_PROXY` first: the host SOCKS5 proxy makes Playwright fail with `Protocol "socks5:" not supported`. Pin `PLAYWRIGHT_PORT` (used 5199) so the run cannot silently reuse another worktree's dev server
- [x] 1.2 Capture the current serialized shapes of `get_lsp_configuration`, `discover_lsp_servers`, and `list_lsp_server_status` as fixtures, so contract drift is diffable rather than argued about
  - The four pre-existing tests in `dto_tests.rs` assert field by field, so an added field passes them silently. Added three whole-object assertions — one per command result — which now fail with a reviewable diff when the contract widens. `cargo test --workspace code_intelligence::dto` — **7 passed, 0 failed**

## 2. Migration and storage

- [x] 2.1 Re-scan every local and remote branch for the highest in-flight migration number and claim the next free one. At the time this change was written the highest anywhere was 85 (`cli-action-plans`), so 86 was next — but all worktrees share one `%APPDATA%\ai.vanehub.app\vanehub.sqlite`, and two sibling branches already carry colliding numbers, so re-verify instead of trusting this line
  - Scanned all local and remote refs: highest anywhere is 85 (`cli-action-plans`, on `main`/`origin/main`). **Claimed 86.**
  - Pre-existing collisions found, not caused by this change and not fixed here: `worktree-personalization` uses 83 for `session-personalization-mode` while `main` uses 83 for `cli-environment-snapshots`; `worktree-workspace` uses 82 for `unified-log-query-index` while `main` uses 82 for `local-media-profiles`.
- [x] 2.2 Write a failing migration fixture test asserting that after migration a pre-existing `lsp_language_configurations` row keeps its `revision` and `updated_at` verbatim, and that a row whose `language_id` is neither `rust` nor `typescript_javascript` can be inserted
  - Added `infrastructure/schema_tests.rs` with five tests. Confirmed red first: `unresolved import super::schema::apply_language_registry_schema`
- [x] 2.3 Add the migration that rebuilds `lsp_language_configurations` without `CHECK (language_id IN (...))` and with `startup_arguments_json` nullable: create replacement, copy all columns, drop original, rename
  - `apply_language_registry_schema` in `infrastructure/schema.rs`, exported through `api.rs`, registered as migration **86 `lsp-language-registry`**. Self-guards on column nullability so repeated application is a no-op.
  - Existing `startup_arguments_json` values are copied as NULL, not carried over. Nothing but the compile-time constant could ever write them, so preserving them would record every existing installation as having explicitly overridden its arguments and freeze today's defaults permanently.
  - All five tests green.
- [x] 2.4 Update the four hard-coded migration-count/version assertions. Neither the compiler nor clippy reports them; find them by running the suite, not by reading
  - **This task's premise was stale.** The migration-count assertions in `platform/database/mod.rs` are derived (`expected_migration_versions().len()`), and `migration_fixture_tests::expected_versions()` delegates to the same function. Adding migration 86 broke no version or count assertion.
  - Exactly one test failed, and for a different reason: `current_schema_adds_disabled_lsp_configuration_and_empty_workspace_trust` read `startup_arguments_json` as a non-null `String`. Updated it to `Option<String>` expecting `None`, and added an assertion that version 86 is recorded as `lsp-language-registry`.
- [x] 2.5 Update `platform/database/mod.rs` schema assertions that name the LSP migration, and confirm `migration_fixture_tests` still asserts a disabled-by-default seeded configuration
  - No change needed in `platform/database/mod.rs`; its LSP assertion names migration 58 by version and still holds. Extended the `migration_fixture_tests` header comment to describe migration 86.
  - Verified green: `cargo test --workspace migration` **76 passed**, `cargo test --workspace platform::database` **28 passed**, `npm run version:unit:test` **9 passed**

## 3. Domain registry

- [x] 3.1 Add the validated `LspLanguageId` string newtype following the `CliToolId` pattern: bounded length, no control characters, no leading or trailing whitespace, constructed at every wire and row boundary
  - `domain/language_id.rs`. Deliberately stricter than the CLI rule: `[a-z0-9_]` only, max 64. A language id is concatenated into the `lspSettings.language.<id>` localization key and stored as a primary key, so casing ambiguity and separator characters are worth refusing outright. Kept the CLI's `new` / `trusted` split so registry literals do not pay a release-build panic for a typo the tests already catch.
- [x] 3.2 Define the language definition struct carrying id, candidate executable names in preference order, project-root markers, extension-to-`languageId` mapping, default startup arguments, default initialization options, platform applicability, and server-test fixture project
  - `domain/registry.rs`. Table stays `const` by holding `&'static str` and minting the validated id on demand, the same way `CliToolDefinition::tool_id()` does.
- [x] 3.3 Add the static definition table with Rust and TypeScript/JavaScript entries whose declared data is byte-identical to today's constants, plus an `Option`-returning lookup by id
  - Three lookups: by language id, by extension (returning the owning language and its LSP `languageId`), and by server id.
- [x] 3.4 Add a registry-completeness test asserting every entry supplies at least one executable name, at least one root marker, at least one extension mapping, and a fixture project
  - `domain/registry_tests.rs`, 7 tests. Also asserts language ids, server ids, and extensions are unique across the registry — extension lookup returns the first match, so a contested extension would resolve by declaration order and silently route a file to the wrong server.
- [x] 3.5 Replace `LspConfiguration`'s two-language `Default` and its `languages.len() != 2` validation with registry-derived defaults, keeping every switch disabled by default
  - The "configuration must name every supported language exactly once" rule is gone, not relaxed. A build that registers a new language has to be able to read a configuration written before it existed, so a partial map is now the normal case. The test that asserted the old rule was rewritten to assert the new one.
- [x] 3.6 Model startup arguments as `Option<Vec<String>>` where `None` means the registry default and `Some` replaces it, and validate the bounded list-of-strings shape
  - Bounds: at most `MAX_STARTUP_ARGUMENTS` (32) entries, `MAX_STARTUP_ARGUMENT_BYTES` (4 KiB) total, and no embedded NUL — a NUL would be truncated or refused by the platform when the list reaches a process, where the reason can no longer be reported.
- [x] 3.7 Delete `LanguageFamily` and `ServerKind`, resolving every match site to a registry lookup
  - Both enums collapse into one `Language = &'static LanguageDefinition`. They were always the same choice expressed twice, so as one `Copy` reference they cannot disagree, and `QueryOutcome`, `ProcessKey`, `LspDiagnosticIdentity`, `ServerStatus`, and `DiscoveredServer` each lost a field rather than gaining a clone.
  - 26 files, ~190 references. `LspLanguageId` (owned) remains where a value crosses storage or the wire; the reference is used everywhere inside the runtime.

## 4. Infrastructure

- [x] 4.1 Drive `server_discovery` from the registry, resolving candidate executables in declared preference order and reporting which candidate was selected
  - `ServerCommandPreset` is gone; `ServerDiscoveryResult` now carries the language, the resolved arguments, and `selected_executable_name`.
- [x] 4.2 Drive `project_root` marker lookup from the registry without changing today's Rust and TypeScript detection results
- [x] 4.3 Drive `document_snapshot` extension-to-`languageId` admission from the registry
  - Found a pre-existing drift while consolidating: `api.rs::language_for_path` accepted `.mts` and `.cts`, which `document_snapshot` then refused, so such a file passed the gate only to fail one step later. Both now read the same registry mapping. No user-visible change — those files were never actually served — and adding them is deliberately left to a change that can spec it.
- [x] 4.4 Drive `server_test`'s isolated minimal project from the registry's per-language fixture declaration
- [x] 4.5 Include startup arguments in the server-instance configuration fingerprint so changing them drains and restarts matching servers, and add a test proving it
  - `api_tests.rs`, 3 tests. One of them pins that argument *boundaries* matter, so `["ab"]` and `["a", "b"]` cannot hash alike and leave a server running under a command line the user changed.
- [x] 4.6 Make `configuration_repository` preserve rows for unregistered language ids untouched while excluding them from the effective configuration, and add a test that inserts an unknown row and asserts startup succeeds with it intact
  - This was a real crash vector, not a hypothetical: `load_configuration` did `LanguageFamily::parse(&language)?`, so once the CHECK constraint was dropped a single unknown row would fail the entire load. Covered at both layers — storage (`schema_tests`) and repository (`configuration_repository_tests`).
- [x] 4.7 Reject requests naming an unregistered language with a safe reason code, with no process start and no fallback to another language
  - Enforced at both entry points: `test_lsp_server` resolves through the registry before doing anything, and `TryFrom<LspConfigurationDto>` refuses to persist settings for a language nothing can serve.
- [x] 4.8 Report a language with no applicability for the host platform as unavailable with a platform reason, distinguishable from undiscovered
  - New `DiscoveryReason::UnsupportedOnThisPlatform`, checked before discovery runs, plus `supportedOnHost` on the wire descriptor.

## 5. Commands and native API

- [ ] 5.1 Widen `commands/code_intelligence/dto.rs` to carry language ids as validated strings and to include the registered-language descriptor list
- [ ] 5.2 Extend `get_lsp_configuration` to return descriptors (id, platform applicability, whether an executable override is permitted) alongside configuration
- [ ] 5.3 Extend `save_lsp_configuration` to accept per-language startup arguments and to reject unregistered ids and malformed arguments with safe reason codes
- [ ] 5.4 Update `discover_lsp_servers`, `test_lsp_server`, and `list_lsp_server_status` to be registry-driven, keeping their existing result shapes for the two current languages
- [ ] 5.5 Update `command_tests.rs` and `dto_tests.rs`, and confirm cross-context access still goes through `code_intelligence::api` so the architecture fitness rules stay satisfied

## 6. Frontend contract and adapters

- [ ] 6.1 Widen `src/types/lsp.ts`: replace the `lspLanguageIds` tuple literal with an opaque `LspLanguageId` string type and add the descriptor type. Do this in one commit so `tsc --noEmit` enumerates every affected site at once
- [ ] 6.2 Update `src/services/agent-service.ts` and `src/services/lsp-contract.ts` for descriptors and startup arguments
- [ ] 6.3 Update the Tauri adapter (`tauri-agent-client.ts`) — desktop path only, `invoke()` stays confined here
- [ ] 6.4 Update the Web/mock adapter (`web-agent-client.ts`, `web-lsp-client.ts`) to return the same descriptor and startup-argument shape deterministically, with no filesystem, process, or network access
- [ ] 6.5 Update `lsp-adapter-conformance.test.ts` so the two adapters are proven to still agree on the widened contract
- [ ] 6.6 Verify the services subtree aggregate line budget with `npm run architecture:check`. It reports after lint, tsc, and build have already passed, so check it here rather than at the end

## 7. Settings UI and localization

- [ ] 7.1 Replace the two hard-coded language sections with one card component rendered per descriptor, keeping discovery state, executable override, initialization options, and server testing per language
- [ ] 7.2 Add the bounded startup-arguments control, distinguishing "not overridden" from "overridden to empty" in the UI, since clearing it must not silently strip `--stdio` from the TypeScript server
- [ ] 7.3 Present a language unsupported on this host distinctly from one whose executable was merely not discovered
- [ ] 7.4 Add the `lspSettings.language.<id>` fallback to the raw id when a locale lacks the key, so a later change adding a language cannot render blank labels across five bundles
- [ ] 7.5 Update `src/i18n/locales/{en,zh-CN,zh-TW,ja,ko}.json` for the startup-arguments control and the unsupported-on-host state, and update `lsp-settings-localization.test.ts`
- [ ] 7.6 Update the LSP settings component tests, keeping in mind that the default test language is not English and that jest-dom matchers are unavailable in this harness

## 8. Documentation

- [ ] 8.1 Update `docs/{user,developer}-guide/{en,zh-CN}/src/lsp-code-intelligence.md` to describe the registry, configurable startup arguments, and the unsupported-on-host state
- [ ] 8.2 Run `npm run docs:check` and fix every CommonMark issue, keeping sentence-final punctuation outside the bold delimiters in Chinese text

## 9. Verification

- [ ] 9.1 `npm run lint:ci`
- [ ] 9.2 `npm run test`
- [ ] 9.3 `npm run build`
- [ ] 9.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 9.5 `cargo check --workspace`
- [ ] 9.6 `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] 9.7 `npm run native:panic:check`
- [ ] 9.8 `cargo test --workspace`
- [ ] 9.9 `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`
- [ ] 9.10 `npm run architecture:check`
- [ ] 9.11 `npx playwright test` — the settings UI behavior changed. Pin `PLAYWRIGHT_PORT` to a dev server this lane started, or the run may silently test another worktree's code
- [ ] 9.12 `npm run desktop:unit:test` and `npm run test:desktop` — the IPC contract changed. Report per-platform results without extrapolating from this host
- [ ] 9.13 `openspec validate extend-lsp-language-registry --strict` and `openspec validate --specs --strict`
- [ ] 9.14 Simulate the real archive merge with `buildUpdatedSpec` before archiving. `validate --strict` does not model it, and a renamed requirement or a dropped scenario fails only at archive time

## 10. Acceptance

- [ ] 10.1 Confirm every suite from task 1.1 passes unchanged, and that the only new tests are for genuinely new behavior: unknown-id handling, startup-argument validation, executable preference order, platform inapplicability, and revision preservation across migration
- [ ] 10.2 Diff the task 1.2 command fixtures and confirm the only differences are the added descriptor list and startup-argument field
- [ ] 10.3 Confirm no language name appears in any frontend component, so `add-lsp-go-python-cpp` needs no new per-language component
