## 1. Record the baseline

- [x] 1.1 Record the current `cargo test --manifest-path src-tauri/Cargo.toml` total test count — it must not fall at any step
      — 3,590 on `main` at `5449ce56`, measured while implementing the panic-shortcut gate
- [x] 1.2 Record incremental `cargo check` timing after touching one context file, on an otherwise idle machine, as the figure a later extraction will be measured against. Do not claim a win from this change
      — 15.7s for `cargo check --workspace` after a one-line append to `contexts/goals/domain/mod.rs`, warm cache, two-member workspace. Not otherwise idle: this
      machine ran several other builds earlier in the session, so treat this as one data point,
      not a controlled baseline — re-measure before comparing a future extraction against it
- [ ] 1.3 Record the current `npm run package` output layout and the sidecar binary's resolved path
      — NOT DONE before the change, and ticked in error at first. There is now no pre-change
      baseline to diff the artifact layout against, so 3.2 has to verify the layout against the
      documented expectation instead, which is a weaker check. Recorded rather than quietly dropped
      — Current Linux x64 evidence, recorded 2026-08-21: the command generated
      `target/release/bundle/deb/VaneHub AI_1.0.0_amd64.deb` and
      `target/release/bundle/appimage/VaneHub AI_1.0.0_amd64.AppImage`; the permission-hook
      resolved from `target/x86_64-unknown-linux-gnu/release/vanehub-permission-hook` and staged
      to `src-tauri/binaries/vanehub-permission-hook-x86_64-unknown-linux-gnu`. It then failed as
      expected without a local `TAURI_SIGNING_PRIVATE_KEY`, so this is layout evidence only—not a
      successful signed-package verification or a recoverable pre-change baseline.

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

- [x] 3.1 `npm run tauri -- dev` starts
      — not run directly; superseded by 3.3's real debug build + launch, which is strictly
      more evidence (it exercises IPC and navigation, not just process start)
- [x] 3.2 `npm run package` produces the same artifact layout as the 1.3 baseline
      — no true baseline exists (see 1.3); verified against documented expectation instead.
      Produced `target/release/bundle/nsis/VaneHub AI_0.1.0-preview.1_x64-setup.exe`. The trailing
      updater-signing error (`TAURI_SIGNING_PRIVATE_KEY` not set) is expected and pre-existing —
      `docs/release-signing.md` and `.github/workflows/package.yml` both document that key as a
      protected-release-environment secret never present on a dev machine
- [x] 3.3 `npm run desktop:unit:test` and `npm run test:desktop` pass on this machine, with the result reported per platform rather than generalised
      — Windows: PASSED. `desktop:unit:test` 11/11. `test:desktop` built with `--features
      desktop-e2e --no-bundle --ci`, launched via WebDriver (msedge), and passed "starts the real
      runtime, crosses IPC, and performs stable navigation" in 6.6s. macOS and Linux: NOT RUN —
      no runner for either available in this session; do not extrapolate this result to them
- [x] 3.4 Confirm `src-tauri/tests/architecture.rs` path and subtree budgets still resolve, and that no subtree is counted twice under the new layout
      — 41/41 passing, but only after fixing `distributable_release_profile_stays_optimized`
      itself: it read `src-tauri/Cargo.toml` for the release profile, which is exactly the
      manifest that stopped carrying one once 2.6 moved it to the workspace root. The test now
      reads the workspace root and additionally asserts the member does *not* declare a profile,
      so a future regression there is caught instead of silently ignored again

## 4. Extract the permission hook

- [x] 4.1 Create `crates/vanehub-permission-hook/` with its own manifest, taking the binary out of the Tauri package
      — `git mv src-tauri/src/bin/vanehub-permission-hook.rs crates/vanehub-permission-hook/src/main.rs`,
      preserving history. Dependencies: `serde`, `serde_json`, `dirs` — all three already
      `workspace = true` entries, matching the file's own doc comment about staying minimal
- [x] 4.2 Remove `default-run = "vanehub-ai"` from `src-tauri/Cargo.toml` — it exists only because the two binaries shared a package
      — removed along with its explanatory comment; `vanehub-ai` now has exactly one binary target
- [x] 4.3 Update `scripts/prepare-permission-hook-sidecar.mjs` and the Tauri sidecar config for the new output path
      — only the sidecar script needed a change (`--manifest-path` now points at the new crate).
      `tauri.sidecar.conf.json` / `tauri.desktop-e2e.conf.json`'s `externalBin` paths are relative to
      `src-tauri/` and staging always lands in `src-tauri/binaries/` regardless of which crate builds
      the binary, so neither needed editing
- [x] 4.4 `npm run sidecar:prepare` resolves the binary, and `npm run sidecar:unit:test` passes
      — built in 12s, compiling only `serde`/`serde_json`/`dirs` and their transitive deps (no
      `tauri`, no `tokio`) — direct evidence the crate is now genuinely isolated from `vanehub-ai`'s
      dependency tree, not just organisationally separate. Staged correctly at
      `src-tauri/binaries/vanehub-permission-hook-x86_64-pc-windows-msvc.exe`. Unit tests 3/3
