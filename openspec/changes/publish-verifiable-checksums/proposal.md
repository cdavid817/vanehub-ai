## Why

`v0.1.0-preview.1` published a `SHA256SUMS` that verifies nothing.

`sha256sum` writes back whatever path it was given, and the publish job gave it runner-workspace paths. The manifest therefore reads:

```
b754c812…  release-assets/packages/nsis/VaneHub AI_0.1.0-preview.1_x64-setup.exe
```

while GitHub serves that asset as `VaneHub.AI_0.1.0-preview.1_x64-setup.exe` — basename only, spaces replaced by dots. The command the release notes tell downloaders to run matches no file, prints `no file was verified`, and exits non-zero:

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

The packages themselves are intact; the recorded digests are correct. What is broken is the only integrity check most downloaders will attempt, on an unsigned build where checksums and attestations are the entire integrity story.

## What Changes

- The published checksum manifest names each entry as GitHub serves it, so the documented verification command works against a downloaded asset.
- The publish job fails when two assets would collapse to the same name, rather than emitting a manifest where one entry silently shadows another.
- The generated manifest is echoed into the job log, so a future regression is visible in the run rather than only to a downloader.
- The release notes state that a mismatch on a large asset is more often a truncated download than a tampered file.

The already-published `v0.1.0-preview.1` manifest was corrected in place by re-uploading the asset with the same digests under the served names. No package changed, so no re-tag was required.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-release-delivery`: publishing checksums gains the requirement that the manifest is usable — entries must name assets as published, and the job must fail rather than emit an ambiguous manifest.

## Impact

**Runtime scope: neither.** Release automation and release documentation only. No application code, no runtime adapter, no Tauri command.

Affected files:

- `.github/workflows/package.yml` — the checksum generation step
- `.github/PREVIEW_RELEASE_NOTES.md` — verification guidance

Downstream: anyone who tried to verify `v0.1.0-preview.1` before the manifest was re-uploaded saw a failed check. The corrected manifest carries the same digests, so a re-download is unnecessary — only the manifest needs fetching again.
