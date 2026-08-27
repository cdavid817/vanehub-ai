## 1. Baseline

- [x] 1.1 Record the pre-change pass state of the suites this change must not disturb: `cargo test --workspace code_intelligence`, `cargo test --workspace native_lsp`, `cargo test --workspace migration_fixture`, the six frontend LSP vitest files, and `npx playwright test tests/e2e/lsp-settings.spec.ts`. Unset `all_proxy`/`ALL_PROXY` and pin `PLAYWRIGHT_PORT` for the last one
  - Baseline on `2ee79404`: `code_intelligence` **179**, architecture **2**, `native_lsp` **1**, `migration_fixture` **13**.
- [x] 1.2 Confirm the first claim this change depends on: adding a language needs no database migration. Note the current highest migration number and expect it to be unchanged at the end
  - Highest was **86** (`lsp-language-registry`). See task 7.2 for the end state.

## 2. Root detection

- [x] 2.1 Write failing tests for multi-marker detection: each of a language's markers identifies a root on its own, a directory holding several resolves identically to one holding a single marker, and a nearer directory wins regardless of which marker each holds
- [x] 2.2 Write a failing test for a marker naming a nested path, since `build/compile_commands.json` depends on `Path::join` accepting a separator
  - Confirmed red first: the tests would not compile because `registry::{go,python,cpp}` and `ProjectRootError::RequiredMarkerMissing` did not exist.
- [x] 2.3 Write a failing test for `requires_root_marker`: a language that declares it and finds no marker fails resolution instead of falling back to the session workspace root
- [x] 2.4 Add `requires_root_marker` to `LanguageDefinition` and the resolver, defaulting to the existing fallback behavior for every language that does not set it
  - The nested-path marker needed no resolver change at all: detection already tests `current.join(marker)`, and `Path::join` accepts a relative path with a separator. A second detection mechanism would have been invented for a case the existing one already handled.
- [x] 2.5 Add the `ProjectRootError` variant and its own safe reason code. It must not fold into the generic project-root-unavailable code — "no compilation database" is actionable and "project root unavailable" is not
  - This was a real hole, not a formality: every `process_launch` failure collapsed to `not_configured`, which points a user at the settings page when the thing to fix is their build system. `CodeIntelligenceApiError::MissingProjectMarker` and the `missing_project_marker` query reason now carry it through.
- [x] 2.6 Confirm Rust and TypeScript root detection results are unchanged, including the two-nested-TypeScript-roots case
  - `project_root_tests` **13 passed**, the seven pre-existing ones unmodified.

## 3. Registry entries

- [x] 3.1 Add the Go entry: `gopls`, no startup arguments, `go.mod`, `.go` to `go`, and a fixture project
- [x] 3.2 Add the Python entry: `basedpyright-langserver` then `pyright-langserver`, `--stdio`, the four ranked markers, `.py` and `.pyi` to `python`, and a fixture project
- [x] 3.3 Add the C/C++ entry: `clangd`, no startup arguments, `compile_commands.json` then `build/compile_commands.json`, `requires_root_marker`, the extension mappings with `.h` to `c`, and a fixture project carrying a compilation database
- [x] 3.4 Extend `registry_tests.rs` for the new entries, and add a test that a language declaring `requires_root_marker` also declares at least one marker — the combination that would otherwise make the language permanently unusable
  - Also added a test that no marker contains a backslash. A marker written that way would resolve on Windows and silently stop matching on macOS and Linux — the same platform trap that has bitten path handling here before.
- [x] 3.5 Confirm the extension-uniqueness assertion still holds and that no new extension collides with an existing one
  - It holds. Two pre-existing tests used `go` as their stand-in for "unregistered" and had to move to `ruby`, which is the assertion doing its job rather than a problem.

## 4. Localization and surfaces

- [x] 4.1 Add `lspSettings.language.{go,python,cpp}` and the new reason-code string to `src/i18n/locales/{en,zh-CN,zh-TW,ja,ko}.json`, and extend `lsp-settings-localization.test.ts`
  - Three keys per locale, +3 lines per file, no reformatting. No reason-code string — see 4.2.
- [x] 4.2 Add the new safe reason code to the frontend `lspSafeReasonCodes` list and the Rust DTO reason enum
  - **Not needed, and the task was wrong to assume it.** Query reason codes are plain strings on the Agent tool result (`bootstrap/code_intelligence.rs:238`), not `LspSafeReasonCodeDto` values — `unsupported_language` and `not_configured` are already there and in neither enum. `LspSafeReasonCodeDto` covers discovery, server-test, and status reasons, none of which this change touches. Adding `missing_project_marker` to it would have put a value in a vocabulary nothing produces.
- [x] 4.3 Verify no frontend component changed. If one had to, that is a defect in `extend-lsp-language-registry`, not work for this change — record it rather than absorbing it
  - **The only frontend diff is five locale files and one localization test.** No component, no service, no type, no adapter. That is the property the registry was built to buy, now demonstrated rather than asserted.

