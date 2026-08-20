## Context

See `proposal.md` for motivation. The repository currently declares `0.1.0-preview.1` in npm, Cargo, and Tauri metadata. A tag-triggered GitHub workflow packages Windows x64, macOS x64/arm64, and Linux x64, then generates checksums, an SPDX SBOM, attestations, updater metadata, and a GitHub Release. Preview notes are tracked, while stable releases currently rely only on generated notes. The protected `release` environment exists and accepts `v*` tags, but it currently exposes none of the credential names required by the fail-closed stable gate.

## Goals / Non-Goals

**Goals:**

- Produce a reviewable commit whose release metadata consistently identifies `1.0.0`.
- Give the first stable release a curated, tracked narrative without losing GitHub-generated change notes.
- Make the pre-tag readiness decision reproducible across local validation, native rehearsal, credential inspection, and post-publication verification.
- Preserve fail-closed updater, Windows signing, and Apple signing/notarization behavior.

**Non-Goals:**

- Store, generate, rotate, or transmit production private keys and certificates in Git.
- Create or push `v1.0.0`, publish a GitHub Release, or mutate stable channel metadata before review and successful rehearsal.
- Add Windows ARM64, Linux ARM64, MSI, or RPM to the supported release matrix.
- Change application runtime behavior, frontend service contracts, Tauri commands, or Web/mock adapters.

## Decisions

### Use `1.0.0` consistently, with no prerelease identifier

The three authoritative manifest versions and their lockfile projections will move directly from `0.1.0-preview.1` to `1.0.0`. This aligns the annotated tag with the existing exact-match validator and causes the release workflow to select the stable channel. Using `0.1.0` was rejected because the requested milestone is the first formal, compatibility-signaling release; creating another release candidate was rejected because it would remain a preview rather than satisfy the requested outcome.

### Keep the proven package matrix for the first stable publication

The first stable release will retain NSIS for Windows x64, DMG/app for macOS x64 and ARM64, and deb/AppImage for Linux x64. This limits the release decision to formats already exercised by the preview pipeline. Adding MSI/RPM or new architectures is deferred because it broadens packaging, signing, installation, and updater-manifest verification at the release boundary.

### Track stable notes separately from preview notes

`.github/STABLE_RELEASE_NOTES.md` will contain stable-specific product highlights, supported downloads, verification guidance, update behavior, support routes, and limitations. The workflow will pass it through `gh release create --notes-file` only for tags without a prerelease identifier; generated notes remain enabled and follow the curated text. Reusing preview notes was rejected because their unsigned-install bypass instructions conflict with the stable fail-closed signing policy.

### Separate repository preparation from protected credential provisioning

The branch will record which environment secret names are required and how to verify readiness, but secret values remain an out-of-band GitHub Environment responsibility. Local tests and manual branch rehearsals use no production credential. The stable tag is created only after the environment reports the full required name set.

### Treat the tag as the irreversible release boundary

The implementation stops at a reviewed, validated branch plus a successful manual `Package Desktop Apps` run. An annotated `v1.0.0` tag is created from the merged `main` commit only after those gates and credential readiness pass. This avoids consuming a release tag on a build that is already known to be blocked.

## Risks / Trade-offs

- [Production signing credentials are currently absent] → Report readiness as `BLOCKED`, document exact names and provisioning commands, and do not create the tag.
- [A branch rehearsal uses ephemeral updater signing and cannot prove production signing] → Treat it as package/matrix evidence only; require the protected credential inventory before tagging and verify signatures in the tag run.
- [Generated notes can be noisy or incomplete for a first release] → Prepend a reviewed stable narrative and retain generated notes as the detailed change inventory.
- [Promoting directly from preview to `1.0.0` raises compatibility expectations] → Keep scope to the already-tested package matrix and require the repository's full validation suite plus native platform evidence.
- [A stable version permits MSI/RPM but the release omits them] → State supported formats explicitly; consider additional formats in a later independent change with their own install/signing tests.

## Migration Plan

1. Update all version declarations and lockfile projections to `1.0.0` and verify exact synchronization.
2. Add stable notes, workflow selection logic, tests, and release-readiness documentation.
3. Run the complete local validation suite and record platform-specific local results.
4. Push the branch and run the non-publishing package workflow on all declared native targets.
5. Provision and inventory the protected release secrets outside the repository.
6. Merge the reviewed commit to `main`, create an annotated `v1.0.0` tag, and push it.
7. Verify the versioned release, signatures/notarization, checksums, SBOM, attestations, and `update-stable/latest.json` before announcing availability.

Rollback before tagging is a normal revert of the release-preparation commit. After a tag-triggered publication, do not silently retag different source; mark a defective release appropriately and prepare a higher patch version.
