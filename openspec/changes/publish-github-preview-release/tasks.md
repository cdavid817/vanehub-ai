## 1. Restore the rehearsal path

Land this group on its own. Every later verification step depends on being able to run the workflow without a tag.

- [x] 1.1 Change `scripts/check-version-sync.mjs` so the `GITHUB_REF_NAME` fallback applies only when `GITHUB_REF_TYPE` is `tag`, leaving an explicit CLI argument authoritative
- [x] 1.2 Add a unit test covering three cases: explicit tag argument, `GITHUB_REF_TYPE=branch` with `GITHUB_REF_NAME=main` (must pass), and `GITHUB_REF_TYPE=tag` with a mismatched tag (must fail)
- [x] 1.3 Simplify the `validate` job's argument in `.github/workflows/package.yml` now that the script no longer depends on the conditional expression for correctness
- [x] 1.4 Verify locally: `npm run version:check`, then `GITHUB_REF_TYPE=branch GITHUB_REF_NAME=main npm run version:check`
- [ ] 1.5 Merge to `main`, then run `workflow_dispatch` on `main` and confirm `validate` passes and all three matrix jobs upload artifacts

## 2. Adopt the pre-release version and declared bundle targets

- [x] 2.1 Set `version` to `0.1.0-preview.1` in `package.json`
- [x] 2.2 Set `[package] version` to `0.1.0-preview.1` in `src-tauri/Cargo.toml` and refresh `src-tauri/Cargo.lock`
- [x] 2.3 Set `version` to `0.1.0-preview.1` in `src-tauri/tauri.conf.json`
- [x] 2.4 Replace `"targets": "all"` with `["nsis", "app", "dmg", "deb", "appimage"]` in `src-tauri/tauri.conf.json`
- [x] 2.5 Verify `npm run version:check` passes and `npm run version:check -- v0.1.0-preview.1` passes

## 3. Accept pre-release version facts in README parity

Tasks 3 and 4 must land in the same commit as task 2. `npm run docs:check` runs in CI and couples them.

- [x] 3.1 Extend the `project-version` badge pattern in `scripts/check-readme-parity.mjs` to capture a shields.io-escaped pre-release segment, and un-escape `--` to `-` before comparing against the `docs-fact` marker
- [x] 3.2 Add cases to `scripts/check-readme-parity.node-test.mjs` for a matching pre-release version, a stale pre-release identifier, and a stable version (no regression)
- [x] 3.3 Verify `npm run docs:unit:test` passes

## 4. Update the multilingual README set

Every edit in this group must be applied to `README.md`, `README.zh-CN.md`, and `README.ja.md` together; `sections`, `commands`, and `links` arrays must stay identical across all three.

- [x] 4.1 Update the `docs-fact:project-version` marker to `0.1.0-preview.1` in all three READMEs
- [x] 4.2 Update the version badge URL to `badge/version-0.1.0--preview.1-blue.svg` in all three READMEs
- [x] 4.3 Add a download section routing readers to the Releases page, stating that the current published build is an unsigned preview, in all three READMEs
- [x] 4.4 Verify `npm run docs:check` and `npm run docs:links:check` pass

## 5. Author the preview release notes

- [x] 5.1 Create `.github/PREVIEW_RELEASE_NOTES.md` stating the build is a preview and that packages are unsigned and un-notarized
- [x] 5.2 Add the feature boundary section, reusing the delivered / preview / planned classification already maintained in `README.md`
- [x] 5.3 Add macOS guidance: the quarantine prompt reporting a damaged application, and `xattr -cr "/Applications/VaneHub AI.app"`
- [x] 5.4 Add Windows guidance: the SmartScreen prompt and the "More info" then "Run anyway" path for the NSIS installer
- [x] 5.5 Add Linux guidance covering the `.deb` and AppImage assets, and state that no `.rpm` or `.msi` is published and which asset serves those users instead
- [x] 5.6 Add `SHA256SUMS` verification steps and state that checksums, SBOM, and attestations establish integrity but do not replace code signing
- [x] 5.7 Add the feedback route to the existing issue templates
- [x] 5.8 Keep the file version-independent: refer to assets by format, never by filename

## 6. Publish pre-releases correctly

- [x] 6.1 In the `publish` job of `.github/workflows/package.yml`, derive pre-release status from whether the tag name contains a hyphen
- [x] 6.2 Pass `--prerelease` and `--latest=false` when the tag carries a pre-release identifier, and neither when it does not
- [x] 6.3 Pass `.github/PREVIEW_RELEASE_NOTES.md` via `--notes-file` alongside `--generate-notes`
- [x] 6.5 Add a `macos-x64` matrix entry on the `macos-15-intel` runner so Intel Macs have a download, and list it in the release notes download table
- [x] 6.4 Confirm generated notes are appended rather than replaced. `gh` reads `--notes-file` into `opts.Body` and formats `"%s\n%s"` with the generated notes second, so the preview guidance lands above the generated summary and no concatenation fallback is needed

## 7. Update release documentation

- [x] 7.1 Add the preview release sequence to `docs/release-signing.md`, including the untagged rehearsal step
- [x] 7.2 Document the declared bundle target set in `docs/release-signing.md` or `docs/build-performance.md`, naming MSI and RPM as excluded and stating why
- [x] 7.3 State in the same document that unsigned preview packages require the published installation guidance

## 8. Verification

- [x] 8.1 Run `npm run lint:ci`
- [x] 8.2 Run `npm run test`
- [x] 8.3 Run `npm run build`
- [x] 8.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 8.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 8.6 Run `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 8.7 Run `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 8.8 Run `npm run docs:check` and `npm run coverage:policy:test`
- [x] 8.9 Run `openspec validate publish-github-preview-release --strict` and `openspec validate --specs --strict`
- [x] 8.10 Run `npm run package:windows:x64` locally and confirm the NSIS installer is produced and no MSI step is attempted
- [ ] 8.11 Install the local NSIS build, launch it, create a session, and run one Agent end to end

## 9. Rehearse and release

- [x] 9.0a Forward Apple and Tauri signing credentials to the build only when non-empty. An unset secret expands to a blank string, and the bundler read that as a request to sign, failing both macOS jobs at `security import` in the first rehearsal
- [x] 9.0b Collect artifacts by distributable format instead of `bundle/**`, which swept in the AppImage `.AppDir` tree and deb staging directory — 270 files in the Linux artifact where 2 are distributables
- [x] 9.1 Run `workflow_dispatch` against the change branch before merging, and confirm every matrix job uploads artifacts
- [ ] 9.2 Download the Windows, macOS, and Linux artifacts and install each on a real machine, following the published guidance verbatim
- [ ] 9.3 Confirm the macOS `xattr` step actually resolves the quarantine prompt on a clean machine
- [ ] 9.4 Create and push the annotated tag `v0.1.0-preview.1`
- [ ] 9.5 Confirm the published release is marked as a pre-release, is not marked latest, and carries the preview notes above the generated notes
- [ ] 9.6 Confirm `SHA256SUMS`, the SPDX SBOM, and the attestations are attached, and run `gh attestation verify` against one asset

## 10. Archive

- [ ] 10.1 Replace the `TBD` Purpose in `openspec/specs/desktop-release-delivery/spec.md` with the capability's actual purpose
- [ ] 10.2 Replace the `TBD` Purpose in `openspec/specs/multilingual-readme/spec.md` with the capability's actual purpose
- [ ] 10.3 Run `openspec archive publish-github-preview-release`, then `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`, and commit the main specs, archive directory, and index together