## 5. Documentation

- [x] 5.1 Update the supported-server tables and install instructions in `docs/user-guide/{en,zh-CN}/src/lsp-code-intelligence.md` for the three languages, including that C/C++ needs a compilation database and what to do when it is missing
  - Also states why Python prefers the fork, and adds a troubleshooting entry separating "no compilation database" from an installation problem: discovery reports `clangd` as available in both cases.
- [x] 5.2 Update `docs/developer-guide/{,zh-CN/}src/lsp-code-intelligence.md` for the marker precedence rule and `requires_root_marker`, and correct the extension-limits section, which currently says these three are excluded
  - Documents that marker order is *not* meaningful, which is the opposite of what an earlier draft of this change specified. The extension-limits section now names Java as the only remaining roadmap language and says which assumption it breaks.
- [x] 5.3 Run `npm run docs:check`, keeping sentence-final punctuation outside the bold delimiters in Chinese text
  - Passes. Also corrected halfwidth punctuation in one added Chinese paragraph — the user guide uses fullwidth throughout and the developer guide does not.

## 6. Verification

- [x] 6.1 `npm run lint:ci` — passes
- [x] 6.2 `npm run test` — **318 files / 1660 tests passed**
- [x] 6.3 `npm run build` — passes; 16 lazy chunks, main static closure 142.8 KiB gzip
- [x] 6.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` — passes
- [x] 6.5 `cargo check --workspace` — passes
- [x] 6.6 `cargo clippy --workspace --all-targets -- -D warnings` — passes
- [x] 6.7 `npm run native:panic:check` — passes
- [x] 6.8 `cargo test --workspace` — **4569 passed** in the main suite (baseline 4551); `tests/architecture.rs` **54 passed**
- [x] 6.9 `npm run architecture:check`, `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`
  - Architecture 54 passed with **no budget change needed** — the services subtree did not move, because the frontend diff is locale data. Contracts 16, coverage policy 5, version 9.
- [x] 6.10 `npx playwright test` — **184 passed**
- [x] 6.11 `npm run desktop:unit:test`, then `npm run test:desktop:build` **before** any desktop layer — the layer scripts do not rebuild, and running one against a stale binary reports the pre-fix failure and reads like the fix did not work
  - `desktop:unit:test` **56 passed**. `npm run test:desktop` builds as its first step, so running the whole suite satisfies the ordering.
- [x] 6.12 `npm run test:desktop`, reporting each layer per platform and marking macOS and Linux NOT RUN when they are
  - Windows x64, every layer **PASSED**: smoke 25/25, cli-terminal 1/1, cli-management 2/2, session-workspace 1/1, dialogs 1/1, settings-persistence 2/2, agent-mcp 1/1. **macOS and Linux: NOT RUN** on this host; their results must come from their own CI runners.
- [x] 6.13 `openspec validate add-lsp-go-python-cpp --strict` and `openspec validate --specs --strict`
  - Change valid; main specs **138 passed, 0 failed**.
- [x] 6.14 Simulate the archive merge with `buildUpdatedSpec`; `validate --strict` does not model it
  - `lsp-server-management +3 ~1`. No warnings, no unaccounted content.

## 7. Acceptance

- [x] 7.1 Confirm every suite from task 1.1 passes unchanged, and that new tests cover only new behavior: multi-marker detection, nested-path markers, the required-marker rule, the three languages' declarations, and Python candidate selection
  - Same filters as the baseline: `migration_fixture` **13 -> 13**, `native_lsp` **1 -> 1**, `code_intelligence` **179 -> 190**, the LSP Playwright spec **1 -> 1**. Full workspace **4551 -> 4569**.
  - The +11 are all new behavior. Three pre-existing assertions changed, each because the change made them false rather than because they were in the way: two used `go` as their stand-in for an unregistered language, and one asserted the loaded configuration covers exactly the two seeded rows.
- [x] 7.2 Confirm no database migration was added, and that the highest migration number matches task 1.2
  - Highest is still **86**, and `git diff src-tauri/src/platform/database/` is **empty**. The claim the previous change made about its own value is now demonstrated.
- [x] 7.3 Confirm the diff contains no new frontend component and no new per-language branch anywhere in `src/`
  - `git status src/` lists only the five locale files and the localization test.
- [x] 7.4 On a host with none of the three servers installed, confirm each reports unavailable with a reason rather than failing a command, and that the desktop LSP spec still passes
  - `domain-lsp.e2e.mjs` passes inside desktop-smoke against the real binary. Its discovery assertion now checks every reported server against the descriptors the same build returns, so the three new languages were exercised rather than skipped, and each reported unavailable with a reason instead of failing the command.
  - `command_tests::discovery_and_server_test_return_safe_unavailable_results` covers the same shape at the unit level, distinguishing the two languages with a broken override from the three simply not installed.
