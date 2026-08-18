## 1. Record the baseline

- [x] 1.1 Record the current `cargo test --manifest-path src-tauri/Cargo.toml` total test count — it must not fall at any step
      — 3,590 on `main` at `5449ce56`, measured while implementing the panic-shortcut gate
- [ ] 1.2 Record incremental `cargo check` timing after touching one context file, on an otherwise idle machine, as the figure a later extraction will be measured against. Do not claim a win from this change
- [ ] 1.3 Record the current `npm run package` output layout and the sidecar binary's resolved path
      — NOT DONE before the change, and ticked in error at first. There is now no pre-change
      baseline to diff the artifact layout against, so 3.2 has to verify the layout against the
      documented expectation instead, which is a weaker check. Recorded rather than quietly dropped

## 2. Workspace skeleton, one member

- [x] 2.1 Add a root `Cargo.toml` with `[workspace]`, `members = ["src-tauri"]`, and `resolver` matching the current edition's default
- [x] 2.2 Move all 71 distinct dependency version declarations (87 counted declaration lines; several crates appear in more than one section) to `[workspace.dependencies]`; `src-tauri/Cargo.toml` switches each to `workspace = true`, preserving every `default-features` and `features` setting exactly
- [x] 2.3 Add an empty `[workspace.lints]` section with a comment recording why the panic-shortcut lints are deliberately not there — `[lints]` has no target selectivity and would fail on the ~9,560 test-code sites
- [x] 2.4 Keep `crate-type = ["staticlib", "cdylib", "rlib"]` on the Tauri crate only
- [x] 2.5 `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass with the 1.1 test count unchanged
      — check and clippy pass clean; test 3,590 passed / 0 failed, matching the 1.1 baseline exactly

### Surfaced while building the skeleton

- [x] 2.6 Move `[profile.release]` to the workspace root. Cargo only *warns* that profiles in a
      non-root member are ignored, and `distributable_release_profile_stays_optimized` reads the
      manifest text rather than the resolved profile, so every release optimisation would have been
      lost behind a passing gate
- [x] 2.7 Point `scripts/prepare-permission-hook-sidecar.mjs` at `<root>/target` instead of
      `<root>/src-tauri/target` — Cargo writes to the workspace root, and the failure surfaces as a
      missing staging source rather than as a path error
- [x] 2.8 Add `/target/` to `.gitignore`; the member's `src-tauri/target/` entry no longer covers it
- [x] 2.9 Repoint the Documentation job's cache key from `src-tauri/Cargo.lock` to `Cargo.lock`.
      `hashFiles()` on a missing path returns a constant, so this would have become a cache that
      never invalidates rather than an error
- [x] 2.10 Record that adopting a workspace adds ten resolved packages (785 → 795), isolated to the
      workspace itself rather than to hoisting: a bare workspace with zero hoisted dependencies
      resolves 795, both resolver 2 and 3 give 795, and re-resolving the pristine manifest still
      gives 785. All ten are build-only — `cargo tree --edges normal` finds no path to `jiff`
- [x] 2.11 Fix `.github/workflows/package.yml`: nine references to
      `src-tauri/target/${{ matrix.rust_target }}/release/bundle` (Windows signing, Windows
      Authenticode verification, macOS notarization verification, artifact upload) all assumed
      the pre-workspace path. Not caught by any check run so far — `npm run package` locally and
      `ci.yml` both avoid the code path that reads this. Would have failed only when a release
      was actually cut. Fixed to `target/${{ matrix.rust_target }}/release/bundle` at all nine sites

## 3. Prove the packaging chain before extracting anything

- [ ] 3.1 `npm run tauri -- dev` starts
- [ ] 3.2 `npm run package` produces the same artifact layout as the 1.3 baseline
- [ ] 3.3 `npm run desktop:unit:test` and `npm run test:desktop` pass on this machine, with the result reported per platform rather than generalised
- [x] 3.4 Confirm `src-tauri/tests/architecture.rs` path and subtree budgets still resolve, and that no subtree is counted twice under the new layout
      — 41/41 passing, but only after fixing `distributable_release_profile_stays_optimized`
      itself: it read `src-tauri/Cargo.toml` for the release profile, which is exactly the
      manifest that stopped carrying one once 2.6 moved it to the workspace root. The test now
      reads the workspace root and additionally asserts the member does *not* declare a profile,
      so a future regression there is caught instead of silently ignored again

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
