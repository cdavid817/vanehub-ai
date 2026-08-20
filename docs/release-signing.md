# Desktop release signing

The `Package Desktop Apps` workflow uses an unprivileged `build-preview` environment for manual branch runs and a protected `release` environment for tag builds. Manual runs build review artifacts without signing secrets; tags matching `v<package-version>` additionally publish one GitHub Release after every platform build succeeds.

## Release sequence

1. Synchronize the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Review the channel-specific release notes and run `npm run version:check` plus the full validation suite.
3. Push the release branch and rehearse the complete build matrix with a manual workflow run.
4. Record Windows x64, macOS x64, macOS ARM64, Linux x64, and Linux ARM64 separately as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`.
5. Confirm that every required secret name is present on the protected `release` environment.
6. Merge the reviewed version change to `main`.
7. Create and push an annotated `v<version>` tag from that exact `main` commit.
8. Approve the `release` environment deployment if environment reviewers are configured.
9. Verify packages, updater signatures, `SHA256SUMS`, the SPDX SBOM, GitHub artifact attestations, release notes, and channel metadata. Operating-system signing is a later phase until its credentials are provisioned.

The publish job cannot run until all Windows, macOS, and Linux jobs finish successfully. It uses short-lived GitHub OIDC identity for attestations and does not require a stored GitHub token.

## Rehearsing without a tag

Start `Package Desktop Apps` manually from GitHub Actions against a branch. The run validates that the three version declarations agree, builds every matrix target, and uploads artifacts, but publishes nothing — the publish job is gated on a tag.

Version validation resolves the release tag from an explicit argument first, and falls back to `GITHUB_REF_NAME` only when `GITHUB_REF_TYPE` is `tag`. A branch reference is therefore never compared against a version. Reproduce either path locally:

```bash
GITHUB_REF_TYPE=branch GITHUB_REF_NAME=main npm run version:check
npm run version:check -- v<version>
```

Use a rehearsal to confirm that installers actually install before spending a tag on it. A failed publish leaves a pushed tag with no release; do not move that tag to different source. Correct the problem and publish a higher version when source or artifacts must change.

The rehearsal exports its ephemeral key path through both `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PATH`. Current Tauri bundling reads the former, while signer tooling documents the latter; both point to the same runner-temporary file and the key is never persisted as an artifact.

## Stable releases

A stable version has no semantic-version prerelease identifier. `.github/STABLE_RELEASE_NOTES.md` is prepended to generated change notes and is reviewed before the tag is created. It describes the supported package matrix, verification evidence, update channel, support routes, and known limitations without reusing unsigned-preview bypass instructions.

After the reviewed release commit is merged to `main`, verify its identity and create an annotated tag:

```bash
git switch main
git pull --ff-only origin main
npm run version:check -- v1.0.0
git tag -a v1.0.0 -m "Release VaneHub AI v1.0.0"
git push origin v1.0.0
```

Do not run these commands until the branch rehearsal passes and the protected credential inventory is complete. Never reuse or move a published version tag.

## Preview releases

Preview and release-candidate builds use a semantic-versioning pre-release identifier — `0.1.0-preview.1`, `0.1.0-rc.1` — declared identically in all three manifests and in the tag. `npm run version:check` compares the tag verbatim, so a pre-release identifier is never normalized away.

The publish job derives pre-release status from the tag alone: a tag containing a hyphen is published with `--prerelease` and `--latest=false`, so a preview never displaces a stable download. No separate workflow input controls this.

For a pre-release, `.github/PREVIEW_RELEASE_NOTES.md` is prepended to the generated notes. That file is where the unsigned-package disclosure and the per-platform installation steps live, so it is reviewed like any other repository file rather than authored inside a workflow run. Keep it version-independent: refer to assets by format, never by filename.

## Distributable formats

`bundle.targets` in `src-tauri/tauri.conf.json` declares an explicit list rather than `"all"`:

| Platform | Produced | Not produced |
| --- | --- | --- |
| Windows | NSIS `.exe` installer | `.msi` |
| macOS | `.app`, `.dmg` | — |
| Linux | `.deb`, AppImage | `.rpm` |

Both formats were originally excluded because of the preview version scheme. `tauri-bundler` aborts an MSI build when the pre-release identifier is not numeric, because the Windows Installer `ProductVersion` field accepts only `major.minor.patch[.build]`. The RPM `Version` field cannot contain a hyphen at all, and the bundler passes the version through unsanitized. The first stable release retains the already-rehearsed format set: users who would take an `.msi` are served by the NSIS installer, and users who would take an `.rpm` are served by the AppImage.

Adding either format remains a separate release change with its own packaging, signing, installation, and updater-manifest tests.

## Native build profile

Desktop packages use the shared Cargo release profile declared in `src-tauri/Cargo.toml`: optimization level 3, ThinLTO, one codegen unit, and debuginfo stripping. ThinLTO and a single codegen unit can extend release link time while enabling whole-program optimization and changing distributable size; they do not guarantee a smaller package on every target.

Windows x64 builds use the Rust-toolchain-provided LLD linker. Linux x64 and ARM64 builds require Clang and mold; AppImage bundling additionally requires `xdg-utils`, which is installed explicitly because it is not preinstalled on every runner image. The package workflow verifies the linkers before compilation. Linux ARM64 runs natively on GitHub's `ubuntu-24.04-arm` hosted runner, whose label is currently in public preview. Other targets retain their platform-default linker unless a target-specific policy is added and validated.

Debuginfo stripping does not remove VaneHub's operational `debug` log level. Release builds continue to persist redacted `error`, `warn`, `info`, and `debug` events through unified logging. Build prerequisites, verification commands, worktree cache behavior, and measurement evidence are documented in `docs/build-performance.md`.

The current measurement record contains optimized Windows executable, MSI, and NSIS sizes but no comparable pre-change package artifacts because the baseline package was interrupted by an external Rust toolchain update. Those absolute sizes do not establish a measured size reduction.

## GitHub environment secrets

Store credentials only as secrets on the `release` environment. Never place their values in repository variables, workflow files, issues, logs, or artifacts.

| Secret | Purpose |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64-encoded Apple Developer ID certificate bundle |
| `APPLE_CERTIFICATE_PASSWORD` | Certificate bundle password |
| `APPLE_SIGNING_IDENTITY` | Apple signing identity used by Tauri |
| `APPLE_ID` | Apple account used for notarization |
| `APPLE_PASSWORD` | App-specific Apple password |
| `APPLE_TEAM_ID` | Apple developer team identifier |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater artifact signing key, when updater publishing is enabled |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the Tauri updater signing key |
| `WINDOWS_CERTIFICATE` | Base64-encoded Windows release certificate bundle |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the Windows certificate bundle |
| `WINDOWS_SIGNER_SUBJECT` | Expected Authenticode publisher subject checked after signing |

Inventory names without reading or printing secret values:

```bash
gh secret list --repo cdavid817/vanehub-ai --env release
```

The expected inventory is the eleven names in the table above. A missing name makes stable release readiness `BLOCKED`; add its value through GitHub's Environment settings or an interactive `gh secret set --env release <NAME>` invocation, never through a command that exposes the value in shell history.

## Verification evidence

Windows evidence follows `artifact -> Get-AuthenticodeSignature -> expected publisher subject -> timestamp certificate`. A `Valid` status alone is insufficient: the workflow rejects an unexpected publisher or missing timestamp.

When Apple credentials are provisioned, macOS evidence follows `build -> codesign --verify --deep --strict -> notarize -> staple -> stapler validate -> spctl --assess -> publish` for both x64 and arm64 matrix entries. During updater-only phase 1, macOS packages are explicitly unsigned and un-notarized.

Linux packages retain SHA-256, SPDX SBOM, and GitHub provenance/SBOM attestations. These prove integrity and provenance; they are not operating-system code signing.

Updater bundles are generated by Tauri with `createUpdaterArtifacts` and signed by `TAURI_SIGNING_PRIVATE_KEY`. The corresponding public key is embedded in `src-tauri/tauri.conf.json`; rotate both as one reviewed release change before using a newly generated private key. Clients reject altered metadata, signatures, or payloads. Stable and preview clients read separate fixed HTTPS channel releases.

The repository does not currently define a Windows Authenticode provider. Before claiming signed Windows binaries, choose a managed certificate or key-vault provider, add its authentication at the `release` environment boundary, and verify the signature in the workflow. Do not export a long-lived private key merely to make CI convenient.

After publication, inspect the `Package Desktop Apps` run and the versioned GitHub Release. Verify that all five matrix jobs passed, package names match their targets, Windows signature output contains the expected publisher and timestamp, both macOS jobs report notarization/stapling success, every package verifies against `SHA256SUMS`, attestations verify with `gh attestation verify`, and `update-stable/latest.json` names version `1.0.0`. Announce the release only after every check passes.

## Environment protection

The environment should allow deployment only from protected `v*` tags. Add a human reviewer when a second trusted maintainer is available. A required self-review is intentionally not configured because it would make a single-maintainer repository impossible to release.

During updater-only phase 1, stable publication requires the updater signing key but permits explicitly disclosed unsigned Windows and un-notarized macOS packages. The checksums, SBOM, and GitHub attestations establish integrity but do not replace operating-system code signing or Apple notarization. A later phase will restore platform-signing gates after the corresponding credentials and provider are configured.

A bare disclosure is not enough for a public download. An unsigned build must also publish the steps a downloader needs to get past the protection each platform raises: macOS reports an un-notarized application as damaged until its quarantine attribute is cleared, and Windows SmartScreen blocks an unsigned installer behind a secondary confirmation. `.github/PREVIEW_RELEASE_NOTES.md` carries both, and must be updated whenever the signing situation changes.
