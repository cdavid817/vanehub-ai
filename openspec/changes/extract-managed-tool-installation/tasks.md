## 1. Baseline

- [x] 1.1 Record the pre-change pass state of `cargo test --workspace cli_environment`, `cargo test --workspace vendor`, and `cargo test --workspace tooling`. These counts are the acceptance evidence for the move, so they have to exist before anything moves
  - Baseline on `f6f7cd3b`: `cli_environment` **13**, `vendor` **30**, `tooling::cli` **725**.
- [x] 1.2 Record the current physical line counts of `domain/trust.rs`, `infrastructure/vendor_downloader.rs`, `infrastructure/vendor_source.rs`, and the `tooling` subtree budget headroom, so the move can be shown to move code rather than duplicate it
  - `trust.rs` 286, `vendor_downloader.rs` 241, `vendor_downloader_tests.rs` 305, `vendor_source.rs` 319. `tooling` subtree 121,663 and no registered budget on it.

## 2. The subdomain

- [x] 2.1 Create `contexts/tooling/managed_install/{mod,api}.rs` with `domain` and `infrastructure`, registered in `tooling/mod.rs`
- [x] 2.2 Add `ManagedInstallError` with the variants the retrieval actually produces, and a `From<ManagedInstallError> for CliEnvironmentError` in the CLI adapter — the conversion belongs on the CLI side, so the shared error never learns CLI vocabulary

## 3. Move the retrieval policy

- [x] 3.1 Move `allowed_hosts`, `max_download_bytes`, `download_timeout_seconds`, and `permits_url` into `managed_install::domain` as `RetrievalPolicy`. **The body of `permits_url` does not change** — a behavior-preserving move means the diff shows relocation, not edits
- [x] 3.2 Move `CliInstallerIntegrity` as `ArtifactIntegrity`
- [x] 3.3 Leave `CliInstallerTemplate`, `CliInstallerRuntime`, `CliInstallerVersionArgument`, `CliPlatform`, and `template_for` in `tooling/cli`. Their no-fallback test stays with them
  - `CliInstallerTrust` keeps `templates` and now nests a `RetrievalPolicy` where its three bound fields were, so a declaration site reads `policy: RetrievalPolicy { .. }, templates: &[..]`. Six literals across four files were rewritten mechanically.
- [x] 3.4 Move the `permits_url` tests verbatim. If a moved test needs editing to pass, the move was not behavior-preserving — find out why before editing it
  - Moved unchanged. What stayed behind is **not** a second copy of the URL matrix: `trust.rs` now asserts only that `permits_url` still reaches the shared decision. Two copies of a security matrix drift, and the one that drifts is the one nobody is looking at.
- [x] 3.5 Add a catalog-walking test that every CLI vendor source declares a non-empty allowlist and a non-zero ceiling, satisfying the "refused at declaration" requirement without making startup fallible over a constant

## 4. Move the retrieval

- [x] 4.1 Move `vendor_downloader.rs` into `managed_install::infrastructure` as the artifact retriever, generalising `CliInstallerDownloader` to a `ManagedArtifactRetriever` port and `DownloadedInstaller` to `RetrievedArtifact`
- [x] 4.2 Move `vendor_downloader_tests.rs` with it, unchanged except for names
- [x] 4.3 Rewire `vendor_source.rs` to the shared port, converting the error at the boundary
  - **One deliberate behavior difference.** The old downloader verified against *whichever* template in the vendor's list declared a digest; the shared retriever verifies against the digest of the template actually selected. Every shipped template is `Unverified` today, so the two agree on current data — but the old rule would have verified a Windows download against a macOS digest the first time a vendor published one.
  - The installer file name also moved from being inferred from the response's content type to being chosen by the selected template's runtime. Same two values, decided from the thing that already knows which interpreter will run it rather than from a header the server controls.
- [x] 4.4 Move the concrete construction in `bootstrap/` to the shared type
- [x] 4.5 Delete the originals. A move that leaves the old file behind is a copy, and the subtree budget will say so
- [x] 4.6 **Acceptance for the move, before any new behavior:** the suites from task 1.1 pass with the same counts. Do not start group 5 until this holds
  - **Held, and every delta is accounted for.** `cli_environment` **13 -> 13**. `tooling::cli` **725 -> 712**: 12 downloader tests left with the code they cover, and the two URL-matrix tests merged into the one delegation assertion. `managed_install` **0 -> 17**: those 12, the 2 moved URL tests, and 3 new ones for `is_bounded`, platform selection, and the digest carrier.
  - No existing assertion was edited to pass. The one behavior difference is deliberate and recorded at 4.3.

## 5. The archive kind

- [x] 5.1 Add archive retrieval: download and verify through the shared path, then extract into an owned directory
  - Split the same way the download is: `ExtractionGuard` owns containment and the limits, a format adapter feeds it entries. A second archive format is a second adapter, not a second set of bounds.
  - **Zip, not tar.gz.** Zip needs no dependency this build does not already carry. `jdtls` publishes `tar.gz`, so the next change adds a `tar` adapter *and* makes the dependency decision that comes with it — which is a supply-chain call that belongs with its consumer, not smuggled in here under an unused affordance.
- [x] 5.2 Enforce path containment on each entry's **resolved** path, not on its name. `a/../../b` passes a leading-slash check
  - `..` is refused rather than popped. Popping would let an archive walk to the destination's parent and back into a sibling it was never given, which is a containment check that permits exactly what it exists to stop.
