## 1. Record the baseline

- [x] 1.1 Capture the violation counts for `--lib`, `--bins`, and `--all-targets` so the production/test split stays checkable, and record the per-file list of the 35 production sites

## 2. Add the gate

- [x] 2.1 Add an npm script running `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins -- -D clippy::unwrap_used -D clippy::expect_used`
- [x] 2.2 Add it as a step in the `Rust` CI job, placed after the existing clippy step so it reuses the same build cache
- [x] 2.3 Confirm `src-tauri/Cargo.toml` is untouched — no `[lints]` section — and record in the change why, so a later contributor does not add one and break the test build

## 3. Whitelist the existing production violations

- [x] 3.1 Add `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of each of the 11 files, with a comment giving the site count and naming the change expected to retire it
- [x] 3.2 Mark the two `domain`-layer files explicitly — `retrieval/domain/code_redaction.rs` and `task_orchestration/domain/graph.rs`, nine sites between them — as the first candidates for removal, since a panic shortcut in a pure domain layer is the least defensible placement
- [x] 3.3 Confirm no file received a line-level allow and no crate-level allow was added

## 4. Prove the gate bites and the exemption holds

- [x] 4.1 The new command passes on the unmodified tree
- [x] 4.2 Temporarily add an `unwrap()` to a production file **not** on the whitelist, confirm the command fails naming that file, then revert
- [x] 4.3 Temporarily add an `unwrap()` to a test module, confirm the command still passes — this is the property the ticket's design could not have provided
- [x] 4.4 Confirm `cargo clippy --all-targets -- -D warnings` still passes, unchanged in meaning

## 5. Verification

- [x] 5.1 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged test count
- [x] 5.2 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo check --manifest-path src-tauri/Cargo.toml` pass
- [x] 5.3 `npm run architecture:check` passes
- [x] 5.4 `openspec validate freeze-panic-shortcuts-in-production-code --strict` and `openspec validate --specs --strict` pass
- [x] 5.5 Record the remaining whitelist as the scope of the follow-up change, replacing the ticket's estimate of roughly 3,700 sites with the measured 35
