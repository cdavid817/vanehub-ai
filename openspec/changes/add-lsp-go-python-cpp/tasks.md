## 1. Baseline

- [ ] 1.1 Record the pre-change pass state of the suites this change must not disturb: `cargo test --workspace code_intelligence`, `cargo test --workspace native_lsp`, `cargo test --workspace migration_fixture`, the six frontend LSP vitest files, and `npx playwright test tests/e2e/lsp-settings.spec.ts`. Unset `all_proxy`/`ALL_PROXY` and pin `PLAYWRIGHT_PORT` for the last one
- [ ] 1.2 Confirm the first claim this change depends on: adding a language needs no database migration. Note the current highest migration number and expect it to be unchanged at the end

## 2. Root detection

- [ ] 2.1 Write failing tests for marker precedence: nearest ancestor wins over marker strength, and declared order breaks a tie inside one directory
- [ ] 2.2 Make `ProjectRootResolver` report which marker matched, so the precedence rule is observable rather than inferred
- [ ] 2.3 Write a failing test for `requires_root_marker`: a language that declares it and finds no marker fails resolution instead of falling back to the session workspace root
- [ ] 2.4 Add `requires_root_marker` to `LanguageDefinition` and the resolver, defaulting to the existing fallback behavior for every language that does not set it
- [ ] 2.5 Add the `ProjectRootError` variant and its own safe reason code. It must not fold into the generic project-root-unavailable code — "no compilation database" is actionable and "project root unavailable" is not
- [ ] 2.6 Confirm Rust and TypeScript root detection results are unchanged, including the two-nested-TypeScript-roots case

## 3. Registry entries

- [ ] 3.1 Add the Go entry: `gopls`, no startup arguments, `go.mod`, `.go` to `go`, and a fixture project
- [ ] 3.2 Add the Python entry: `basedpyright-langserver` then `pyright-langserver`, `--stdio`, the four ranked markers, `.py` and `.pyi` to `python`, and a fixture project
- [ ] 3.3 Add the C/C++ entry: `clangd`, no startup arguments, `compile_commands.json` then `build/compile_commands.json`, `requires_root_marker`, the extension mappings with `.h` to `c`, and a fixture project carrying a compilation database
- [ ] 3.4 Extend `registry_tests.rs` for the new entries, and add a test that a language declaring `requires_root_marker` also declares at least one marker — the combination that would otherwise make the language permanently unusable
- [ ] 3.5 Confirm the extension-uniqueness assertion still holds and that no new extension collides with an existing one

## 4. Localization and surfaces

- [ ] 4.1 Add `lspSettings.language.{go,python,cpp}` and the new reason-code string to `src/i18n/locales/{en,zh-CN,zh-TW,ja,ko}.json`, and extend `lsp-settings-localization.test.ts`
- [ ] 4.2 Add the new safe reason code to the frontend `lspSafeReasonCodes` list and the Rust DTO reason enum
- [ ] 4.3 Verify no frontend component changed. If one had to, that is a defect in `extend-lsp-language-registry`, not work for this change — record it rather than absorbing it

## 5. Documentation

- [ ] 5.1 Update the supported-server tables and install instructions in `docs/user-guide/{en,zh-CN}/src/lsp-code-intelligence.md` for the three languages, including that C/C++ needs a compilation database and what to do when it is missing
- [ ] 5.2 Update `docs/developer-guide/{,zh-CN/}src/lsp-code-intelligence.md` for the marker precedence rule and `requires_root_marker`, and correct the extension-limits section, which currently says these three are excluded
- [ ] 5.3 Run `npm run docs:check`, keeping sentence-final punctuation outside the bold delimiters in Chinese text

## 6. Verification

- [ ] 6.1 `npm run lint:ci`
- [ ] 6.2 `npm run test`
- [ ] 6.3 `npm run build`
- [ ] 6.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 6.5 `cargo check --workspace`
- [ ] 6.6 `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] 6.7 `npm run native:panic:check`
- [ ] 6.8 `cargo test --workspace`
- [ ] 6.9 `npm run architecture:check`, `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`
- [ ] 6.10 `npx playwright test`
- [ ] 6.11 `npm run desktop:unit:test`, then `npm run test:desktop:build` **before** any desktop layer — the layer scripts do not rebuild, and running one against a stale binary reports the pre-fix failure and reads like the fix did not work
- [ ] 6.12 `npm run test:desktop`, reporting each layer per platform and marking macOS and Linux NOT RUN when they are
- [ ] 6.13 `openspec validate add-lsp-go-python-cpp --strict` and `openspec validate --specs --strict`
- [ ] 6.14 Simulate the archive merge with `buildUpdatedSpec`; `validate --strict` does not model it

## 7. Acceptance

- [ ] 7.1 Confirm every suite from task 1.1 passes unchanged, and that new tests cover only new behavior: marker precedence, the required-marker rule, the three languages' declarations, and Python candidate selection
- [ ] 7.2 Confirm no database migration was added, and that the highest migration number matches task 1.2
- [ ] 7.3 Confirm the diff contains no new frontend component and no new per-language branch anywhere in `src/`
- [ ] 7.4 On a host with none of the three servers installed, confirm each reports unavailable with a reason rather than failing a command, and that the desktop LSP spec still passes