- [x] 5.3 Enforce a total-bytes ceiling and an entry-count ceiling while extracting, removing the destination if either trips
- [x] 5.4 Discard the downloaded archive after successful extraction
- [x] 5.5 Add tests for: an entry escaping the destination, an absolute-path entry, a parent-component entry, exceeding the byte ceiling, exceeding the entry count, and a clean extraction reporting its directory
- [x] 5.6 Confirm the archive kind has no production caller in this change and is not wired into any command surface
  - Confirmed. Every archive symbol is `expect(dead_code)` under `cfg(not(test))` — `expect`, not `allow`, so the change that adds a caller has to remove the attribute rather than inherit a permanently silenced one.

## 6. Platform selection

- [x] 6.1 Add the shared capability's own platform-selection type and exact-match rule, independent of `CliPlatform`
  - `ManagedPlatform` and `PlatformArtifact` are the capability's own. Sharing `CliPlatform` would make the shared type describe the first consumer's vocabulary, which is the drift the extraction exists to prevent, arriving from the other direction.
- [x] 6.2 Add a test that no artifact is selected when none is declared for the current platform, and that nothing is substituted

## 7. Documentation

- [x] 7.1 Update the developer guide's CLI environment section to say where the download path now lives and why only that half moved
  - Added to both language editions of `cli-lifecycle.md`, including the reason the rest did not move (a 24,000-line behavior-preserving refactor reaching a persisted primary key, for no user-visible gain) and the resolved-path containment rule.
- [x] 7.2 Run `npm run docs:check`

## 8. Verification

- [x] 8.1 `npm run lint:ci`
- [x] 8.2 `npm run test` — **1,666 passed**
- [x] 8.3 `npm run build`
- [x] 8.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 8.5 `cargo check --workspace`
- [x] 8.6 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 8.7 `npm run native:panic:check`
- [x] 8.8 `cargo test --workspace` — 4,594 tests. Three full runs: **4,593/1**, then **4,592/2** twice.
  - The failures are `initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` and `relay_stdio::child_exit_does_not_wait_for_open_parent_input`, both documented load-sensitive tests, both passing in isolation, and both in subsystems this change does not touch — `code_intelligence` and `tooling/mcp`, against a change confined to `tooling/cli` and the new `tooling/managed_install`.
  - Stated plainly rather than rounded up: **no run of this change's suite was fully green.** The evidence that these are not regressions is the combination of isolation passes, the documented history, and the untouched subsystems — not a clean run, because there was not one.
- [x] 8.9 `npm run architecture:check`, `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test` — all pass, **no budget needed raising**
- [x] 8.10 `npx playwright test` — **184 passed**
- [x] 8.11 `npm run desktop:unit:test`, then `npm run test:desktop`. The `desktop-cli-management` layer is the one that exercises this path end to end, against a temporary PATH with fake package managers — it is the closest thing to evidence that the move did not break a real install
  - **`desktop-cli-management`: PASSED.** That is the evidence this task was written to collect, and it is the layer that would notice a broken catalog, a mangled trust literal, or a download path that no longer reaches its adapter.
  - Six of seven layers passed. **`desktop-settings-persistence` FAILED in the full run** and passed when re-run alone against the identical binary — both of its specs, in 4.3s and 6.6s.
  - Not written off as flake without a reason: the native log for the failing run shows real `npm.cmd view` and `winget show` catalog calls blocking for about ten seconds inside that layer's window. That layer does not stub them the way `desktop-cli-management` does, and this change touches neither the npm nor the WinGet source. The same layer passed in the full run of the previous change on this machine.
  - **Windows only.** macOS and Linux are NOT RUN here; CI's `Desktop Smoke` is the only evidence for those.
- [x] 8.12 `openspec validate extract-managed-tool-installation --strict` and `openspec validate --specs --strict` — valid; 138/138
- [x] 8.13 Simulate the archive merge with `buildUpdatedSpec` — `managed-tool-installation +7`, `cli-environment-management ~1`, both merging cleanly

## 9. Acceptance

- [x] 9.1 Confirm the task 4.6 checkpoint held: the move alone changed no test outcome
  - Held, and it was checked before group 5 began rather than reconstructed afterwards. The commit boundary is the same boundary, so a bisect lands on the move by itself.
- [x] 9.2 Confirm the move moved rather than copied — the `tooling` subtree total should fall or hold, not rise by the size of the moved code
  - **The measurement as written does not apply, because the shared subdomain landed inside `tooling` rather than outside it.** A move between two directories of the same subtree is net zero there by construction, so the +712 says nothing about duplication either way. Recorded rather than reported as a pass.
  - The check that does apply: `grep "fn permits_url"` finds **one** implementation, in `managed_install`; the CLI side has a one-line delegation. `strip_prefix("https://")` appears once in the whole tree. Both original files are deleted, not emptied.
  - The +712 is new code, and it is accounted for: the archive extractor and its seven tests, the error type, platform selection and its two tests, and the module and api files a subdomain needs.
- [x] 9.3 Confirm no schema change and no migration were added
  - `git diff -- src-tauri/src/platform/database/` is empty.
- [x] 9.4 Confirm no command, DTO, or frontend file changed
  - `git diff -- src/ src-tauri/src/commands/` is empty. The change is invisible from outside the native process, which is what the proposal claimed.
- [x] 9.5 Confirm the shared error type carries no CLI-specific variant, and the shared domain names no CLI type
  - `ManagedInstallError` has five variants — `Refused`, `Transfer`, `TimedOut`, `Cancelled`, `ChecksumMismatch` — and none of them names a CLI concept. The shared domain names no `Cli*` type; `ManagedPlatform` exists precisely so `CliPlatform` did not have to cross.
  - The conversion lives on the CLI side, in `vendor_source.rs`, so the shared capability never learns how a refusal should read to a CLI caller.
