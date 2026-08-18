## 1. Record the baseline

- [ ] 1.1 Record the current `cargo test --manifest-path src-tauri/Cargo.toml` total test count — it must not fall at any step
- [ ] 1.2 Record incremental `cargo check` timing after touching one context file, on an otherwise idle machine, as the figure a later extraction will be measured against. Do not claim a win from this change
- [ ] 1.3 Record the current `npm run package` output layout and the sidecar binary's resolved path

## 2. Workspace skeleton, one member

- [ ] 2.1 Add a root `Cargo.toml` with `[workspace]`, `members = ["src-tauri"]`, and `resolver` matching the current edition's default
- [ ] 2.2 Move all 87 dependency version declarations to `[workspace.dependencies]`; `src-tauri/Cargo.toml` switches each to `workspace = true`, preserving every `default-features` and `features` setting exactly
- [ ] 2.3 Add an empty `[workspace.lints]` section with a comment recording why the panic-shortcut lints are deliberately not there — `[lints]` has no target selectivity and would fail on the ~9,560 test-code sites
- [ ] 2.4 Keep `crate-type = ["staticlib", "cdylib", "rlib"]` on the Tauri crate only
- [ ] 2.5 `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass with the 1.1 test count unchanged

## 3. Prove the packaging chain before extracting anything

- [ ] 3.1 `npm run tauri -- dev` starts
- [ ] 3.2 `npm run package` produces the same artifact layout as the 1.3 baseline
- [ ] 3.3 `npm run desktop:unit:test` and `npm run test:desktop` pass on this machine, with the result reported per platform rather than generalised
- [ ] 3.4 Confirm `src-tauri/tests/architecture.rs` path and subtree budgets still resolve, and that no subtree is counted twice under the new layout

## 4. Extract the permission hook

- [ ] 4.1 Create `crates/vanehub-permission-hook/` with its own manifest, taking the binary out of the Tauri package
- [ ] 4.2 Remove `default-run = "vanehub-ai"` from `src-tauri/Cargo.toml` — it exists only because the two binaries shared a package
- [ ] 4.3 Update `scripts/prepare-permission-hook-sidecar.mjs` and the Tauri sidecar config for the new output path
- [ ] 4.4 `npm run sidecar:prepare` resolves the binary, and `npm run sidecar:unit:test` passes
- [ ] 4.5 Re-run the full packaging chain from group 3 and confirm the artifact layout is unchanged from the 1.3 baseline

## 5. CI and verification

- [ ] 5.1 Switch the CI Rust job to `--workspace` for check, clippy, and test
- [ ] 5.2 Confirm `npm run native:panic:check` still scopes correctly under a workspace — `--lib --bins` now selects across members, and the intent is still non-test targets only
- [ ] 5.3 `npm run architecture:check` passes
- [ ] 5.4 `openspec validate establish-cargo-workspace-skeleton --strict` and `openspec validate --specs --strict` pass
- [ ] 5.5 Record what this change did **not** do, with the measured pilot order for the follow-ups: `work_board` or `goals` first, then `retrieval`, then `operations`, and the migration inversion before `vanehub-platform`
