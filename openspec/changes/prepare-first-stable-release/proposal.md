## Why

VaneHub AI has published one preview build but still declares a prerelease version and has no reviewed, stable-specific release narrative. The first public stable release needs a reproducible `v1.0.0` source state, an auditable release rehearsal, and publication that fails closed when protected signing prerequisites are unavailable.

## What Changes

- Promote the synchronized application version from `0.1.0-preview.1` to `1.0.0` in every release manifest and lockfile entry.
- Add tracked stable-release notes that describe the first stable release, supported packages, verification evidence, update behavior, support routes, and known limitations.
- Make stable GitHub Releases prepend the reviewed stable notes while retaining generated change notes.
- Add an explicit release-readiness procedure covering the full repository validation suite, an untagged cross-platform rehearsal, protected credential readiness, tag creation, and post-publication verification.
- Keep publication fail-closed: no stable GitHub Release or stable updater metadata may be published without updater signing, Windows Authenticode evidence, and macOS signing/notarization evidence.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-release-delivery`: Require reviewed stable-release notes and an auditable readiness gate before the first stable tag can publish artifacts or stable updater metadata.

## Impact

- Release metadata: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and Cargo lock metadata.
- GitHub automation and tracked release material: `.github/workflows/package.yml`, a stable release-notes file, and release documentation.
- GitHub repository configuration: the protected `release` environment must be provisioned with updater, Windows, and Apple signing secrets outside Git.
- Runtime scope: desktop release delivery and desktop updater metadata are affected; Web runtime behavior and the frontend/native service boundary are unchanged.