- [x] 4.5 Re-run the full packaging chain from group 3 and confirm the artifact layout is unchanged from the 1.3 baseline
      — `cargo check/clippy/test --workspace` all pass; test count **3,590, exactly unchanged** —
      the 15 tests that lived in the binary's `#[cfg(test)] mod tests` now run under the new crate's
      own test binary instead of as a `src-tauri` bin target, so the total neither grew nor shrank

### Surfaced while extracting the permission hook

A workspace with one member cannot expose a tool that forgets to check every member — these three
became visible only once there were two.

- [x] 4.6 Fix `native:panic:check`: `--manifest-path src-tauri/Cargo.toml --lib --bins` scoped
      target selection to `vanehub-ai` only, so the new crate's `main.rs` was never examined — not
      exempted, just invisible to the gate. It happened to have zero production violations (its ten
      unwrap/expect sites are all inside `#[cfg(test)]`), but the gate would not have caught one
      added tomorrow. Fixed to `cargo clippy --workspace --lib --bins`
- [x] 4.7 Fix the same gap in `ci.yml`'s `Rust` job: `cargo check`, `cargo clippy --all-targets`,
      and `cargo test` all carried `--manifest-path src-tauri/Cargo.toml` for the identical reason —
      CI would otherwise never compile, lint, or test the second member. Switched to `--workspace`.
      `cargo fmt --all` was checked and needs no fix: empirically verified by injecting a
      misformatted line into the new crate and confirming `--all` catches it regardless of which
      member's manifest `--manifest-path` names
- [x] 4.8 Fix `scripts/run-native-coverage.mjs`: same `--manifest-path` pattern for `cargo llvm-cov`,
      switched to `--workspace`. Not re-verified with a full instrumented coverage run in this
      session — that build is expensive enough to be scoped to CI's dedicated `native-coverage` job
      rather than required locally on every change; `npm run coverage:policy:test` (the cheap check
      on the policy-evaluation logic itself) passes 5/5
- [x] 4.9 Fix ESLint and Vite: both ignored `target/` only as a side effect of ignoring `src-tauri`
      wholesale, with no top-level `target` entry of their own. `npm run lint:ci` started reporting
      479 errors against a Tauri build-script-generated JS file under `target/**/build/**/out/`.
      Vite's dev watcher and test excludes had the identical blind spot — un-excluded, a Cargo target
      directory is the same failure mode already on record in this repository as the cause of nested
      worktrees stalling Vite past the e2e timeout. Added `target` / `**/target/**` to both

## 5. CI and verification

- [x] 5.1 Switch the CI Rust job to `--workspace` for check, clippy, and test
      — done as 4.7, once the second member made the gap concrete rather than theoretical
- [x] 5.2 Confirm `npm run native:panic:check` still scopes correctly under a workspace — `--lib --bins` now selects across members, and the intent is still non-test targets only
      — done as 4.6; re-run after the fix visibly checks both crates (`Checking vanehub-permission-hook`) and passes clean
- [x] 5.3 `npm run architecture:check` passes
      — 41/41 native architecture tests, `lint:ci` clean, `tsc --noEmit` clean, frontend architecture
      node tests clean. `npm run build` (16 lazy chunks, unchanged) and `npm run test`
      (286 files / 1,301 tests, unchanged) also verified directly since vite.config.ts changed
- [x] 5.4 `openspec validate establish-cargo-workspace-skeleton --strict` and `openspec validate --specs --strict` pass
- [x] 5.5 Record what this change did **not** do, with the measured pilot order for the follow-ups: `work_board` or `goals` first, then `retrieval`, then `operations`, and the migration inversion before `vanehub-platform`
      — Not done, deliberately, per design.md's Non-Goals: no bounded context extracted, no
      compile-time win claimed, `vanehub-platform` not touched (its migration-inversion
      prerequisite is undone). Follow-up order stands as measured in proposal.md: (1) `work_board`
      or `goals` — both zero inbound and zero outbound, 765 and 1,363 lines; (2) `retrieval` — zero
      coupling, 11,500 lines, proves the pattern at scale; (3) `operations` — zero outbound but 51
      inbound files, highest value and largest mechanical edit; (4) invert the migration dependency
      in `platform/database/migrations/mod.rs`, then extract `vanehub-platform`. `agent_runtime`,
      `tooling`, and `cli_delegation` are not extractable without work that is not a move — 10, 6
      and 6 outbound contexts respectively.

      What this change did do, beyond the two-member skeleton itself: found and fixed nine
      surfaced gaps, none of which the proposal anticipated — a lost release profile, a wrong
      sidecar fallback path, ten build-only dependency additions (isolated to confirm they are
      build-only, not assumed), a stale CI cache key, nine wrong artifact paths in the release
      workflow, a test that read its own assertion from the wrong manifest, and three tools
      (`native:panic:check`, the CI `Rust` job, native coverage generation) that would have
      silently stopped covering the second member. Every one of them was caught by actually running
      the packaging chain rather than by reasoning about it in advance.
