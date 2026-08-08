## Context

The publish job downloads every platform artifact into `release-assets/`, generates an SPDX SBOM, then pipes `find` output into `xargs -0 sha256sum`. `sha256sum` echoes back the path it was handed, so the manifest recorded runner paths: `release-assets/packages/nsis/VaneHub AI_0.1.0-preview.1_x64-setup.exe`.

GitHub serves release assets under the basename with spaces replaced by dots. Confirmed against the published release: every asset carrying a space in its Tauri-generated name is served with a dot, and no other character was substituted.

The manifest and the served names therefore had nothing in common, which was only visible by downloading the published release and running the command the notes prescribe.

## Goals / Non-Goals

**Goals:**

- The documented command verifies a downloaded asset with no renaming or path reconstruction.
- A future regression is caught in the run, not by a downloader.

**Non-Goals:**

- Recomputing digests from downloaded assets. Checksums must attest to what the build produced, not to what the host served back; recomputing after download would certify the wrong thing.
- Renaming the assets. The names come from Tauri's `productName`, and changing them would churn every download link for a cosmetic gain.
- Re-tagging `v0.1.0-preview.1`. No package changed.

## Decisions

### Record the served name, not the build path

The loop emits `basename` with spaces mapped to dots. That is the transformation GitHub applies, verified against the published assets rather than assumed from documentation.

The alternative — telling downloaders to reconstruct the build path, or to pass `--ignore-missing` and accept a silent no-op — pushes a workflow defect onto every user, which is the situation being fixed.

### Fail on a name collision instead of publishing an ambiguous manifest

Two assets whose names differ only by a space and a dot collapse to one name. The manifest would then carry two entries with the same name: a checksum tool verifies the file against the first matching line and the other package silently goes unverified.

The job now fails before creating the release. A release that cannot be verified unambiguously is worse than a release that did not happen, because the failure is invisible from the download page. This is currently unreachable — Tauri names each bundle by architecture and format — which is exactly why it needs a guard rather than an assumption.

### Echo the manifest into the job log

The original defect was invisible from the run: every step reported success. Printing the manifest makes the names reviewable at the moment they are produced, which is where a maintainer can still act.

### Say that a mismatch usually means a truncated download

Two `gh release download` attempts against this release returned 9.6 MB and 9.4 MB for a 12.0 MB asset — two different truncations, no error. A downloader who hits that and follows the verification step sees a checksum mismatch, whose obvious reading is tampering. The notes now name the likelier cause and tell them to check the size and retry.

## Risks / Trade-offs

- **The transformation is inferred from observed behaviour, not a published contract.** → It matched every asset in this release, and the collision guard fails loudly if a future name maps unexpectedly. Directly querying the release for its asset names would be exact, but the manifest is generated before the release exists.
- **The manifest no longer distinguishes assets by directory.** → It never usefully did; the directories are runner-local and absent from the download page.
- **The already-published manifest was replaced rather than re-tagged.** → The digests are unchanged, so the replacement asserts nothing new about the packages. The alternative — a `preview.2` whose only difference is a text file — would cost every downloader a re-download for nothing.

## Open Questions

None.
