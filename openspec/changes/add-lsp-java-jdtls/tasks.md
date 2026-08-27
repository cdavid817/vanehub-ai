## 1. Baseline

- [x] 1.1 Record the pre-change pass state of `cargo test --workspace code_intelligence`, the frontend LSP vitest files, and the settings vitest files
  - Baseline on `d514cb84`: `code_intelligence` **202**, the LSP/settings vitest files **752**.
- [x] 1.2 Record how many languages the registry declares and which tests assert that count, so the fifth is a deliberate diff
  - Five before Java. **Every count assertion derives from `LANGUAGE_DEFINITIONS.len()`**, so the sixth entry broke none of them — the registry paying off a third time.

## 2. The launch shape

- [x] 2.1 Add `LaunchShape` to the registry with `Executable` and `Interpreter` variants, and give the four existing entries `Executable`. **No existing behavior may change here** — this step is a field with one value in use
  - Five existing entries, not four; the task list predated C/C++ landing. All five declare `Executable` and none of their tests changed.
- [x] 2.2 Add `InterpreterLaunch`: interpreter candidates, launcher directory and name pattern, per-platform configuration directory, argument template, and the prerequisite's display name
- [x] 2.3 Add the argument template's placeholder set — launcher, configuration directory, workspace data directory — as an enum, so an unresolved placeholder is a compile-time-known case rather than a string that failed to substitute
- [x] 2.4 Add a registry test that every `Interpreter` entry declares a launcher pattern and a configuration directory for every platform it claims

## 3. Discovery

- [x] 3.1 Split discovery on the launch shape. The executable path is unchanged; add the interpreter path
- [x] 3.2 Add the five reasons: prerequisite missing, install directory not set, override missing, launcher not found, ambiguous install. Order them so the first missing thing is what gets reported
- [x] 3.3 Resolve the launcher by prefix-and-suffix match in one declared directory, no recursion, refusing on zero or several
- [x] 3.4 Add tests for each of the five, plus the success case reporting the resolved launcher rather than the directory
- [x] 3.5 Confirm the existing executable-shaped discovery tests pass unchanged. If one needed editing, the split was not behavior-preserving
  - **Held.** All eight pass untouched; the seven new ones are interpreter-shaped. One *command* test did need editing — it asserted every unconfigured language reports `ExecutableNotFound`, which is exactly the thing this change makes untrue, and it now accepts either interpreter reason for Java because which one appears depends on whether the runner has a JDK.

## 4. Launch

- [x] 4.1 Resolve the template where the workspace is known, and append the user's configured startup arguments after it
- [x] 4.2 Derive the per-workspace data directory from a hash of the canonical root, under the app data directory
- [x] 4.3 Remove that directory on trust revocation, beside the process stop. **Not** on idle shutdown — it is the server's index, and discarding it on idle makes the next start pay for a full re-index
- [x] 4.4 Add a test that revocation removes the directory, asserting the directory is gone rather than that a call was made. This is the one failure here with a privacy shape
  - Six tests: two workspaces never share, two languages never share, the workspace path never reaches the directory name, removal is asserted on the filesystem, a sibling workspace survives, and removing a directory that was never created is uneventful — because revocation runs for every registered language and most have none.

## 5. The Java entry

- [x] 5.1 Register Java: `jdtls`, JVM interpreter, the four project-root markers, `.java` to `java`, disabled by default
- [x] 5.2 Declare its fixture project for the isolated server test
- [x] 5.3 Add the registry tests the other languages have: the entry exists, its markers are declared, its extension maps
- [x] 5.4 Add locale strings for the language name and the new discovery reasons, in all five bundles, and extend `lsp-settings-localization.test.ts`

## 6. The settings surface

- [x] 6.1 Extend the language descriptor with the launch shape, so the card learns what an override means from the backend rather than from the language id. **A `language === "java"` check anywhere in the frontend fails this task**
  - `overrideTarget` and `prerequisite` on the descriptor. No frontend file names Java.
- [x] 6.2 Render the directory-shaped override with its own label and validation
- [x] 6.3 Present the prerequisite state distinctly from an unset directory and from a directory without a launcher
- [x] 6.4 Update the Web/mock registry with Java and the launch shape, keeping the adapters' contract shape identical
- [x] 6.5 Add a component test that the override control follows the reported shape, driven by a descriptor rather than by a language name
  - The test uses **`elixir`**, deliberately not Java. If the card branched on a language id it would render an executable override and the test would fail — which is the only way to assert the absence of a check rather than to assert around it.

## 7. Documentation

- [x] 7.1 User guides: Java in the language table, what to install, how to point at it, and what each of the three unavailable states means
- [x] 7.2 Developer guides: the launch shape, the override's shape-dependent meaning, the launcher resolution rule, and the data directory's lifetime
- [x] 7.3 Correct the developer guides' extension-limits section, which currently says Java "does not fit"
  - Replaced with what the shape is and the two rules whose opposites look reasonable. The exclusion list's "downloaded servers" now says why it is still there and which change removes it.
- [x] 7.4 Run `npm run docs:check`

## 8. Verification

- [x] 8.1 `npm run lint:ci`
- [x] 8.2 `npm run test`
- [x] 8.3 `npm run build`
- [x] 8.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 8.5 `cargo check --workspace`
- [x] 8.6 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 8.7 `npm run native:panic:check`
- [x] 8.8 `cargo test --workspace` — **4,610 passed, 0 failed.** A fully green run, including both tests that were load-sensitive in the previous two changes.
- [x] 8.9 `npm run architecture:check`, `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`
  - One budget moved: `src/services` +20, for the descriptor's two new fields and the mock's Java entry. That entry is what makes the Web adapter exercise the `install_directory` branch at all.
- [ ] 8.10 `npx playwright test`
- [ ] 8.11 `npm run desktop:unit:test`, then `npm run test:desktop`
- [ ] 8.12 `openspec validate add-lsp-java-jdtls --strict` and `openspec validate --specs --strict`
- [ ] 8.13 Simulate the archive merge with `buildUpdatedSpec`

## 9. Acceptance

- [ ] 9.1 Confirm the four existing languages behave identically: their discovery tests pass unchanged and no executable-shaped code path was rewritten
- [ ] 9.2 Confirm no frontend file names Java. The card, the override control, and the reason display all read a descriptor
- [ ] 9.3 Confirm no database migration was added
- [ ] 9.4 Measure whether the isolated server test completes for `jdtls` within the current fixed deadline, and either record that it does or move the deadline into the registry with the measurement that justified it
- [ ] 9.5 Confirm the deferred half is deferred and not forgotten: state plainly in the change record that Java must be installed by hand until `manage-language-server-installation` lands
