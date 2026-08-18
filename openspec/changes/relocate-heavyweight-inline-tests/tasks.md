## 1. Establish the corrected premise before moving anything

- [x] 1.1 Confirm both target files are already `#[cfg(test)] mod tests;` child modules, not inline `mod tests` blocks, and record the declaring lines — `sessions/infrastructure/mod.rs:27` and `agent_runtime/application/mod.rs:240`. The ticket's "inline test module" premise is wrong; the extraction it asks for is already done.
- [x] 1.2 Confirm the library's public API is `pub fn run()` alone and that `bootstrap`, `commands`, `contexts`, and `platform` are private at the crate root — `lib.rs:7-10` declare all four with a bare `mod`; `lib.rs:26` is the only `pub` item.
- [x] 1.3 Confirm `mod test_support` is `#[cfg(test)]` — `lib.rs:22-23`. An external test crate links the library built without `--cfg test`, so `TempDirectory` does not exist there, and every sessions fixture uses it.
- [x] 1.4 Record the visibility of every item the two files reach outside their own module, and confirm the maximum is `pub(crate)` — `contexts/mod.rs` publishes contexts as `pub(crate)`; `sqlite_repository.rs:1036`, `:1189`, `:1225` are `pub(super)`, visible only inside `infrastructure`.
- [x] 1.5 Confirm no existing file in `src-tauri/tests/` references the library crate — a search for `vanehub_ai_lib` across `src-tauri/tests/*.rs` returns nothing. `architecture.rs` parses source as text; the MCP tests drive subprocesses.
- [x] 1.6 Conclude and record whether relocation to `src-tauri/tests/` is possible — **it is not, for 0 of 131 tests.** Any of 1.2, 1.3, or 1.4 alone is sufficient. Enabling it would mean publishing the native internals as crate API, contradicting `openspec/project.md`. The goal is withdrawn in the proposal, not deferred.

## 2. Capture the baseline

- [x] 2.1 Record the physical line count of both files and of their containing subtrees — `sessions/infrastructure/tests.rs` 5,110 (42% of its 12,013-line subtree); `agent_runtime/application/tests.rs` 4,628.
- [x] 2.2 Capture the full unsorted test listing and a sorted list of bare test names — 3,543 tests total, 131 of them in the two target modules (64 sessions, 67 agent_runtime).

## 3. Split `sessions/infrastructure/tests.rs`

- [x] 3.1 Create `sessions/infrastructure/tests/` and move each subject range into its named module verbatim, one contiguous cut per file — 7 modules, 4,274 lines moved.
- [x] 3.2 Leave `Fixture`, `fixture()`, `session_record()`, `message_record()`, `correlated_message_record()`, `usage_record()`, the evidence/logging port doubles, and the tests that sit among them in `tests.rs`
- [x] 3.3 Declare the new modules in `tests.rs` and give each child `use super::*;` as its first import
- [x] 3.4 Confirm the ranges plus the retained content partition the original file exactly — asserted mechanically at split time (concatenating the slices reproduced the source byte-for-byte) and re-checked independently against `git show HEAD:` afterwards.

## 4. Split `agent_runtime/application/tests.rs`

- [x] 4.1 Create `agent_runtime/application/tests/` and move each subject range into its named module verbatim — 7 modules, 2,784 lines moved.
- [x] 4.2 Leave `FakeWorld` and its port implementations, `FailingExecutionTelemetry`, `FakeMessageTerminalCompletions`, the agent builders, and the two tests interleaved with the fixture in `tests.rs`
- [x] 4.3 Declare the new modules in `tests.rs` and give each child `use super::*;` as its first import
- [x] 4.4 Confirm the ranges plus the retained content partition the original file exactly — same two checks as 3.4.

## 5. Prove the split was pure

- [x] 5.1 Confirm the crate compiles with no visibility widening anywhere — `cargo test --lib --no-run` succeeded on the first attempt with no errors and no warnings. **No item's visibility was changed.** A grandchild module sees its ancestors' private items and private `use` bindings, so `use super::*;` resolved everything.
- [x] 5.2 Re-capture the sorted bare test-name list and assert it is byte-identical to the 2.2 baseline — `cmp` reports no difference across all 3,543 names.
- [x] 5.3 Re-capture the unsorted listing and confirm it differs only in the module-path prefix of moved tests — the 3,412 untouched tests are byte-identical in identical order; the 131 moved tests keep identical bare names and each destination module preserves their relative order.
- [x] 5.4 Confirm `git diff` touches no production Rust file and no `pub` keyword — only the two `tests.rs` files, the new `tests/` directories, and the budget entries in `architecture.rs`.

## 6. Update budgets and verify

- [x] 6.1 Lower the two `NATIVE_PATH_BUDGETS` entries to the measured post-split counts — 5,110 → 843 and 4,628 → 1,851, each with its reason recorded inline. The `tooling/skills/application/tests.rs` entry is untouched.
- [x] 6.2 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged total test count — 3,543 lib tests, exactly the baseline count. One failure, `playwright_sidecar::…::real_playwright_worker_bounds_page_operations_handoff_and_artifact_bytes`, is the known load sensitivity of issue #170 in a context this change never touches; it passes on an isolated re-run.
- [x] 6.2a Fix the architecture gate's test-code predicate — splitting the tests made `provider_neutral_layers_do_not_select_concrete_cli_providers` fire on three new modules whose `"codex-cli"` references had always been exempt, because the rule matched `ends_with("tests.rs")` and a `tests/` directory does not. Two other rules carried the same latent gap. All three now share an `is_test_source` helper, with its own fixture test.
- [x] 6.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` passes — rustfmt's only correction was one trailing blank line per parent, applied before the budgets were set.
- [x] 6.4 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes
- [x] 6.5 `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] 6.6 `npm run architecture:check` passes
- [ ] 6.7 `openspec validate relocate-heavyweight-inline-tests --strict` and `openspec validate --specs --strict` pass
