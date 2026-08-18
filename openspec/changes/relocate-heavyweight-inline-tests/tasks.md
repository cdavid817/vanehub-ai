## 1. Establish the corrected premise before moving anything

- [ ] 1.1 Confirm both target files are already `#[cfg(test)] mod tests;` child modules, not inline `mod tests` blocks, and record the declaring lines
- [ ] 1.2 Confirm the library's public API is `pub fn run()` alone and that `bootstrap`, `commands`, `contexts`, and `platform` are private at the crate root
- [ ] 1.3 Confirm `mod test_support` is `#[cfg(test)]`, so it does not exist in the library artifact an external test crate links against
- [ ] 1.4 Record the visibility of every item the two files reach outside their own module, and confirm the maximum is `pub(crate)` — including the three `pub(super)` items in `sqlite_repository`
- [ ] 1.5 Confirm no existing file in `src-tauri/tests/` references the library crate, so the directory has no precedent for testing lib internals
- [ ] 1.6 Conclude and record whether relocation to `src-tauri/tests/` is possible; if it is not, withdraw that goal in the proposal rather than deferring it

## 2. Capture the baseline

- [ ] 2.1 Record the physical line count of both files and of their containing subtrees
- [ ] 2.2 Capture the full unsorted test listing via `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list`, and a sorted list of bare test names, and save both for the post-split comparison

## 3. Split `sessions/infrastructure/tests.rs`

- [ ] 3.1 Create `sessions/infrastructure/tests/` and move each subject range from design.md into its named module verbatim, one contiguous cut per file
- [ ] 3.2 Leave `Fixture`, `fixture()`, `session_record()`, `message_record()`, `correlated_message_record()`, `usage_record()`, the evidence/logging port doubles, and the tests that sit among them in `tests.rs`
- [ ] 3.3 Declare the new modules in `tests.rs` and give each child `use super::*;` as its first import
- [ ] 3.4 Confirm the six ranges plus the retained content partition the original file exactly, with no line duplicated or dropped

## 4. Split `agent_runtime/application/tests.rs`

- [ ] 4.1 Create `agent_runtime/application/tests/` and move each subject range from design.md into its named module verbatim
- [ ] 4.2 Leave `FakeWorld` and its port implementations, `FailingExecutionTelemetry`, `FakeMessageTerminalCompletions`, the agent builders, and the two tests interleaved with the fixture in `tests.rs`
- [ ] 4.3 Declare the new modules in `tests.rs` and give each child `use super::*;` as its first import
- [ ] 4.4 Confirm the seven ranges plus the retained content partition the original file exactly

## 5. Prove the split was pure

- [ ] 5.1 Confirm `cargo check --manifest-path src-tauri/Cargo.toml` compiles with no visibility widening anywhere; if any item needs `pub(super)`, record which and why rather than applying it silently
- [ ] 5.2 Re-capture the sorted bare test-name list and assert it is byte-identical to the 2.2 baseline — not merely the same length
- [ ] 5.3 Re-capture the unsorted listing and confirm it differs only in the module-path prefix of moved tests, with names in unchanged relative order
- [ ] 5.4 Confirm `git diff` touches no production Rust file and no `pub` keyword

## 6. Update budgets and verify

- [ ] 6.1 Lower the two `NATIVE_PATH_BUDGETS` entries to the measured post-split counts, leaving the `tooling/skills/application/tests.rs` entry untouched
- [ ] 6.2 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged total test count
- [ ] 6.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` passes
- [ ] 6.4 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes
- [ ] 6.5 `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] 6.6 `npm run architecture:check` passes
- [ ] 6.7 `openspec validate relocate-heavyweight-inline-tests --strict` and `openspec validate --specs --strict` pass
